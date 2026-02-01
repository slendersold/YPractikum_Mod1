//! Реализация CSV-формата YPBank:
//! - UTF-8
//! - первая строка — строгий заголовок
//! - далее по одной транзакции на строку
//! - пустые строки игнорируются
//! - DESCRIPTION всегда в двойных кавычках и может содержать запятые
//!
//! Модуль предоставляет две стратегии:
//! - next_op: FnMut(&mut R) -> Result<Option<Operation>, DataError>
//! - write_op: FnMut(&mut W, &Operation) -> Result<(), DataError>

use std::io::{self, BufRead, Read, Write};

use crate::errors::{DataError, ParseError, ParseErrorKind, Result};
use crate::formats::data::{Operation, Status, TransactionType};

const HEADER: &str = "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION";

use crate::formats::ypbank::fields::{
    K_AMOUNT,
    K_DESCRIPTION,
    // K_RECORD_SIZE,
    // K_MAGIC,
    K_FROM_USER_ID,
    K_STATUS,
    K_TIMESTAMP,
    K_TO_USER_ID,
    K_TX_ID,
    K_TX_TYPE,
};

fn err(line_no: usize, field: Option<&'static str>, kind: ParseErrorKind, line: &str) -> DataError {
    DataError::from(ParseError::full(line_no, field, kind, line))
}

///
/// CSV-парсер YPBank поверх `BufReader<&mut R>`.
///
/// Хранит состояние:
/// - проверка заголовка (один раз)
/// - текущий номер строки
///
pub struct YPBankCsv<'a, R: Read> {
    reader: io::BufReader<&'a mut R>,
    line_no: usize,
    header_done: bool,
}

impl<'a, R: Read> YPBankCsv<'a, R> {
    /// Создаёт парсер поверх источника `Read`.
    pub fn new(r: &'a mut R) -> Self {
        Self {
            reader: io::BufReader::new(r),
            line_no: 0,
            header_done: false,
        }
    }

