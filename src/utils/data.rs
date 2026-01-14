use std::collections::HashMap;

enum TransactionType {
    DEPOSIT,
    TRANSFER,
    WITHDRAWAL
}

enum Status {
    SUCCESS,
    FAILURE,
    PENDING
}

struct Operation {
    TX_ID: u64,// беззнаковое 64-битное | Уникальный идентификатор транзакции. |
    TX_TYPE: TransactionType,// перечисление (0 = DEPOSIT, 1 = TRANSFER, 2 = WITHDRAWAL) | |
    FROM_USER_ID: u64,// беззнаковое 64-битное | Счёт отправителя; `0` для DEPOSIT. |
    TO_USER_ID: u64,// беззнаковое 64-битное | Счёт получателя; `0` для WITHDRAWAL. |
    AMOUNT: i64,// знаковое 64-битное | Сумма в наименьшей денежной единице (центах). Положительное значение для зачислений, отрицательное для списаний. |
    TIMESTAMP: u64,// беззнаковое 64-битное | Время выполнения транзакции в миллисекундах от эпохи Unix. |
    STATUS: Status,// перечисление (0 = SUCCESS, 1 = FAILURE, 2 = PENDING) | |
    // DESC_LEN: u32,// беззнаковое 32-битное | Длина следующего описания в кодировке UTF-8. |
    DESCRIPTION: String// UTF-8 | Необязательное текстовое описание. Если описание отсутствует, `DESC_LEN` равен `0`. |
}

enum AccountStatementType {
    YPBank,
    MT940,
    camt053,
    Sber
}


struct Data {
    statement: AccountStatementType,
    HeaderData: String,
    data: Vec<Operation>
}