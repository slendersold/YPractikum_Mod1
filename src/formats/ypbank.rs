//! Набор форматов YPBank и связанных модулей.

/// Бинарный формат YPBank.
pub mod bin;
/// CSV-формат YPBank.
pub mod csv;
mod fields;
/// Текстовый формат YPBank.
pub mod txt;

// pub use csv::*;
// pub use txt::*;
// pub use bin::*;
