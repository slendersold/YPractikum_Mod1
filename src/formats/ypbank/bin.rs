//! Реализация бинарного формата YPBankBin:
//! - файл = поток записей
//! - каждая запись: [MAGIC][RECORD_SIZE][BODY...]
//! - MAGIC = b"YPBN" (0x59 0x50 0x42 0x4E)
//! - RECORD_SIZE: u32 big-endian, количество байт тела записи
//! - все целые числа big-endian
//!
//! Этот модуль предоставляет две стратегии:
//! - next_op: FnMut(&mut R) -> Result<Option<Operation>, DataError>
//! - write_op: FnMut(&mut W, &Operation) -> Result<(), DataError>

use std::io::{self, Read, Write};

use crate::errors::{DataError, ParseError, ParseErrorKind, Result};
use crate::formats::data::{Operation, Status, TransactionType};

use crate::formats::ypbank::fields::{
    K_DESCRIPTION,
    K_FROM_USER_ID,
    K_RECORD_SIZE,
    // K_MAGIC,
    // K_AMOUNT,
    // K_TIMESTAMP,
    K_STATUS,
    K_TO_USER_ID,
    // K_TX_ID,
    K_TX_TYPE,
};

const MAGIC: [u8; 4] = *b"YPBN";

fn err(line_no: usize, field: Option<&'static str>, kind: ParseErrorKind, line: &str) -> DataError {
    DataError::from(ParseError::full(line_no, field, kind, line))
}

///
/// Парсер YPBankBin поверх `BufReader<&'a mut R>`.
///
/// Поддерживает повторную синхронизацию по MAGIC:
/// если MAGIC не найден на ожидаемой границе, читает поток байт и ищет последовательность `YPBN`.
///
pub struct YPBankBin<'a, R: Read> {
    reader: io::BufReader<&'a mut R>,
    /// Псевдо-номер "позиции" (не строка), используется как счётчик блоков/чтений для диагностики.
    rec_no: usize,
}

impl<'a, R: Read> YPBankBin<'a, R> {
    /// Создаёт парсер поверх источника `Read`.
    pub fn new(r: &'a mut R) -> Self {
        Self {
            reader: io::BufReader::new(r),
            rec_no: 0,
        }
    }

