use std::collections::HashMap;

enum TransactionType {
    Deposit,
    Transfer,
    Withdrawal
}

enum Status {
    Success,
    Failure,
    Pending
}

enum AccountStatementType {
    YPBank,
    MT940,
    camt053,
    Sber
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub struct Statement {
    pub operations: Vec<Operation>,
    pub meta: StatementMeta, // опционально
}

pub struct StatementMeta {
    pub source: Option<String>,   // например "YPBankCsv"
    pub account_id: Option<String>,
    pub generated_at_ms: Option<u64>,
}