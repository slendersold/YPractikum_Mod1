// здесь должны находиться все типы хранения информации

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum TransactionType {
    Deposit,
    Transfer,
    Withdrawal
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Status {
    Success,
    Failure,
    Pending
}

#[derive(Debug, Clone)]
enum AccountStatementType {
    YPBank,
    MT940,
    camt053,
    Sber
}

// Структура метаданных записки: Информация из шапки или в целом информация
#[derive(Display, Debug, Default)]
pub struct StatementMeta {
    pub source: Option<String>,   // например "YPBankCsv"
    pub account_id: Option<String>,
    pub generated_at_ms: Option<u64>,
}

// Структура описания одной операции
#[derive(Display, Debug, PartialOrd, PartialEq, Eq)]
pub struct Operation {
    pub tx_id: u64,
    pub tx_type: TransactionType,
    pub from_user_id: u64,
    pub to_user_id: u64,
    pub amount: i64,
    pub timestamp_ms: u64,
    pub status: Status,
    pub description: Option<String>,
}

// убрать #[derive(PartialOrd, PartialEq, Eq)] и прописать свою логику без description в эти черты

// Структура хранения записки целиком
#[derive(Display, Debug)]
pub struct Statement {
    pub operations: Vec<Operation>,
    pub meta: StatementMeta, // опционально
}

// Добавить и имплементирвать черту сравниваемости, добавить функционал пополнения внутреннего массива с предварительной сортировкой