    fn read_line(&mut self) -> Result<Option<String>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).map_err(DataError::Io)?;
        if n == 0 {
            return Ok(None);
        }
        self.line_no += 1;
        while line.ends_with('\n') || line.ends_with('\r') {
            line.pop();
        }
        Ok(Some(line))
    }

    fn ensure_header(&mut self) -> Result<()> {
        if self.header_done {
            return Ok(());
        }

        loop {
            let Some(line) = self.read_line()? else {
                return Err(err(
                    0,
                    None,
                    ParseErrorKind::SemanticRule {
                        msg: "Пустой CSV: отсутствует заголовок",
                    },
                    "",
                ));
            };

            if line.trim().is_empty() {
                continue;
            }

            if line != HEADER {
                return Err(err(
                    self.line_no,
                    None,
                    ParseErrorKind::SemanticRule {
                        msg: "Неверный заголовок CSV (ожидался строго заданный HEADER)",
                    },
                    &line,
                ));
            }

            self.header_done = true;
            return Ok(());
        }
    }

    fn parse_u64(&self, s: &str, field: &'static str, line: &str) -> Result<u64> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(0);
        }
        s.parse::<u64>().map_err(|_| {
            err(
                self.line_no,
                Some(field),
                ParseErrorKind::BadNumber {
                    value: s.to_string(),
                    ty: "u64",
                },
                line,
            )
        })
    }

    fn parse_i64_nonneg(&self, s: &str, field: &'static str, line: &str) -> Result<i64> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(0);
        }
        let v = s.parse::<i64>().map_err(|_| {
            err(
                self.line_no,
                Some(field),
                ParseErrorKind::BadNumber {
                    value: s.to_string(),
                    ty: "i64",
                },
                line,
            )
        })?;

        if v < 0 {
            return Err(err(
                self.line_no,
                Some(field),
                ParseErrorKind::NegativeNotAllowed {
                    value: s.to_string(),
                },
                line,
            ));
        }
        Ok(v)
    }

    fn parse_tx_type(&self, s: &str, line: &str) -> Result<TransactionType> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(TransactionType::Deposit);
        }
        match s {
            "DEPOSIT" => Ok(TransactionType::Deposit),
            "TRANSFER" => Ok(TransactionType::Transfer),
            "WITHDRAWAL" => Ok(TransactionType::Withdrawal),
            _ => Err(err(
                self.line_no,
                Some(K_TX_TYPE),
                ParseErrorKind::BadEnum {
                    value: s.to_string(),
                    expected: "DEPOSIT | TRANSFER | WITHDRAWAL",
                },
                line,
            )),
        }
    }

    fn parse_status(&self, s: &str, line: &str) -> Result<Status> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(Status::Success);
        }
        match s {
            "SUCCESS" => Ok(Status::Success),
            "FAILURE" => Ok(Status::Failure),
            "PENDING" => Ok(Status::Pending),
            _ => Err(err(
                self.line_no,
                Some(K_STATUS),
                ParseErrorKind::BadEnum {
                    value: s.to_string(),
                    expected: "SUCCESS | FAILURE | PENDING",
                },
                line,
            )),
        }
    }

    /// DESCRIPTION: всегда в двойных кавычках. Запятые внутри разрешены.
    /// Экранирование кавычек внутри не поддерживается (в спецификации не описано).
    fn parse_description(&self, s: &str, line: &str) -> Result<String> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(String::new());
        }
        if !s.starts_with('"') || !s.ends_with('"') || s.len() < 2 {
            return Err(err(
                self.line_no,
                Some(K_DESCRIPTION),
                ParseErrorKind::BadQuotedString {
                    value: s.to_string(),
                },
                line,
            ));
        }
        Ok(s[1..s.len() - 1].to_string())
    }

    /// Разделение CSV-строки на 8 полей с учётом того, что DESCRIPTION может содержать запятые.
    ///
    /// Правило:
    /// - 7 первых полей — до 7-й запятой (фиксированно)
    /// - остаток строки — поле DESCRIPTION (последнее), включая кавычки
    fn split_8_fields<'b>(
        &self,
        line: &'b str,
    ) -> Result<(
        &'b str,
        &'b str,
        &'b str,
        &'b str,
        &'b str,
        &'b str,
        &'b str,
        &'b str,
    )> {
        // Находим позиции первых 7 запятых
        let mut idxs = [0usize; 7];
        let mut found = 0usize;

        for (i, b) in line.bytes().enumerate() {
            if b == b',' {
                if found < 7 {
                    idxs[found] = i;
                    found += 1;
                    if found == 7 {
                        break;
                    }
                }
            }
        }

        if found != 7 {
            return Err(err(
                self.line_no,
                None,
                ParseErrorKind::SemanticRule {
                    msg: "Неверное число полей: ожидалось минимум 7 запятых (8 полей)",
                },
                line,
            ));
        }

        // Срезы по индексам запятых
        let c0 = idxs[0];
        let c1 = idxs[1];
        let c2 = idxs[2];
        let c3 = idxs[3];
        let c4 = idxs[4];
        let c5 = idxs[5];
        let c6 = idxs[6];

        let f0 = &line[..c0];
        let f1 = &line[c0 + 1..c1];
        let f2 = &line[c1 + 1..c2];
        let f3 = &line[c2 + 1..c3];
        let f4 = &line[c3 + 1..c4];
        let f5 = &line[c4 + 1..c5];
        let f6 = &line[c5 + 1..c6];
        let f7 = &line[c6 + 1..];

        Ok((f0, f1, f2, f3, f4, f5, f6, f7))
    }

    /// Проверка семантики:
    /// - DEPOSIT => FROM_USER_ID == 0
    /// - WITHDRAWAL => TO_USER_ID == 0
    fn validate_semantics(&self, op: &Operation, line: &str) -> Result<()> {
        match op.tx_type() {
            TransactionType::Deposit => {
                if op.from_user_id() != 0 {
                    return Err(err(
                        self.line_no,
                        Some(K_FROM_USER_ID),
                        ParseErrorKind::SemanticRule {
                            msg: "Для DEPOSIT ожидается FROM_USER_ID = 0",
                        },
                        line,
                    ));
                }
            }
            TransactionType::Withdrawal => {
                if op.to_user_id() != 0 {
                    return Err(err(
                        self.line_no,
                        Some(K_TO_USER_ID),
                        ParseErrorKind::SemanticRule {
                            msg: "Для WITHDRAWAL ожидается TO_USER_ID = 0",
                        },
                        line,
                    ));
                }
            }
            TransactionType::Transfer => {}
        }
        Ok(())
    }

    /// Читает следующую операцию из CSV.
    pub fn next_op(&mut self) -> Result<Option<Operation>> {
        self.ensure_header()?;

        loop {
            let Some(line) = self.read_line()? else {
                return Ok(None);
            };

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let (f0, f1, f2, f3, f4, f5, f6, f7) = self.split_8_fields(trimmed)?;

            let tx_id = self.parse_u64(f0.trim(), K_TX_ID, trimmed)?;
            let tx_type = self.parse_tx_type(f1.trim(), trimmed)?;
            let from_user_id = self.parse_u64(f2.trim(), K_FROM_USER_ID, trimmed)?;
            let to_user_id = self.parse_u64(f3.trim(), K_TO_USER_ID, trimmed)?;
            let amount = self.parse_i64_nonneg(f4.trim(), K_AMOUNT, trimmed)?;
            let timestamp_ms = self.parse_u64(f5.trim(), K_TIMESTAMP, trimmed)?;
            let status = self.parse_status(f6.trim(), trimmed)?;
            let description = self.parse_description(f7.trim(), trimmed)?;

            let op = Operation::new(
                tx_id,
                tx_type,
                from_user_id,
                to_user_id,
                amount,
                timestamp_ms,
                status,
                Some(description),
            );

            self.validate_semantics(&op, trimmed)?;
            return Ok(Some(op));
        }
    }

    /// Пишет одну операцию как CSV строку (без заголовка).
    pub fn write_op<W: Write>(w: &mut W, op: &Operation) -> Result<()> {
        let tx_type = match op.tx_type() {
            TransactionType::Deposit => "DEPOSIT",
            TransactionType::Transfer => "TRANSFER",
            TransactionType::Withdrawal => "WITHDRAWAL",
        };
        let status = match op.status() {
            Status::Success => "SUCCESS",
            Status::Failure => "FAILURE",
            Status::Pending => "PENDING",
        };

        // DESCRIPTION всегда в кавычках. В спецификации не описано экранирование кавычек,
        // поэтому кавычки внутри заменяются на апостроф.
        let desc = op.description().unwrap_or("").replace('"', "'");

        writeln!(
            w,
            "{},{},{},{},{},{},{},\"{}\"",
            op.tx_id(),
            tx_type,
            op.from_user_id(),
            op.to_user_id(),
            op.amount(),
            op.timestamp_ms(),
            status,
            desc
        )
        .map_err(DataError::Io)?;

        Ok(())
    }

    /// Пишет заголовок CSV.
    pub fn write_header<W: Write>(w: &mut W) -> Result<()> {
        writeln!(w, "{HEADER}").map_err(DataError::Io)?;
        Ok(())
    }
}

///
/// Создаёт функцию-стратегию `next_op` для использования в `Statement::from_read`.
///
/// Стратегия хранит состояние парсера (BufReader, заголовок, line_no) внутри замыкания.
///
pub fn make_next_op<'a, R: Read>(
    r: &'a mut R,
) -> impl FnMut(&mut R) -> Result<Option<Operation>> + 'a {
    let mut parser = YPBankCsv::new(r);
    move |_rr: &mut R| parser.next_op()
}

///
/// Создаёт функцию-стратегию `write_op` для использования в `Statement::write_to`.
///
/// Функция generic по `W`, чтобы избежать HRTB-ограничений в return type.
///
pub fn make_write_op<W: Write>() -> impl FnMut(&mut W, &Operation) -> Result<()> {
    move |w: &mut W, op: &Operation| YPBankCsv::<io::Empty>::write_op(w, op)
}
