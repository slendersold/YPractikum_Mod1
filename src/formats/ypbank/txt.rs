//! Реализация формата YPBankText:
//! - запись = блок "KEY: VALUE", блоки разделены пустой строкой
//! - строки-комментарии начинаются с '#', игнорируются
//! - поля могут идти в любом порядке, но должны присутствовать ровно один раз
//! - DESCRIPTION хранится в двойных кавычках, UTF-8, допускаются пробелы внутри
//!
//! Этот модуль предоставляет две стратегии:
//! - next_op: FnMut(&mut R) -> Result<Option<Operation>, DataError>
//! - write_op: FnMut(&mut W, &Operation) -> Result<(), DataError>

use std::io::{self, BufRead, Read, Write};

use crate::errors::{DataError, ParseErrorKind, Result};
use crate::formats::data::{Operation, Status, TransactionType};

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

/// Построитель операции из распарсенных полей
#[derive(Default)]
struct OpBuilder {
    tx_id: Option<u64>,
    tx_type: Option<TransactionType>,
    from_user_id: Option<u64>,
    to_user_id: Option<u64>,
    amount: Option<i64>,
    timestamp_ms: Option<u64>,
    status: Option<Status>,
    description: Option<String>,
}

impl OpBuilder {
    /// Проверка дубликата поля по Option-состоянию
    fn ensure_empty<T>(
        slot: &Option<T>,
        line_no: usize,
        field: &'static str,
        line: &str,
    ) -> Result<()> {
        if slot.is_some() {
            return Err(crate::parse_err!(
                line_no,
                Some(field),
                ParseErrorKind::DuplicateField { name: field },
                line
            ));
        }
        Ok(())
    }

    /// Сборка Operation с плейсхолдерами для отсутствующих полей.
    fn build(self, _line_no: usize) -> Result<Operation> {
        Ok(Operation::new(
            self.tx_id.unwrap_or(0),
            self.tx_type.unwrap_or(TransactionType::Deposit),
            self.from_user_id.unwrap_or(0),
            self.to_user_id.unwrap_or(0),
            self.amount.unwrap_or(0),
            self.timestamp_ms.unwrap_or(0),
            self.status.unwrap_or(Status::Success),
            self.description.or(Some(String::new())),
        ))
    }
}

///
/// Парсер YPBankText поверх `BufReader<&mut R>`.
///
/// Вся логика чтения состояния (между вызовами `next_op`) хранится в структуре.
///
pub struct YPBankText<'a, R: Read> {
    reader: io::BufReader<&'a mut R>,
    /// Номер последней прочитанной строки
    line_no: usize,
}

impl<'a, R: Read> YPBankText<'a, R> {
    /// Создаёт парсер поверх уже открытого источника `Read`.
    pub fn new(r: &'a mut R) -> Self {
        Self {
            reader: io::BufReader::new(r),
            line_no: 0,
        }
    }

    /// Читает одну физическую строку (без `\r\n`), возвращает None на EOF.
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

    /// Проверяет, что строка пустая или комментарий.
    fn is_ignorable_line(line: &str) -> bool {
        let t = line.trim();
        t.is_empty() || t.starts_with('#')
    }

