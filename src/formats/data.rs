/// здесь должны находиться все типы хранения информации
use std::cmp::Ordering;
// use std::collections::HashMap;


/// Перечисление возможных типов транзакции
/// Черты перечисления: Debug, Clone, PartialEq, PartialOrd, Eq, Ord
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum TransactionType {
    Deposit,
    Transfer,
    Withdrawal
}

/// Перечисление возможных результатов транзакции
/// Черты перечисления: Debug, Clone, PartialEq, PartialOrd, Eq, Ord
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Status {
    Success,
    Failure,
    Pending
}


/// Перечисление возможных типов записки
/// Черты перечисления: Debug, Clone
#[derive(Debug)]
pub enum AccountStatementType {
    YPBank,
    MT940,
    Camt053,
    Sber
}

/// Структура метаданных записки: Информация из шапки или в целом информация
/// Черты структуры: Display, Debug, Default
#[derive(Debug, Default)]
pub struct StatementMeta {
    pub source: Option<String>,   // например "YPBankCsv"
    pub account_id: Option<String>,
    pub generated_at_ms: Option<u64>,
}


/// Структура описания одной операции
/// Черты структуры: Display, Debug, PartialEq, Eq, Ord, PartialOrd
#[derive(Debug)]
pub struct Operation {
    tx_id: u64,
    tx_type: TransactionType,
    from_user_id: u64,
    to_user_id: u64,
    amount: i64,
    timestamp_ms: u64,
    status: Status,
    description: Option<String>,
}



impl Operation {
    pub fn new(
        tx_id: u64,
        tx_type: TransactionType,
        from_user_id: u64,
        to_user_id: u64,
        amount: i64,
        timestamp_ms: u64,
        status: Status,
        description: Option<String>,
    ) -> Self {
        Self {
            tx_id,
            tx_type,
            from_user_id,
            to_user_id,
            amount,
            timestamp_ms,
            status,
            description,
        }
    }

    // Геттеры нужны для write_to (иначе не достать поля извне impl Operation/Statement)
    pub fn tx_id(&self) -> u64 { self.tx_id }
    pub fn tx_type(&self) -> &TransactionType { &self.tx_type }
    pub fn from_user_id(&self) -> u64 { self.from_user_id }
    pub fn to_user_id(&self) -> u64 { self.to_user_id }
    pub fn amount(&self) -> i64 { self.amount }
    pub fn timestamp_ms(&self) -> u64 { self.timestamp_ms }
    pub fn status(&self) -> &Status { &self.status }
    pub fn description(&self) -> Option<&str> { self.description.as_deref() }

