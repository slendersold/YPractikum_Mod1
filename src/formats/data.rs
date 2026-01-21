// здесь должны находиться все типы хранения информации
use std::cmp::Ordering;
use std::collections::HashMap;

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
#[derive(Display, Debug)]
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

impl Operation {
    #[inline]
    fn key(&self) -> (
        u64,            // tx_id
        TransactionType,
        u64,            // from_user_id
        u64,            // to_user_id
        i64,            // amount
        u64,            // timestamp_ms
        Status,
    ) {
        (
            self.tx_id,
            self.tx_type,
            self.from_user_id,
            self.to_user_id,
            self.amount,
            self.timestamp_ms,
            self.status,
        )
    }
}
// Сначала имплементируем сравниваемость
impl Ord for Operation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}
impl PartialEq for Operation {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}
impl Eq for Operation {}
// Потом выполняем условия наследования
impl PartialOrd for Operation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other)) // делаем полный порядок
    }
}


// Структура хранения записки целиком
#[derive(Display, Debug)]
pub struct Statement {
    operations: Vec<Operation>,
    pub meta: StatementMeta, // опционально
}
// Методы для добавления данных
impl Statement {
    // Добавить одну в порядке возрастания
    pub fn append(&mut self, op: Operation) {
        let idx = match self.operations.binary_search(&op) {
            Ok(i) | Err(i) => i,
        };
        self.operations.insert(idx, op);
    }

    // Если нужно “добавить пачку”, выгоднее append + sort один раз:
    pub fn extend_and_sort(&mut self, mut ops: Vec<Operation>) {
        self.operations.append(&mut ops);
        self.operations.sort_unstable(); // Ord без description
    }
}

// Имплементируем равенство списков для выполнения задания
impl PartialEq for Statement {
    fn eq(&self, other: &Self) -> bool {
        // попробуем сначала проверку двух отсортированных массивов
        self.operations == other.operations
        // // Ранний выход, можно попробовать в будущем поменять на истинное частичное сравнение, 
        // // когда проверяется один список на подмножество другого
        // if self.operations.len() != other.operations.len() {
        //     return false;
        // }

        // let mut a: Vec<&Operation> = self.operations.iter().collect();
        // let mut b: Vec<&Operation> = other.operations.iter().collect();

        // // // Сортируем ВЕКТОРЫ ССЫЛОК. Наверное пока не релевантно
        // // a.sort_unstable(); // использует Ord у Operation (без description)
        // // b.sort_unstable();

        // a.into_iter().zip(b.into_iter()).all(|(x, y)| x == y) // Eq у Operation (без description)
    }
}
impl Eq for Statement {}