    /// Парсит строку `KEY: VALUE` -> (KEY, VALUE).
    fn parse_kv<'s>(&self, line: &'s str) -> Result<(&'s str, &'s str)> {
        let (k, v) = line.split_once(':').ok_or_else(|| {
            crate::parse_err!(self.line_no, None, ParseErrorKind::ExpectedKeyValue, line)
        })?;

        Ok((k.trim(), v.trim()))
    }

    /// Парсит u64 с унифицированной ошибкой.
    fn parse_u64(&self, s: &str, field: &'static str, line: &str) -> Result<u64> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(0);
        }
        s.parse::<u64>().map_err(|_| {
            crate::parse_err!(
                self.line_no,
                Some(field),
                ParseErrorKind::BadNumber {
                    value: s.to_string(),
                    ty: "u64"
                },
                line
            )
        })
    }

    /// Парсит i64 >= 0 с унифицированной ошибкой.
    fn parse_i64_nonneg(&self, s: &str, field: &'static str, line: &str) -> Result<i64> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(0);
        }
        let v = s.parse::<i64>().map_err(|_| {
            crate::parse_err!(
                self.line_no,
                Some(field),
                ParseErrorKind::BadNumber {
                    value: s.to_string(),
                    ty: "i64"
                },
                line
            )
        })?;

        if v < 0 {
            return Err(crate::parse_err!(
                self.line_no,
                Some(field),
                ParseErrorKind::NegativeNotAllowed {
                    value: s.to_string()
                },
                line
            ));
        }
        Ok(v)
    }

    /// Парсит TX_TYPE.
    fn parse_tx_type(&self, s: &str, line: &str) -> Result<TransactionType> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(TransactionType::Deposit);
        }
        match s {
            "DEPOSIT" => Ok(TransactionType::Deposit),
            "TRANSFER" => Ok(TransactionType::Transfer),
            "WITHDRAWAL" => Ok(TransactionType::Withdrawal),
            _ => Err(crate::parse_err!(
                self.line_no,
                Some(K_TX_TYPE),
                ParseErrorKind::BadEnum {
                    value: s.to_string(),
                    expected: "DEPOSIT | TRANSFER | WITHDRAWAL"
                },
                line
            )),
        }
    }

    /// Парсит STATUS.
    fn parse_status(&self, s: &str, line: &str) -> Result<Status> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(Status::Success);
        }
        match s {
            "SUCCESS" => Ok(Status::Success),
            "FAILURE" => Ok(Status::Failure),
            "PENDING" => Ok(Status::Pending),
            _ => Err(crate::parse_err!(
                self.line_no,
                Some(K_STATUS),
                ParseErrorKind::BadEnum {
                    value: s.to_string(),
                    expected: "SUCCESS | FAILURE | PENDING"
                },
                line
            )),
        }
    }

    /// Парсит DESCRIPTION в двойных кавычках.
    fn parse_description(&self, s: &str, line: &str) -> Result<String> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(String::new());
        }
        if !s.starts_with('"') || !s.ends_with('"') || s.len() < 2 {
            return Err(crate::parse_err!(
                self.line_no,
                Some(K_DESCRIPTION),
                ParseErrorKind::BadQuotedString {
                    value: s.to_string()
                },
                line
            ));
        }
        Ok(s[1..s.len() - 1].to_string())
    }

    /// Валидирует дополнительные правила спецификации:
    /// - DEPOSIT => FROM_USER_ID == 0
    /// - WITHDRAWAL => TO_USER_ID == 0
    fn validate_semantics(&self, op: &Operation) -> Result<()> {
        match op.tx_type() {
            TransactionType::Deposit => {
                if op.from_user_id() != 0 {
                    return Err(DataError::from(crate::errors::ParseError::full(
                        self.line_no,
                        Some(K_FROM_USER_ID),
                        ParseErrorKind::SemanticRule {
                            msg: "Для DEPOSIT ожидается FROM_USER_ID = 0",
                        },
                        "",
                    )));
                }
            }
            TransactionType::Withdrawal => {
                if op.to_user_id() != 0 {
                    return Err(DataError::from(crate::errors::ParseError::full(
                        self.line_no,
                        Some(K_TO_USER_ID),
                        ParseErrorKind::SemanticRule {
                            msg: "Для WITHDRAWAL ожидается TO_USER_ID = 0",
                        },
                        "",
                    )));
                }
            }
            TransactionType::Transfer => {}
        }
        Ok(())
    }

    /// Читает следующий блок записи и возвращает Operation.
    pub fn next_op(&mut self) -> Result<Option<Operation>> {
        let mut b = OpBuilder::default();
        let mut any_field = false;

        loop {
            let Some(line) = self.read_line()? else {
                if !any_field {
                    return Ok(None);
                }
                let op = b.build(self.line_no)?;
                self.validate_semantics(&op)?;
                return Ok(Some(op));
            };

            if Self::is_ignorable_line(&line) {
                if line.trim().is_empty() && any_field {
                    let op = b.build(self.line_no)?;
                    self.validate_semantics(&op)?;
                    return Ok(Some(op));
                }
                continue;
            }

            let raw_line = line.as_str();
            let (k, v) = self.parse_kv(raw_line)?;

            match k {
                K_TX_ID => {
                    OpBuilder::ensure_empty(&b.tx_id, self.line_no, K_TX_ID, raw_line)?;
                    b.tx_id = Some(self.parse_u64(v, K_TX_ID, raw_line)?);
                }
                K_TX_TYPE => {
                    OpBuilder::ensure_empty(&b.tx_type, self.line_no, K_TX_TYPE, raw_line)?;
                    b.tx_type = Some(self.parse_tx_type(v, raw_line)?);
                }
                K_FROM_USER_ID => {
                    OpBuilder::ensure_empty(
                        &b.from_user_id,
                        self.line_no,
                        K_FROM_USER_ID,
                        raw_line,
                    )?;
                    b.from_user_id = Some(self.parse_u64(v, K_FROM_USER_ID, raw_line)?);
                }
                K_TO_USER_ID => {
                    OpBuilder::ensure_empty(&b.to_user_id, self.line_no, K_TO_USER_ID, raw_line)?;
                    b.to_user_id = Some(self.parse_u64(v, K_TO_USER_ID, raw_line)?);
                }
                K_AMOUNT => {
                    OpBuilder::ensure_empty(&b.amount, self.line_no, K_AMOUNT, raw_line)?;
                    b.amount = Some(self.parse_i64_nonneg(v, K_AMOUNT, raw_line)?);
                }
                K_TIMESTAMP => {
                    OpBuilder::ensure_empty(&b.timestamp_ms, self.line_no, K_TIMESTAMP, raw_line)?;
                    b.timestamp_ms = Some(self.parse_u64(v, K_TIMESTAMP, raw_line)?);
                }
                K_STATUS => {
                    OpBuilder::ensure_empty(&b.status, self.line_no, K_STATUS, raw_line)?;
                    b.status = Some(self.parse_status(v, raw_line)?);
                }
                K_DESCRIPTION => {
                    OpBuilder::ensure_empty(&b.description, self.line_no, K_DESCRIPTION, raw_line)?;
                    b.description = Some(self.parse_description(v, raw_line)?);
                }
                other => {
                    return Err(crate::parse_err!(
                        self.line_no,
                        None,
                        ParseErrorKind::UnknownField {
                            name: other.to_string()
                        },
                        raw_line
                    ));
                }
            }

            any_field = true;
        }
    }

    /// Пишет одну операцию как блок YPBankText.
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

        /// Пишет строку KEY: VALUE с переводом строки.
        fn kv<W: Write>(w: &mut W, k: &str, v: impl AsRef<str>) -> Result<()> {
            writeln!(w, "{k}: {}", v.as_ref()).map_err(DataError::Io)?;
            Ok(())
        }

        kv(w, K_TX_ID, op.tx_id().to_string())?;
        kv(w, K_TX_TYPE, tx_type)?;
        kv(w, K_FROM_USER_ID, op.from_user_id().to_string())?;
        kv(w, K_TO_USER_ID, op.to_user_id().to_string())?;
        kv(w, K_AMOUNT, op.amount().to_string())?;
        kv(w, K_TIMESTAMP, op.timestamp_ms().to_string())?;
        kv(w, K_STATUS, status)?;

        let desc = op.description().unwrap_or("");
        // DESCRIPTION обязателен: если None, пишется пустая строка в кавычках.
        // Экранирование кавычек внутри не поддерживается (в спецификации не описано).
        let quoted = format!("\"{}\"", desc.replace('"', "'"));
        kv(w, K_DESCRIPTION, quoted)?;

        writeln!(w).map_err(DataError::Io)?;
        Ok(())
    }
}

///
/// Создаёт функцию-стратегию `next_op` для использования в `Statement::from_read`.
///
/// Стратегия хранит состояние парсера (BufReader, line_no) внутри замыкания.
///
pub fn make_next_op<'a, R: Read>(
    r: &'a mut R,
) -> impl FnMut(&mut R) -> Result<Option<Operation>> + 'a {
    let mut parser = YPBankText::new(r);
    move |_rr: &mut R| parser.next_op()
}

///
/// Создаёт функцию-стратегию `write_op` для использования в `Statement::write_to`.
///
/// Стратегия не хранит состояния; пишется один блок на операцию.
///
pub fn make_write_op<W: Write>() -> impl FnMut(&mut W, &Operation) -> Result<()> {
    move |w: &mut W, op: &Operation| YPBankText::<io::Empty>::write_op(w, op)
}
