//! Ошибки парсинга/формата и тип результата для библиотеки.

use std::{fmt, io};

/// Единый Result для всего проекта
pub type Result<T> = std::result::Result<T, DataError>;

/// Ошибки данных (парсинг/формат/семантика) + I/O
#[derive(Debug)]
pub enum DataError {
    /// Ошибка работы с вводом/выводом (файл, stdin/stdout, сокет и т.п.)
    Io(io::Error),

    /// Ошибка конкретной строки (совместимость со старым интерфейсом)
    BadLine {
        line_no: usize,
        line: String,
        msg: String,
    },

    /// Общая ошибка формата без номера строки
    Format(String),

    /// Структурированная ошибка формата
    Parse(ParseError),
}

impl From<io::Error> for DataError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Структурированная ошибка парсинга.
///
/// `line` хранит исходную строку (или её фрагмент) только при необходимости.
/// Если не хочется тащить большие строки, можно передавать пустую строку.
#[derive(Debug)]
pub struct ParseError {
    pub line_no: usize,
    pub field: Option<&'static str>,
    pub kind: ParseErrorKind,
    pub line: String,
}

///
/// Классификация ошибок, чтобы парсер не собирал текстовые сообщения вручную.
///
/// Любой вариант содержит ровно ту информацию, которая нужна для формирования
/// понятного сообщения в Display.
///
#[derive(Debug)]
pub enum ParseErrorKind {
    /// Ожидалась строка вида KEY: VALUE (не нашли ':')
    ExpectedKeyValue,

    /// Встречено неизвестное поле
    UnknownField { name: String },

    /// Поле повторилось в пределах одной записи
    DuplicateField { name: &'static str },

    /// В записи отсутствуют обязательные поля
    MissingFields { names: Vec<&'static str> },

    /// Значение не распарсилось в число
    BadNumber { value: String, ty: &'static str },

    /// Значение распарсилось, но не проходит ограничение ">= 0"
    NegativeNotAllowed { value: String },

    /// Значение не входит в допустимые перечисления (TX_TYPE/STATUS)
    BadEnum {
        value: String,
        expected: &'static str,
    },

    /// Ожидалась строка в двойных кавычках
    BadQuotedString { value: String },

    /// Нарушение правил формата на уровне смысла (DEPOSIT/FROM_USER_ID и т.п.)
    SemanticRule { msg: &'static str },
}

impl ParseError {
    /// Создать ParseError без привязки к полю.
    pub fn new(line_no: usize, kind: ParseErrorKind) -> Self {
        Self {
            line_no,
            field: None,
            kind,
            line: String::new(),
        }
    }

    /// Добавить имя поля.
    pub fn with_field(mut self, field: &'static str) -> Self {
        self.field = Some(field);
        self
    }

    /// Добавить исходную строку (или её фрагмент) в ошибку.
    pub fn with_line(mut self, line: impl Into<String>) -> Self {
        self.line = line.into();
        self
    }

    /// Сокращённый конструктор: line_no + field + kind + line
    pub fn full(
        line_no: usize,
        field: Option<&'static str>,
        kind: ParseErrorKind,
        line: impl Into<String>,
    ) -> Self {
        Self {
            line_no,
            field,
            kind,
            line: line.into(),
        }
    }
}

impl From<ParseError> for DataError {
    fn from(e: ParseError) -> Self {
        DataError::Parse(e)
    }
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::Io(e) => write!(f, "I/O error: {e}"),
            DataError::Format(s) => write!(f, "Format error: {s}"),
            DataError::BadLine { line_no, msg, line } => {
                if line.is_empty() {
                    write!(f, "Line {line_no}: {msg}")
                } else {
                    write!(f, "Line {line_no}: {msg} (input: {line})")
                }
            }
            DataError::Parse(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DataError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let where_ = match self.field {
            Some(field) => format!("Line {}, field {}", self.line_no, field),
            None => format!("Line {}", self.line_no),
        };

        use ParseErrorKind::*;
        match &self.kind {
            ExpectedKeyValue => write!(f, "{where_}: expected `KEY: VALUE`"),
            UnknownField { name } => write!(f, "{where_}: unknown field `{name}`"),
            DuplicateField { name } => write!(f, "{where_}: duplicate field `{name}`"),
            MissingFields { names } => write!(f, "{where_}: missing fields: {}", names.join(", ")),
            BadNumber { value, ty } => write!(f, "{where_}: bad {ty} number `{value}`"),
            NegativeNotAllowed { value } => {
                write!(f, "{where_}: negative value not allowed `{value}`")
            }
            BadEnum { value, expected } => {
                write!(f, "{where_}: bad value `{value}`, expected {expected}")
            }
            BadQuotedString { value } => {
                write!(f, "{where_}: expected quoted string, got `{value}`")
            }
            SemanticRule { msg } => write!(f, "{where_}: {msg}"),
        }?;

        if !self.line.is_empty() {
            write!(f, " (input: {})", self.line)?;
        }
        Ok(())
    }
}

///
/// Макрос для лаконичного создания структурированных ошибок парсинга.
///
/// Примеры:
/// ```ignore
/// return Err(parse_err!(line_no, "TX_ID", BadNumber { value: v.to_string(), ty: "u64" }, raw_line));
/// return Err(parse_err!(line_no, None, ExpectedKeyValue, raw_line));
/// ```
#[macro_export]
macro_rules! parse_err {
    ($line_no:expr, $field:expr, $kind:expr, $line:expr) => {{
        $crate::errors::DataError::from($crate::errors::ParseError::full(
            $line_no, $field, $kind, $line,
        ))
    }};
}
