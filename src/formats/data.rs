//! Доменная модель: операции, выписки и связанные типы.

/// Здесь находятся все типы хранения информации.
use std::cmp::Ordering;
// use std::collections::HashMap;

/// Возможные типы транзакции.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum TransactionType {
    Deposit,
    Transfer,
    Withdrawal,
}

/// Возможные статусы транзакции.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Status {
    Success,
    Failure,
    Pending,
}

/// Возможные типы выписок (для классификации источника).
#[derive(Debug)]
pub enum AccountStatementType {
    YPBank,
    MT940,
    Camt053,
    Sber,
}

/// Метаданные выписки (происхождение и служебные поля).
#[derive(Debug, Default)]
pub struct StatementMeta {
    pub source: Option<String>, // например "YPBankCsv"
    pub account_id: Option<String>,
    pub generated_at_ms: Option<u64>,
}

/// Описание одной операции.
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
    /// Создает новую операцию.
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

    /// Возвращает идентификатор транзакции.
    pub fn tx_id(&self) -> u64 {
        self.tx_id
    }
    /// Возвращает тип транзакции.
    pub fn tx_type(&self) -> &TransactionType {
        &self.tx_type
    }
    /// Возвращает идентификатор отправителя.
    pub fn from_user_id(&self) -> u64 {
        self.from_user_id
    }
    /// Возвращает идентификатор получателя.
    pub fn to_user_id(&self) -> u64 {
        self.to_user_id
    }
    /// Возвращает сумму транзакции.
    pub fn amount(&self) -> i64 {
        self.amount
    }
    /// Возвращает временную метку.
    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }
    /// Возвращает статус транзакции.
    pub fn status(&self) -> &Status {
        &self.status
    }
    /// Возвращает описание, если есть.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    #[inline]
    /// Выделяет ключевые элементы структуры в кортеж ссылок
    fn key(
        &self,
    ) -> (
        &u64, // tx_id
        &TransactionType,
        &u64, // from_user_id
        &u64, // to_user_id
        &i64, // amount
        &u64, // timestamp_ms
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

use crate::errors::DataError;
use std::io::{Read, Write};
/// Выписка целиком (набор операций + метаданные).
#[derive(Debug)]
pub struct Statement {
    operations: Vec<Operation>,
    pub meta: StatementMeta, // опционально
}
// Методы для добавления данных
impl Statement {
    /// Добавляет одну операцию с сохранением сортировки.
    pub fn append(&mut self, op: Operation) {
        let idx = match self.operations.binary_search(&op) {
            Ok(i) | Err(i) => i,
        };
        self.operations.insert(idx, op);
    }

    /// Добавляет несколько операций и сортирует один раз.
    // Если нужно “добавить пачку”, выгоднее append + sort один раз:
    pub fn extend_and_sort(&mut self, mut ops: Vec<Operation>) {
        self.operations.append(&mut ops);
        self.operations.sort_unstable(); // Ord без description
    }

    /// Читает операции через стратегию `next_op` и строит выписку.
    pub fn from_read<R: Read, Next>(r: &mut R, mut next_op: Next) -> Result<Self, DataError>
    where
        Next: FnMut(&mut R) -> Result<Option<Operation>, DataError>,
    {
        let mut st = Statement {
            operations: Vec::new(),
            meta: StatementMeta::default(),
        };

        while let Some(op) = next_op(r)? {
            st.append(op);
        }

        Ok(st)
    }

    /// Записывает операции через стратегию `write_op`.
    pub fn write_to<W: Write, F>(&mut self, w: &mut W, mut write_op: F) -> Result<(), DataError>
    where
        F: FnMut(&mut W, &Operation) -> Result<(), DataError>,
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
    use std::io;

    // --- Helpers -------------------------------------------------------------

    fn meta(src: &str, acc: &str, ts: u64) -> StatementMeta {
        StatementMeta {
            source: Some(src.to_string()),
            account_id: Some(acc.to_string()),
            generated_at_ms: Some(ts),
        }
    }

    fn op_new(
        tx_id: u64,
        tx_type: TransactionType,
        from_user_id: u64,
        to_user_id: u64,
        amount: i64,
        timestamp_ms: u64,
        status: Status,
        description: Option<&str>,
    ) -> Operation {
        Operation::new(
            tx_id,
            tx_type,
            from_user_id,
            to_user_id,
            amount,
            timestamp_ms,
            status,
            description.map(|s| s.to_string()),
        )
    }

    fn empty_statement() -> Statement {
        // Creates an empty statement through the public constructor path (from_read),
        // without relying on private fields.
        let mut r: &[u8] = &[];
        Statement::from_read(&mut r, |_| Ok(None)).unwrap()
    }

    fn tx_type_name(t: &TransactionType) -> String {
        match t {
            TransactionType::Deposit => "Deposit".to_string(),
            TransactionType::Transfer => "Transfer".to_string(),
            TransactionType::Withdrawal => "Withdrawal".to_string(),
        }
    }

    fn status_name(s: &Status) -> String {
        match s {
            Status::Success => "Success".to_string(),
            Status::Failure => "Failure".to_string(),
            Status::Pending => "Pending".to_string(),
        }
    }

    fn key_string(op: &Operation) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            op.tx_id(),
            tx_type_name(op.tx_type()),
            op.from_user_id(),
            op.to_user_id(),
            op.amount(),
            op.timestamp_ms(),
            status_name(op.status()),
        )
    }

    fn collect_keys_via_write_to(st: &mut Statement) -> Vec<String> {
        let mut out: Vec<u8> = Vec::new();
        let mut keys: Vec<String> = Vec::new();
        st.write_to(&mut out, |_, op| {
            keys.push(key_string(op));
            Ok(())
        })
        .unwrap();
        keys
    }

    fn assert_sorted_keys(keys: &[String]) {
        assert!(
            keys.windows(2).all(|w| w[0] <= w[1]),
            "Checks that the written operations are sorted by Operation::Ord key"
        );
    }

    fn statement_from_ops(mut ops: Vec<Operation>, meta: StatementMeta) -> Statement {
        // Builds a statement using the public from_read path and moves operations out of a Vec.
        let mut r: &[u8] = &[];
        let mut st = Statement::from_read(&mut r, move |_| {
            Ok(ops.pop()) // pop moves values out; order does not matter because append sorts
        })
        .unwrap();
        st.meta = meta;
        st
    }

    // --- Operation: Eq/Ord/PartialOrd ---------------------------------------

    #[test]
    fn operation_eq_ignores_description() {
        // Checks that description does not affect equality and ordering.
        let a = op_new(
            1,
            TransactionType::Deposit,
            10,
            11,
            100,
            1_000,
            Status::Success,
            Some("desc A"),
        );
        let b = op_new(
            1,
            TransactionType::Deposit,
            10,
            11,
            100,
            1_000,
            Status::Success,
            Some("desc B"),
        );

        assert_eq!(a, b);
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
    }

    #[test]
    fn operation_ord_primary_by_tx_id() {
        // Checks that tx_id is the primary ordering key.
        let a = op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        );
        let b = op_new(
            2,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        );
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_tx_type() {
        // Checks that tx_type participates in ordering after tx_id.
        let a = op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        );
        let b = op_new(
            1,
            TransactionType::Transfer,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        );
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_from_user_id() {
        // Checks that from_user_id participates in ordering after tx_type.
        let a = op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        );
        let b = op_new(
            1,
            TransactionType::Deposit,
            2,
            2,
            10,
            100,
            Status::Success,
            None,
        );
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_to_user_id() {
        // Checks that to_user_id participates in ordering after from_user_id.
        let a = op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        );
        let b = op_new(
            1,
            TransactionType::Deposit,
            1,
            3,
            10,
            100,
            Status::Success,
            None,
        );
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_amount_including_negative() {
        // Checks that amount participates in ordering and handles negative values.
        let a = op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            -5,
            100,
            Status::Success,
            None,
        );
        let b = op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        );
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_timestamp() {
        // Checks that timestamp_ms participates in ordering after amount.
        let a = op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        );
        let b = op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            200,
            Status::Success,
            None,
        );
        assert!(a < b);
    }

    #[test]
    fn operation_ord_then_by_status() {
        // Checks that status participates in ordering after timestamp_ms.
        let a = op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        );
        let b = op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Failure,
            None,
        );
        assert!(a < b);
    }

    #[test]
    fn operation_partial_ord_is_total_order() {
        // Checks that partial_cmp is consistent with cmp and forms a total order.
        let a = op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        );
        let b = op_new(
            2,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        );

        assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less));
        assert_eq!(b.partial_cmp(&a), Some(std::cmp::Ordering::Greater));
        assert_eq!(a.partial_cmp(&a), Some(std::cmp::Ordering::Equal));
    }

    // --- Statement: append / extend_and_sort --------------------------------

    #[test]
    fn statement_append_inserts_sorted() {
        // Checks that append maintains sorted order through binary_search insertion.
        let mut st = empty_statement();
        st.meta = meta("TestSource", "acc-1", 123);

        st.append(op_new(
            2,
            TransactionType::Deposit,
            1,
            1,
            10,
            100,
            Status::Success,
            None,
        ));
        st.append(op_new(
            1,
            TransactionType::Deposit,
            1,
            1,
            10,
            100,
            Status::Success,
            None,
        ));
        st.append(op_new(
            3,
            TransactionType::Deposit,
            1,
            1,
            10,
            100,
            Status::Success,
            None,
        ));

        let keys = collect_keys_via_write_to(&mut st);
        assert_eq!(keys.len(), 3);
        assert_sorted_keys(&keys);
        assert!(keys[0].starts_with("1|"));
        assert!(keys[1].starts_with("2|"));
        assert!(keys[2].starts_with("3|"));
    }

    #[test]
    fn statement_append_orders_by_full_key_not_only_tx_id() {
        // Checks that ordering depends on the full Operation key tuple.
        let mut st = empty_statement();

        st.append(op_new(
            1,
            TransactionType::Transfer,
            2,
            1,
            5,
            100,
            Status::Success,
            None,
        ));
        st.append(op_new(
            1,
            TransactionType::Deposit,
            2,
            1,
            5,
            100,
            Status::Success,
            None,
        ));
        st.append(op_new(
            1,
            TransactionType::Withdrawal,
            1,
            1,
            5,
            100,
            Status::Success,
            None,
        ));

        let keys = collect_keys_via_write_to(&mut st);
        assert_eq!(keys.len(), 3);
        assert_sorted_keys(&keys);

        // Expected order by enum declaration: Deposit < Transfer < Withdrawal.
        assert!(keys[0].contains("|Deposit|"));
        assert!(keys[1].contains("|Transfer|"));
        assert!(keys[2].contains("|Withdrawal|"));
    }

    #[test]
    fn statement_append_allows_duplicates_by_key() {
        // Checks that two operations equal by key can coexist and remain adjacent.
        let mut st = empty_statement();

        st.append(op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            Some("a"),
        ));
        st.append(op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            Some("b"),
        ));

        let keys = collect_keys_via_write_to(&mut st);
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], keys[1]);
        assert_sorted_keys(&keys);
    }

    #[test]
    fn extend_and_sort_sorts_once_and_keeps_duplicates() {
        // Checks that extend_and_sort sorts combined data and keeps duplicates by key.
        let mut st = empty_statement();
        st.append(op_new(
            5,
            TransactionType::Deposit,
            1,
            1,
            10,
            100,
            Status::Success,
            None,
        ));

        let batch = vec![
            op_new(
                3,
                TransactionType::Deposit,
                1,
                1,
                10,
                100,
                Status::Success,
                None,
            ),
            op_new(
                1,
                TransactionType::Deposit,
                1,
                1,
                10,
                100,
                Status::Success,
                None,
            ),
            op_new(
                4,
                TransactionType::Deposit,
                1,
                1,
                10,
                100,
                Status::Success,
                None,
            ),
            op_new(
                2,
                TransactionType::Deposit,
                1,
                1,
                10,
                100,
                Status::Success,
                None,
            ),
            op_new(
                2,
                TransactionType::Deposit,
                1,
                1,
                10,
                100,
                Status::Success,
                Some("dup with different description"),
            ),
        ];

        st.extend_and_sort(batch);

        let keys = collect_keys_via_write_to(&mut st);
        assert_eq!(keys.len(), 6);
        assert_sorted_keys(&keys);

        let two_count = keys.iter().filter(|k| k.starts_with("2|")).count();
        assert_eq!(two_count, 2);
        assert!(keys[0].starts_with("1|"));
    }

    // --- Statement: Eq -------------------------------------------------------

    #[test]
    fn statement_eq_true_when_operations_identical_and_meta_ignored() {
        // Checks that Statement equality is based on operations only, meta is ignored.
        let ops1 = vec![
            op_new(
                1,
                TransactionType::Deposit,
                1,
                2,
                10,
                100,
                Status::Success,
                None,
            ),
            op_new(
                2,
                TransactionType::Transfer,
                2,
                3,
                20,
                110,
                Status::Failure,
                Some("x"),
            ),
            op_new(
                3,
                TransactionType::Withdrawal,
                3,
                0,
                30,
                120,
                Status::Pending,
                Some("y"),
            ),
        ];
        let ops2 = vec![
            op_new(
                1,
                TransactionType::Deposit,
                1,
                2,
                10,
                100,
                Status::Success,
                None,
            ),
            op_new(
                2,
                TransactionType::Transfer,
                2,
                3,
                20,
                110,
                Status::Failure,
                Some("x"),
            ),
            op_new(
                3,
                TransactionType::Withdrawal,
                3,
                0,
                30,
                120,
                Status::Pending,
                Some("y"),
            ),
        ];

        let a = statement_from_ops(ops1, meta("A", "acc-1", 111));
        let b = statement_from_ops(ops2, meta("B", "acc-2", 222));

        assert_eq!(a, b);
    }

    #[test]
    fn statement_eq_false_when_one_operation_differs_by_key() {
        // Checks that a single key difference makes statements unequal.
        let ops1 = vec![op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            10,
            100,
            Status::Success,
            None,
        )];
        let ops2 = vec![op_new(
            1,
            TransactionType::Deposit,
            1,
            2,
            11,
            100,
            Status::Success,
            None,
        )];

        let a = statement_from_ops(ops1, StatementMeta::default());
        let b = statement_from_ops(ops2, StatementMeta::default());

        assert_ne!(a, b);
    }

    // --- Statement: from_read / write_to abstractions ------------------------

    #[test]
    fn from_read_empty_when_next_op_returns_none() {
        // Checks that from_read produces an empty statement when next_op returns None immediately.
        let mut r: &[u8] = b"irrelevant";
        let mut st = Statement::from_read(&mut r, |_| Ok(None)).unwrap();

        let mut out: Vec<u8> = Vec::new();
        let mut calls = 0usize;
        st.write_to(&mut out, |_, _| {
            calls += 1;
            Ok(())
        })
        .unwrap();

        assert_eq!(calls, 0);
    }

    #[test]
    fn from_read_propagates_error_from_next_op() {
        // Checks that from_read propagates errors produced by the next_op strategy.
        let mut r: &[u8] = b"x";

        let err = Statement::from_read(&mut r, |_| -> Result<Option<Operation>, DataError> {
            Err(DataError::BadLine {
                line_no: 1,
                line: "x".to_string(),
                msg: "boom".to_string(),
            })
        })
        .unwrap_err();

        match err {
            DataError::BadLine { line_no, .. } => assert_eq!(line_no, 1),
            _ => panic!("Expected AppError::BadLine"),
        }
    }

    #[test]
    fn write_to_calls_writer_for_each_operation_in_sorted_order() {
        // Checks that write_to calls the writer for each operation and in sorted order.
        let ops = vec![
            op_new(
                3,
                TransactionType::Deposit,
                1,
                1,
                10,
                100,
                Status::Success,
                None,
            ),
            op_new(
                1,
                TransactionType::Deposit,
                1,
                1,
                10,
                100,
                Status::Success,
                None,
            ),
            op_new(
                2,
                TransactionType::Deposit,
                1,
                1,
                10,
                100,
                Status::Success,
                None,
            ),
        ];
        let mut st = statement_from_ops(ops, StatementMeta::default());

        let keys = collect_keys_via_write_to(&mut st);
        assert_eq!(keys.len(), 3);
        assert_sorted_keys(&keys);
        assert!(keys[0].starts_with("1|"));
        assert!(keys[1].starts_with("2|"));
        assert!(keys[2].starts_with("3|"));
    }

    #[test]
    fn write_to_propagates_io_error_via_writer_strategy() {
        // Checks that write_to propagates IO errors surfaced by the writer strategy.
        struct FailWriter;
        impl io::Write for FailWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::Other, "fail"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let ops = vec![op_new(
            1,
            TransactionType::Deposit,
            1,
            1,
            10,
            100,
            Status::Success,
            None,
        )];
        let mut st = statement_from_ops(ops, StatementMeta::default());

        let mut w = FailWriter;
        let err = st
            .write_to(&mut w, |w, op| {
                use std::io::Write as _;
                writeln!(w, "{}", op.tx_id()).map_err(DataError::Io)?;
                Ok(())
            })
            .unwrap_err();

        match err {
            DataError::Io(_) => {}
            _ => panic!("Expected AppError::Io"),
        }
    }

    #[test]
    fn round_trip_using_test_format_read_and_write_strategies() {
        // Checks that the statement can be written and then read back using matching strategies.
        let ops = vec![
            op_new(
                2,
                TransactionType::Transfer,
                2,
                3,
                20,
                110,
                Status::Failure,
                Some("x"),
            ),
            op_new(
                1,
                TransactionType::Deposit,
                1,
                2,
                10,
                100,
                Status::Success,
                None,
            ),
            op_new(
                3,
                TransactionType::Withdrawal,
                3,
                0,
                30,
                120,
                Status::Pending,
                Some("y"),
            ),
        ];
        let mut st = statement_from_ops(ops, StatementMeta::default());

        // Writer: one operation per line, pipe-separated; description "-" means None.
        let mut bytes: Vec<u8> = Vec::new();
        st.write_to(&mut bytes, |w, op| {
            let t = match op.tx_type() {
                TransactionType::Deposit => "D",
                TransactionType::Transfer => "T",
                TransactionType::Withdrawal => "W",
            };
            let s = match op.status() {
                Status::Success => "S",
                Status::Failure => "F",
                Status::Pending => "P",
            };
            let d = op.description().unwrap_or("-");
            use std::io::Write as _;
            writeln!(
                w,
                "{}|{}|{}|{}|{}|{}|{}|{}",
                op.tx_id(),
                t,
                op.from_user_id(),
                op.to_user_id(),
                op.amount(),
                op.timestamp_ms(),
                s,
                d
            )
            .map_err(DataError::Io)?;
            Ok(())
        })
        .unwrap();

        // Reader: reads one line and parses it into Operation.
        let mut input: &[u8] = &bytes;
        let mut buf = io::BufReader::new(&mut input);

        let st2 = Statement::from_read(&mut buf, |r| {
            let mut line = String::new();
            let n = io::BufRead::read_line(r, &mut line).map_err(DataError::Io)?;
            if n == 0 {
                return Ok(None);
            }
            let line = line.trim_end_matches(['\r', '\n']);
            if line.trim().is_empty() {
                return Ok(None);
            }

            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() != 8 {
                return Err(DataError::BadLine {
                    line_no: 0,
                    line: line.to_string(),
                    msg: format!("Expected 8 fields, got {}", parts.len()),
                });
            }

            let tx_id: u64 = parts[0].parse().map_err(|_| DataError::BadLine {
                line_no: 0,
                line: line.to_string(),
                msg: "Bad tx_id".to_string(),
            })?;

            let tx_type = match parts[1] {
                "D" => TransactionType::Deposit,
                "T" => TransactionType::Transfer,
                "W" => TransactionType::Withdrawal,
                _ => {
                    return Err(DataError::BadLine {
                        line_no: 0,
                        line: line.to_string(),
                        msg: "Bad tx_type".to_string(),
                    });
                }
            };

            let from_user_id: u64 = parts[2].parse().map_err(|_| DataError::BadLine {
                line_no: 0,
                line: line.to_string(),
                msg: "Bad from_user_id".to_string(),
            })?;
            let to_user_id: u64 = parts[3].parse().map_err(|_| DataError::BadLine {
                line_no: 0,
                line: line.to_string(),
                msg: "Bad to_user_id".to_string(),
            })?;
            let amount: i64 = parts[4].parse().map_err(|_| DataError::BadLine {
                line_no: 0,
                line: line.to_string(),
                msg: "Bad amount".to_string(),
            })?;
            let timestamp_ms: u64 = parts[5].parse().map_err(|_| DataError::BadLine {
                line_no: 0,
                line: line.to_string(),
                msg: "Bad timestamp_ms".to_string(),
            })?;
            let status = match parts[6] {
                "S" => Status::Success,
                "F" => Status::Failure,
                "P" => Status::Pending,
                _ => {
                    return Err(DataError::BadLine {
                        line_no: 0,
                        line: line.to_string(),
                        msg: "Bad status".to_string(),
                    });
                }
            };
            let desc = if parts[7] == "-" {
                None
            } else {
                Some(parts[7].to_string())
            };

            Ok(Some(Operation::new(
                tx_id,
                tx_type,
                from_user_id,
                to_user_id,
                amount,
                timestamp_ms,
                status,
                desc,
            )))
        })
        .unwrap();

        let expected = statement_from_ops(
            vec![
                op_new(
                    1,
                    TransactionType::Deposit,
                    1,
                    2,
                    10,
                    100,
                    Status::Success,
                    None,
                ),
                op_new(
                    2,
                    TransactionType::Transfer,
                    2,
                    3,
                    20,
                    110,
                    Status::Failure,
                    Some("x"),
                ),
                op_new(
                    3,
                    TransactionType::Withdrawal,
                    3,
                    0,
                    30,
                    120,
                    Status::Pending,
                    Some("y"),
                ),
            ],
            StatementMeta::default(),
        );

        assert_eq!(st2, expected);
    }

    // --- Nomenclature: AccountStatementType ---------------------------------

    #[test]
    fn account_statement_type_variants_exist() {
        // Checks that all statement type variants exist and are publicly accessible.
        let _ = AccountStatementType::YPBank;
        let _ = AccountStatementType::MT940;
        let _ = AccountStatementType::Camt053;
        let _ = AccountStatementType::Sber;
    }
}
