//! Библиотека форматов и доменной модели для задания №1.

mod formats;

/// Доменная модель (выписка/операции).
pub use formats::data;
/// Ошибки и типы результата.
pub use formats::errors;
/// Форматы YPBank (csv/txt/bin).
pub use formats::ypbank;
