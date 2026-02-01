// src/bin/comparer.rs
//
// CLI-утилита для сравнения записей операций в двух файлах.
//
// Пример:
//   ypbank_compare --file1 a.bin --format1 bin --file2 b.csv --format2 csv
//
// Если записи совпадают — печатает "The transaction records are identical."
// Иначе — сообщает, что записи различаются.

use std::env;
use std::fs::File;
use std::io::{self, Read};

use utils::data::Statement;
use utils::errors::{DataError, Result};
use utils::ypbank::{bin as ybin, csv as ycsv, txt as ytxt};

/// Поддерживаемые форматы входных данных.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Txt,
    Csv,
    Bin,
}

/// Разбирает строковое имя формата.
fn parse_format(s: &str) -> Option<Format> {
    match s.to_ascii_lowercase().as_str() {
        "txt" | "text" => Some(Format::Txt),
        "csv" => Some(Format::Csv),
        "bin" | "binary" => Some(Format::Bin),
        _ => None,
    }
}

/// Текст справки по CLI.
fn usage() -> &'static str {
    r#"Usage:
  ypbank_compare --file1 <path> --format1 <txt|csv|bin> --file2 <path> --format2 <txt|csv|bin>

Options:
  --file1         First input file
  --format1       First input format:  txt | csv | bin
  --file2         Second input file
  --format2       Second input format: txt | csv | bin
  -h, --help      Show this help
"#
}

/// Точка входа CLI-сравнивателя.
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

/// Основной сценарий: парсинг аргументов, чтение, сравнение.
fn run() -> Result<()> {
    let mut file1: Option<String> = None;
    let mut file2: Option<String> = None;
    let mut format1: Option<Format> = None;
    let mut format2: Option<Format> = None;

    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{}", usage());
                return Ok(());
            }
            "--file1" => {
                file1 = Some(
                    args.next()
                        .ok_or_else(|| DataError::Format("Missing value for --file1".into()))?,
                );
            }
            "--file2" => {
                file2 = Some(
                    args.next()
                        .ok_or_else(|| DataError::Format("Missing value for --file2".into()))?,
                );
            }
            "--format1" => {
                let v = args
                    .next()
                    .ok_or_else(|| DataError::Format("Missing value for --format1".into()))?;
                format1 = Some(
                    parse_format(&v)
                        .ok_or_else(|| DataError::Format("Unknown --format1 format".into()))?,
                );
            }
            "--format2" => {
                let v = args
                    .next()
                    .ok_or_else(|| DataError::Format("Missing value for --format2".into()))?;
                format2 = Some(
                    parse_format(&v)
                        .ok_or_else(|| DataError::Format("Unknown --format2 format".into()))?,
                );
            }
            _ => return Err(DataError::Format(format!("Unknown argument: {a}"))),
        }
    }

    let file1 = file1.ok_or_else(|| DataError::Format("Missing --file1".into()))?;
    let file2 = file2.ok_or_else(|| DataError::Format("Missing --file2".into()))?;
    let format1 = format1.ok_or_else(|| DataError::Format("Missing --format1".into()))?;
    let format2 = format2.ok_or_else(|| DataError::Format("Missing --format2".into()))?;

    let mut input1: Box<dyn Read> = Box::new(File::open(file1).map_err(DataError::Io)?);
    let mut input2: Box<dyn Read> = Box::new(File::open(file2).map_err(DataError::Io)?);

    let mut statement1 = read_statement(format1, &mut input1)?;
    let mut statement2 = read_statement(format2, &mut input2)?;

    compare_statements(&mut statement1, &mut statement2)?;
    Ok(())
}

/// Читает выписку в `Statement` с учетом выбранного формата.
fn read_statement<R: Read>(fmt: Format, r: &mut R) -> Result<Statement> {
    let mut dummy = io::empty();

    match fmt {
        Format::Txt => {
            let mut parser = ytxt::YPBankText::new(r);
            Statement::from_read(&mut dummy, |_| parser.next_op())
        }
        Format::Csv => {
            let mut parser = ycsv::YPBankCsv::new(r);
            Statement::from_read(&mut dummy, |_| parser.next_op())
        }
        Format::Bin => {
            let mut parser = ybin::YPBankBin::new(r);
            Statement::from_read(&mut dummy, |_| parser.next_op())
        }
    }
}

/// Сравнивает две выписки и выводит результат.
fn compare_statements(a: &mut Statement, b: &mut Statement) -> Result<()> {
    if a == b {
        println!("The transaction records are identical.");
    } else {
        println!("The transaction records are different.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op_new(
        tx_id: u64,
        tx_type: utils::data::TransactionType,
        from_user_id: u64,
        to_user_id: u64,
        amount: i64,
        timestamp_ms: u64,
        status: utils::data::Status,
    ) -> utils::data::Operation {
        utils::data::Operation::new(
            tx_id,
            tx_type,
            from_user_id,
            to_user_id,
            amount,
            timestamp_ms,
            status,
            None,
        )
    }

    fn statement_from_ops(mut ops: Vec<utils::data::Operation>) -> Statement {
        let mut r: &[u8] = &[];
        Statement::from_read(&mut r, move |_| Ok(ops.pop())).unwrap()
    }

    #[test]
    fn parse_format_prinimaet_ozhidaemye_aliasy() {
        assert_eq!(parse_format("txt"), Some(Format::Txt));
        assert_eq!(parse_format("TEXT"), Some(Format::Txt));
        assert_eq!(parse_format("csv"), Some(Format::Csv));
        assert_eq!(parse_format("bin"), Some(Format::Bin));
        assert_eq!(parse_format("binary"), Some(Format::Bin));
        assert_eq!(parse_format("unknown"), None);
    }

    #[test]
    fn statement_eq_ispolzuet_operatsii_bez_meta() {
        let a = statement_from_ops(vec![
            op_new(
                1,
                utils::data::TransactionType::Deposit,
                1,
                1,
                10,
                100,
                utils::data::Status::Success,
            ),
            op_new(
                2,
                utils::data::TransactionType::Transfer,
                1,
                2,
                20,
                110,
                utils::data::Status::Failure,
            ),
        ]);
        let b = statement_from_ops(vec![
            op_new(
                1,
                utils::data::TransactionType::Deposit,
                1,
                1,
                10,
                100,
                utils::data::Status::Success,
            ),
            op_new(
                2,
                utils::data::TransactionType::Transfer,
                1,
                2,
                20,
                110,
                utils::data::Status::Failure,
            ),
        ]);
        assert_eq!(a, b);
    }
}