        #[inline]
    /// Выделяет ключевые элементы структуры в кортеж ссылок
    fn key(&self) -> (
        &u64,               // tx_id
        &TransactionType,
        &u64,               // from_user_id
        &u64,               // to_user_id
        &i64,               // amount
        &u64,               // timestamp_ms
        &Status,
    ) {
        (
            &self.tx_id,
            &self.tx_type,
            &self.from_user_id,
            &self.to_user_id,
            &self.amount,
            &self.timestamp_ms,
            &self.status,
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

use crate::error::AppError;
use std::io::{Read, Write};
/// Структура хранения записки целиком
/// Черты структуры: Display, Debug, PartialEq, Eq
#[derive(Debug)]
pub struct Statement {
    operations: Vec<Operation>,
    pub meta: StatementMeta, // опционально
}
// Методы для добавления данных
impl Statement {
    /// Добавить одну запись в порядке возрастания
    pub fn append(&mut self, op: Operation) {
        let idx = match self.operations.binary_search(&op) {
            Ok(i) | Err(i) => i,
        };
        self.operations.insert(idx, op);
    }

    /// Добавить группу записей в векторе с последующей сортировкой
    // Если нужно “добавить пачку”, выгоднее append + sort один раз:
    pub fn extend_and_sort(&mut self, mut ops: Vec<Operation>) {
        self.operations.append(&mut ops);
        self.operations.sort_unstable(); // Ord без description
    }

    pub fn from_read<R: Read, Next>(
        r: &mut R,
        mut next_op: Next,
    ) -> Result<Self, AppError>
    where
        Next: FnMut(&mut R) -> Result<Option<Operation>, AppError>,
    {
        let mut st = Statement { operations: Vec::new(), meta: StatementMeta::default() };

        while let Some(op) = next_op(r)? {
            st.append(op);
        }

        Ok(st)
    }

    pub fn write_to<W: Write, F>(
        &mut self,
        w: &mut W,
        mut write_op: F,
    ) -> Result<(), AppError>
    where
        F: FnMut(&mut W, &Operation) -> Result<(), AppError>,
    {
        // self.operations.sort_unstable();

        for op in &self.operations {
            write_op(w, op)?;
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Хелперы -------------------------------------------------------------

    fn meta(src: &str, acc: &str, ts: u64) -> StatementMeta {
        StatementMeta {
            source: Some(src.to_string()),
            account_id: Some(acc.to_string()),
            generated_at_ms: Some(ts),
        }
    }

    fn op(
        tx_id: u64,
        tx_type: TransactionType,
        from_user_id: u64,
        to_user_id: u64,
        amount: i64,
        timestamp_ms: u64,
        status: Status,
        description: Option<&str>,
    ) -> Operation {
        Operation {
            tx_id,
            tx_type,
            from_user_id,
            to_user_id,
            amount,
            timestamp_ms,
            status,
            description: description.map(|s| s.to_string()),
        }
    }

    fn empty_statement_with_meta() -> Statement {
        Statement {
            operations: vec![],
            meta: meta("TestSource", "acc-1", 123),
        }
    }

    fn assert_sorted_ops(ops: &[Operation]) {
        assert!(
            ops.windows(2).all(|w| w[0] <= w[1]),
            "operations должны быть отсортированы по Ord"
        );
    }

    // --- Operation: Eq/Ord/PartialOrd ---------------------------------------

    #[test]
    fn operation_eq_ignores_description() {
        let a = op(
            1,
            TransactionType::Deposit,
            10,
            11,
            100,
            1_000,
            Status::Success,
            Some("desc A"),
        );
        let b = op(
            1,
            TransactionType::Deposit,
            10,
            11,
            100,
            1_000,
            Status::Success,
            Some("desc B"),
        );

        assert_eq!(a, b, "description не должен влиять на Eq/Ord");
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
    }

    #[test]
    fn operation_ord_primary_by_tx_id() {
        let a = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        let b = op(2, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_tx_type() {
        let a = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        let b = op(1, TransactionType::Transfer, 1, 2, 10, 100, Status::Success, None);

        // По derive(Ord) у enum: Deposit < Transfer < Withdrawal (в порядке объявления)
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_from_user_id() {
        let a = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        let b = op(1, TransactionType::Deposit, 2, 2, 10, 100, Status::Success, None);
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_to_user_id() {
        let a = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        let b = op(1, TransactionType::Deposit, 1, 3, 10, 100, Status::Success, None);
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_amount_including_negative() {
        let a = op(1, TransactionType::Deposit, 1, 2, -5, 100, Status::Success, None);
        let b = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_timestamp() {
        let a = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        let b = op(1, TransactionType::Deposit, 1, 2, 10, 200, Status::Success, None);
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_status() {
        let a = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        let b = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Failure, None);

        // По объявлению enum: Success < Failure < Pending
        assert!(a < b);
    }

    #[test]
    fn operation_partial_ord_is_total_order() {
        let a = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        let b = op(2, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);

        assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less));
        assert_eq!(b.partial_cmp(&a), Some(std::cmp::Ordering::Greater));
        assert_eq!(a.partial_cmp(&a), Some(std::cmp::Ordering::Equal));
    }

    // --- Statement: append / extend_and_sort --------------------------------

    #[test]
    fn statement_append_inserts_into_sorted_position_basic() {
        let mut s = empty_statement_with_meta();

        // вставляем намеренно "вразнобой"
        s.append(op(2, TransactionType::Deposit, 1, 1, 10, 100, Status::Success, None));
        s.append(op(1, TransactionType::Deposit, 1, 1, 10, 100, Status::Success, None));
        s.append(op(3, TransactionType::Deposit, 1, 1, 10, 100, Status::Success, None));

        assert_eq!(s.operations.len(), 3);
        assert_sorted_ops(&s.operations);

        assert_eq!(s.operations[0].tx_id, 1);
        assert_eq!(s.operations[1].tx_id, 2);
        assert_eq!(s.operations[2].tx_id, 3);
    }

    #[test]
    fn statement_append_keeps_sorted_for_many_fields_not_only_tx_id() {
        let mut s = empty_statement_with_meta();

        // одинаковый tx_id, но разные поля => сортировка по key()
        let a = op(1, TransactionType::Transfer, 2, 1, 5, 100, Status::Success, None);
        let b = op(1, TransactionType::Deposit, 2, 1, 5, 100, Status::Success, None);
        let c = op(1, TransactionType::Withdrawal, 1, 1, 5, 100, Status::Success, None);

        // clone запрещён — просто создаём значения и сразу добавляем
        s.append(a);
        s.append(b);
        s.append(c);

        assert_sorted_ops(&s.operations);

        // Deposit < Transfer < Withdrawal
        assert_eq!(s.operations[0].tx_type, TransactionType::Deposit);
        assert_eq!(s.operations[1].tx_type, TransactionType::Transfer);
        assert_eq!(s.operations[2].tx_type, TransactionType::Withdrawal);
    }

    #[test]
    fn statement_append_allows_duplicates_by_key() {
        let mut s = empty_statement_with_meta();

        let a = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, Some("a"));
        let b = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, Some("b"));

        // Они равны по key(), binary_search вернет Ok(i), insert(i, op) => вставит рядом
        s.append(a);
        s.append(b);

        assert_eq!(s.operations.len(), 2);
        assert_eq!(s.operations[0], s.operations[1]);
        assert_sorted_ops(&s.operations);
    }

    #[test]
    fn extend_and_sort_sorts_once() {
        let mut s = empty_statement_with_meta();

        s.append(op(5, TransactionType::Deposit, 1, 1, 10, 100, Status::Success, None));

        let mut batch = vec![
            op(3, TransactionType::Deposit, 1, 1, 10, 100, Status::Success, None),
            op(1, TransactionType::Deposit, 1, 1, 10, 100, Status::Success, None),
            op(4, TransactionType::Deposit, 1, 1, 10, 100, Status::Success, None),
            op(2, TransactionType::Deposit, 1, 1, 10, 100, Status::Success, None),
        ];

        // специально сделаем batch еще и с "лишним" описанием
        batch.push(op(
            2,
            TransactionType::Deposit,
            1,
            1,
            10,
            100,
            Status::Success,
            Some("dup with different description"),
        ));

        s.extend_and_sort(batch);

        assert_eq!(s.operations.len(), 6);
        assert_sorted_ops(&s.operations);

        // tx_id должны начинаться с 1
        assert_eq!(s.operations[0].tx_id, 1);
        // два элемента с tx_id=2 равны по key()
        let two_count = s.operations.iter().filter(|x| x.tx_id == 2).count();
        assert_eq!(two_count, 2);
    }

    // --- Statement: Eq -------------------------------------------------------

    #[test]
    fn statement_eq_true_when_operations_identical() {
        let mut a = empty_statement_with_meta();
        let mut b = empty_statement_with_meta();

        // clone запрещён — создаём два одинаковых набора отдельно
        let ops_a = vec![
            op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None),
            op(2, TransactionType::Transfer, 2, 3, 20, 110, Status::Failure, Some("x")),
            op(3, TransactionType::Withdrawal, 3, 0, 30, 120, Status::Pending, Some("y")),
        ];
        let ops_b = vec![
            op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None),
            op(2, TransactionType::Transfer, 2, 3, 20, 110, Status::Failure, Some("x")),
            op(3, TransactionType::Withdrawal, 3, 0, 30, 120, Status::Pending, Some("y")),
        ];

        for o in ops_a {
            a.append(o);
        }
        for o in ops_b {
            b.append(o);
        }

        assert_eq!(a, b);
    }

    #[test]
    fn statement_eq_ignores_meta_by_design() {
        let mut a = Statement {
            operations: vec![],
            meta: meta("A", "acc-1", 111),
        };
        let mut b = Statement {
            operations: vec![],
            meta: meta("B", "acc-2", 222),
        };

        a.append(op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None));
        b.append(op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None));

        // У тебя PartialEq сравнивает только operations
        assert_eq!(a, b);
    }

    #[test]
    fn statement_eq_false_when_one_operation_differs_by_key() {
        let mut a = empty_statement_with_meta();
        let mut b = empty_statement_with_meta();

        a.append(op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None));
        b.append(op(1, TransactionType::Deposit, 1, 2, 11, 100, Status::Success, None)); // amount differs

        assert_ne!(a, b);
    }

    #[test]
    fn statement_eq_false_when_same_set_but_different_order() {
        // ВАЖНО: твой Eq = сравнение Vec как есть.
        // Поэтому если операции одинаковые, но в разных порядках (и без сортировки),
        // будет false. Этот тест фиксирует текущее поведение.

        let op1a = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        let op2a = op(2, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);

        let op1b = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        let op2b = op(2, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);

        let a = Statement {
            operations: vec![op1a, op2a],
            meta: StatementMeta::default(),
        };
        let b = Statement {
            operations: vec![op2b, op1b],
            meta: StatementMeta::default(),
        };

        assert_ne!(a, b, "Eq у Statement сейчас чувствителен к порядку в Vec");
    }

    #[test]
    fn statement_eq_true_after_sorting_same_set() {
        let op1a = op(2, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        let op2a = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);

        let op1b = op(2, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);
        let op2b = op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None);

        let mut a = Statement {
            operations: vec![op1a, op2a],
            meta: StatementMeta::default(),
        };
        let mut b = Statement {
            operations: vec![op2b, op1b],
            meta: StatementMeta::default(),
        };

        a.operations.sort_unstable();
        b.operations.sort_unstable();

        assert_eq!(a, b);
    }

    // --- Edge cases ----------------------------------------------------------

    #[test]
    fn empty_statements_are_equal() {
        let a = Statement {
            operations: vec![],
            meta: StatementMeta::default(),
        };
        let b = Statement {
            operations: vec![],
            meta: meta("X", "Y", 999),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn append_many_mixed_cases_and_verify_sorted() {
        let mut s = empty_statement_with_meta();

        let ops = vec![
            op(10, TransactionType::Transfer, 100, 200, 1, 900, Status::Failure, Some("a")),
            op(10, TransactionType::Transfer, 100, 200, 1, 900, Status::Success, Some("b")), // status differs => key differs
            op(9, TransactionType::Deposit, 99, 0, -100, 800, Status::Pending, None),
            op(9, TransactionType::Deposit, 99, 0, -100, 800, Status::Pending, Some("ignored")),
            op(11, TransactionType::Withdrawal, 0, 1, 500, 1_000, Status::Success, None),
            op(1, TransactionType::Deposit, 1, 2, 10, 100, Status::Success, None),
            op(1, TransactionType::Deposit, 1, 2, 9, 100, Status::Success, None), // amount differs
        ];

        for o in ops {
            s.append(o);
        }

        assert_eq!(s.operations.len(), 7);
        assert_sorted_ops(&s.operations);

        // Быстрая sanity-проверка первого и последнего
        assert_eq!(s.operations.first().unwrap().tx_id, 1);
        assert_eq!(s.operations.last().unwrap().tx_id, 11);

        // Два одинаковых по key (9/Deposit/99/0/-100/800/Pending) должны быть рядом
        let idxs: Vec<usize> = s
            .operations
            .iter()
            .enumerate()
            .filter(|(_, x)| {
                x.tx_id == 9
                    && x.tx_type == TransactionType::Deposit
                    && x.from_user_id == 99
                    && x.to_user_id == 0
                    && x.amount == -100
                    && x.timestamp_ms == 800
                    && x.status == Status::Pending
            })
            .map(|(i, _)| i)
            .collect();

        assert_eq!(idxs.len(), 2);
        assert_eq!(idxs[1], idxs[0] + 1, "дубликаты по key должны стоять рядом");
    }
}
