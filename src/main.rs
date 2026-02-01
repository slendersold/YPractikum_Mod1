// src/main.rs
//
// Главная CLI-утилита: конвертер форматов YPBank (txt/csv/bin)
//
// Примеры:
//   ypbank-convert --in txt --out csv -i input.txt -o output.csv
//   ypbank-convert --in csv --out bin -i input.csv -o output.ypb
//   ypbank-convert --in bin --out txt -i input.ypb -o output.txt
//
// Если -i не указан — читается stdin.
// Если -o не указан — пишется stdout.

use std::env;
use std::fs::File;
use std::io::{self, Read, Write};

use utils::data::Statement;
use utils::errors::{DataError, Result};
use utils::ypbank::{bin as ybin, csv as ycsv, txt as ytxt};

/// Поддерживаемые форматы входных и выходных данных.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Txt,
    Csv,
    Bin,
}

/// Разбирает строковое имя формата.
fn parse_format(s: &str) -> Option<Format> {
    match s.to_ascii_lowercase().as_str() {
        "txt" => Some(Format::Txt),
        "csv" => Some(Format::Csv),
        "bin" => Some(Format::Bin),
        _ => None,
    }
}

/// Текст справки по CLI.
fn usage() -> &'static str {
    r#"Usage:
  ypbank-convert --in <txt|csv|bin> --out <txt|csv|bin> [-i <path>] [-o <path>]

Options:
  --in,  -I    Input format:  txt | csv | bin
  --out, -O    Output format: txt | csv | bin
  -i            Input file (default: stdin)
  -o            Output file (default: stdout)
  -h, --help    Show this help
"#
}

/// Точка входа CLI-конвертера.
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

/// Основной сценарий: парсинг аргументов, чтение, конвертация, запись.
fn run() -> Result<()> {
    let mut in_fmt: Option<Format> = None;
    let mut out_fmt: Option<Format> = None;
    let mut in_path: Option<String> = None;
    let mut out_path: Option<String> = None;

    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{}", usage());
                return Ok(());
            }
            "--in" | "-I" => {
                let v = args
                    .next()
                    .ok_or_else(|| DataError::Format("Missing value for --in".into()))?;
                in_fmt = Some(
                    parse_format(&v)
                        .ok_or_else(|| DataError::Format("Unknown --in format".into()))?,
                );
            }
            "--out" | "-O" => {
                let v = args
                    .next()
                    .ok_or_else(|| DataError::Format("Missing value for --out".into()))?;
                out_fmt = Some(
                    parse_format(&v)
                        .ok_or_else(|| DataError::Format("Unknown --out format".into()))?,
                );
            }
            "-i" => {
                in_path = Some(
                    args.next()
                        .ok_or_else(|| DataError::Format("Missing value for -i".into()))?,
                );
            }
            "-o" => {
                out_path = Some(
                    args.next()
                        .ok_or_else(|| DataError::Format("Missing value for -o".into()))?,
                );
            }
            _ => return Err(DataError::Format(format!("Unknown argument: {a}"))),
        }
    }

    let in_fmt = in_fmt.ok_or_else(|| DataError::Format("Missing --in".into()))?;
    let out_fmt = out_fmt.ok_or_else(|| DataError::Format("Missing --out".into()))?;

    // --- input ---
    let mut input: Box<dyn Read> = match in_path {
        Some(p) => Box::new(File::open(p).map_err(DataError::Io)?),
        None => Box::new(io::stdin().lock()),
    };

    // --- parse -> Statement ---
    let mut statement = read_statement(in_fmt, &mut input)?;

    // --- output ---
    let mut output: Box<dyn Write> = match out_path {
        Some(p) => Box::new(File::create(p).map_err(DataError::Io)?),
        None => Box::new(io::stdout().lock()),
    };

    write_statement(out_fmt, &mut statement, &mut output)?;
    output.flush().map_err(DataError::Io)?;
    Ok(())
}

/// Читает выписку в `Statement` с учетом выбранного формата.
fn read_statement<R: Read>(fmt: Format, r: &mut R) -> Result<Statement> {
    // Statement::from_read требует &mut R, но парсеры держат &mut R внутри себя.
    // Поэтому используем "пустой" reader и замыкание, которое читает из parser.next_op().
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

/// Записывает `Statement` в выбранном формате.
fn write_statement<W: Write>(fmt: Format, st: &mut Statement, w: &mut W) -> Result<()> {
    match fmt {
        Format::Txt => {
            let write_op = ytxt::make_write_op::<W>();
            st.write_to(w, write_op)
        }
        Format::Csv => {
            ycsv::YPBankCsv::<io::Empty>::write_header(w)?;
            let write_op = ycsv::make_write_op::<W>();
            st.write_to(w, write_op)
        }
        Format::Bin => {
            let write_op = ybin::make_write_op::<W>();
            st.write_to(w, write_op)
        }
    }
}