    /// Читает ровно N байт или возвращает I/O ошибку.
    fn read_exact_n<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut buf = [0u8; N];
        self.reader.read_exact(&mut buf).map_err(DataError::Io)?;
        Ok(buf)
    }

    fn read_u32_be(&mut self) -> Result<u32> {
        let b = self.read_exact_n::<4>()?;
        Ok(u32::from_be_bytes(b))
    }

    // fn read_u64_be(&mut self) -> Result<u64> {
    //     let b = self.read_exact_n::<8>()?;
    //     Ok(u64::from_be_bytes(b))
    // }

    // fn read_i64_be(&mut self) -> Result<i64> {
    //     let b = self.read_exact_n::<8>()?;
    //     Ok(i64::from_be_bytes(b))
    // }

    // fn read_u8(&mut self) -> Result<u8> {
    //     let b = self.read_exact_n::<1>()?;
    //     Ok(b[0])
    // }

    /// Ищет MAGIC в потоке. Возвращает Ok(false) на чистом EOF.
    fn sync_to_magic(&mut self) -> Result<bool> {
        let mut w = [0u8; 4];

        // Пытаемся прочитать первые 4 байта. Если EOF — значит данных нет.
        let mut filled = 0usize;
        while filled < 4 {
            let n = self.reader.read(&mut w[filled..]).map_err(DataError::Io)?;
            if n == 0 {
                return Ok(false);
            }
            filled += n;
        }

        loop {
            if w == MAGIC {
                return Ok(true);
            }

            // Сдвиг окна на 1 байт влево + дочитывание 1 байта
            w[0] = w[1];
            w[1] = w[2];
            w[2] = w[3];

            let mut last = [0u8; 1];
            let n = self.reader.read(&mut last).map_err(DataError::Io)?;
            if n == 0 {
                return Ok(false);
            }
            w[3] = last[0];
        }
    }

    fn decode_tx_type(&self, b: u8) -> std::result::Result<TransactionType, ParseErrorKind> {
        match b {
            0 => Ok(TransactionType::Deposit),
            1 => Ok(TransactionType::Transfer),
            2 => Ok(TransactionType::Withdrawal),
            _ => Err(ParseErrorKind::BadEnum {
                value: b.to_string(),
                expected: "0=DEPOSIT | 1=TRANSFER | 2=WITHDRAWAL",
            }),
        }
    }

    fn decode_status(&self, b: u8) -> std::result::Result<Status, ParseErrorKind> {
        match b {
            0 => Ok(Status::Success),
            1 => Ok(Status::Failure),
            2 => Ok(Status::Pending),
            _ => Err(ParseErrorKind::BadEnum {
                value: b.to_string(),
                expected: "0=SUCCESS | 1=FAILURE | 2=PENDING",
            }),
        }
    }

    /// Проверка семантики:
    /// - DEPOSIT => FROM_USER_ID == 0
    /// - WITHDRAWAL => TO_USER_ID == 0
    fn validate_semantics(&self, op: &Operation) -> Result<()> {
        match op.tx_type() {
            TransactionType::Deposit => {
                if op.from_user_id() != 0 {
                    return Err(err(
                        self.rec_no,
                        Some(K_FROM_USER_ID),
                        ParseErrorKind::SemanticRule {
                            msg: "Для DEPOSIT ожидается FROM_USER_ID = 0",
                        },
                        "",
                    ));
                }
            }
            TransactionType::Withdrawal => {
                if op.to_user_id() != 0 {
                    return Err(err(
                        self.rec_no,
                        Some(K_TO_USER_ID),
                        ParseErrorKind::SemanticRule {
                            msg: "Для WITHDRAWAL ожидается TO_USER_ID = 0",
                        },
                        "",
                    ));
                }
            }
            TransactionType::Transfer => {}
        }
        Ok(())
    }

    /// Читает следующую запись и возвращает Operation.
    pub fn next_op(&mut self) -> Result<Option<Operation>> {
        // 1) синхронизация по MAGIC
        let has = self.sync_to_magic()?;
        if !has {
            return Ok(None);
        }
        self.rec_no += 1;

        // 2) record size
        let record_size = self.read_u32_be()?;
        let body_len = record_size as usize;

        // Минимальный размер тела без DESCRIPTION:
        // TX_ID(8) + TX_TYPE(1) + FROM(8) + TO(8) + AMOUNT(8) + TIMESTAMP(8) + STATUS(1) + DESC_LEN(4) = 46
        if body_len < 46 {
            return Err(err(
                self.rec_no,
                Some(K_RECORD_SIZE),
                ParseErrorKind::SemanticRule {
                    msg: "RECORD_SIZE меньше минимально допустимого размера тела записи",
                },
                &record_size.to_string(),
            ));
        }

        // 3) читаем тело
        let mut body = vec![0u8; body_len];
        self.reader.read_exact(&mut body).map_err(DataError::Io)?;

        // 4) парсим тело из среза
        let mut cur = io::Cursor::new(&body);

        let tx_id = {
            let mut b = [0u8; 8];
            cur.read_exact(&mut b).map_err(DataError::Io)?;
            u64::from_be_bytes(b)
        };

        let tx_type = {
            let mut b = [0u8; 1];
            cur.read_exact(&mut b).map_err(DataError::Io)?;
            self.decode_tx_type(b[0])
                .map_err(|k| err(self.rec_no, Some(K_TX_TYPE), k, ""))?
        };

        let from_user_id = {
            let mut b = [0u8; 8];
            cur.read_exact(&mut b).map_err(DataError::Io)?;
            u64::from_be_bytes(b)
        };

        let to_user_id = {
            let mut b = [0u8; 8];
            cur.read_exact(&mut b).map_err(DataError::Io)?;
            u64::from_be_bytes(b)
        };

        let amount = {
            let mut b = [0u8; 8];
            cur.read_exact(&mut b).map_err(DataError::Io)?;
            i64::from_be_bytes(b)
        };

        let timestamp_ms = {
            let mut b = [0u8; 8];
            cur.read_exact(&mut b).map_err(DataError::Io)?;
            u64::from_be_bytes(b)
        };

        let status = {
            let mut b = [0u8; 1];
            cur.read_exact(&mut b).map_err(DataError::Io)?;
            self.decode_status(b[0])
                .map_err(|k| err(self.rec_no, Some(K_STATUS), k, ""))?
        };

        let desc_len = {
            let mut b = [0u8; 4];
            cur.read_exact(&mut b).map_err(DataError::Io)?;
            u32::from_be_bytes(b) as usize
        };

        let pos = cur.position() as usize;
        let remaining = body_len.saturating_sub(pos);

        if remaining != desc_len {
            return Err(err(
                self.rec_no,
                Some(K_DESCRIPTION),
                ParseErrorKind::SemanticRule {
                    msg: "DESC_LEN не совпадает с количеством оставшихся байт тела записи",
                },
                "",
            ));
        }

        let description = if desc_len == 0 {
            None
        } else {
            let mut db = vec![0u8; desc_len];
            cur.read_exact(&mut db).map_err(DataError::Io)?;
            match String::from_utf8(db) {
                Ok(s) => Some(s),
                Err(e) => {
                    return Err(err(
                        self.rec_no,
                        Some(K_DESCRIPTION),
                        ParseErrorKind::SemanticRule {
                            msg: "DESCRIPTION не является корректным UTF-8",
                        },
                        &e.to_string(),
                    ));
                }
            }
        };

        let op = Operation::new(
            tx_id,
            tx_type,
            from_user_id,
            to_user_id,
            amount,
            timestamp_ms,
            status,
            description,
        );

        self.validate_semantics(&op)?;
        Ok(Some(op))
    }

    /// Пишет одну запись YPBankBin (MAGIC + RECORD_SIZE + BODY).
    pub fn write_op<W: Write>(w: &mut W, op: &Operation) -> Result<()> {
        // Кодирование enum
        let tx_type_b: u8 = match op.tx_type() {
            TransactionType::Deposit => 0,
            TransactionType::Transfer => 1,
            TransactionType::Withdrawal => 2,
        };
        let status_b: u8 = match op.status() {
            Status::Success => 0,
            Status::Failure => 1,
            Status::Pending => 2,
        };

        // DESCRIPTION
        let desc_bytes = match op.description() {
            None => Vec::new(),
            Some(s) => s.as_bytes().to_vec(),
        };

        let desc_len_u32: u32 = match u32::try_from(desc_bytes.len()) {
            Ok(v) => v,
            Err(_) => {
                return Err(err(
                    0,
                    Some(K_DESCRIPTION),
                    ParseErrorKind::SemanticRule {
                        msg: "DESCRIPTION слишком длинное для u32",
                    },
                    "",
                ));
            }
        };

        // BODY: фиксированная часть + desc
        let mut body: Vec<u8> = Vec::with_capacity(46 + desc_bytes.len());

        body.extend_from_slice(&op.tx_id().to_be_bytes());
        body.push(tx_type_b);
        body.extend_from_slice(&op.from_user_id().to_be_bytes());
        body.extend_from_slice(&op.to_user_id().to_be_bytes());
        body.extend_from_slice(&op.amount().to_be_bytes());
        body.extend_from_slice(&op.timestamp_ms().to_be_bytes());
        body.push(status_b);
        body.extend_from_slice(&desc_len_u32.to_be_bytes());
        body.extend_from_slice(&desc_bytes);

        let record_size_u32: u32 = match u32::try_from(body.len()) {
            Ok(v) => v,
            Err(_) => {
                return Err(err(
                    0,
                    Some(K_RECORD_SIZE),
                    ParseErrorKind::SemanticRule {
                        msg: "Тело записи слишком большое для u32",
                    },
                    "",
                ));
            }
        };

        // HEADER
        w.write_all(&MAGIC).map_err(DataError::Io)?;
        w.write_all(&record_size_u32.to_be_bytes())
            .map_err(DataError::Io)?;
        w.write_all(&body).map_err(DataError::Io)?;
        Ok(())
    }
}

///
/// Создаёт функцию-стратегию `next_op` для использования в `Statement::from_read`.
///
/// Стратегия хранит состояние парсера (BufReader, синхронизация, счётчик записей) внутри замыкания.
///
pub fn make_next_op<'a, R: Read>(
    r: &'a mut R,
) -> impl FnMut(&mut R) -> Result<Option<Operation>> + 'a {
    let mut parser = YPBankBin::new(r);
    move |_rr: &mut R| parser.next_op()
}

///
/// Создаёт функцию-стратегию `write_op` для использования в `Statement::write_to`.
///
/// Функция generic по `W`, чтобы избежать HRTB-ограничений в return type.
///
pub fn make_write_op<W: Write>() -> impl FnMut(&mut W, &Operation) -> Result<()> {
    move |w: &mut W, op: &Operation| YPBankBin::<io::Empty>::write_op(w, op)
}
