use backtrace::Backtrace;
use postgres::rows::Rows;

pub use postgres::rows::Row;
pub use postgres::transaction::Transaction;
pub use postgres::Error as PostgresError;

#[derive(Debug)]
pub enum Error {
    Postgres(Backtrace, PostgresError),
    MissingReturnedIndex(Backtrace),
    MissingRow(usize, Backtrace),
    MissingColumn(usize, Backtrace),
    NoResult,
}

impl Error {
    pub fn expect_returned_index(rows: &Rows) -> Result<i32, Error> {
        if rows.is_empty() {
            Err(Error::MissingReturnedIndex(Backtrace::new()))
        } else {
            let row = rows.get(0);
            if let Some(value) = row.get_opt(0) {
                Ok(value?)
            } else {
                Err(Error::MissingReturnedIndex(Backtrace::new()))
            }
        }
    }

    pub fn value_at<T: postgres::types::FromSql>(
        rows: &Rows,
        row: usize,
        column: usize,
    ) -> Result<T, Error> {
        if rows.is_empty() || rows.len() <= row {
            Err(Error::MissingRow(row, Backtrace::new()))
        } else {
            let row = rows.get(row);
            if let Some(value) = row.get_opt(column) {
                Ok(value?)
            } else {
                Err(Error::MissingColumn(column, Backtrace::new()))
            }
        }
    }

    pub fn first_not_null<T: postgres::types::FromSql>(
        row: &Row,
        columns: &[usize],
    ) -> Result<(usize, T), Error> {
        for column in columns {
            match row.get_opt::<usize, Option<T>>(*column) {
                Some(Ok(Some::<T>(value))) => return Ok((*column, value)),
                Some(Ok(None::<T>)) => {} // null in db, ignore
                Some(Err(e)) => return Err(Error::from(e)),
                None => return Err(Error::MissingColumn(*column, Backtrace::new())),
            };
        }
        Err(Error::NoResult)
    }
}

impl From<PostgresError> for Error {
    fn from(e: PostgresError) -> Self {
        Error::Postgres(Backtrace::new(), e)
    }
}

pub trait Representable {
    fn table_name(&self) -> &'static str;
}

pub trait Insertable: Representable {
    fn insert_statement(&self) -> &'static str;
    fn insert_with(&self, transaction: &Transaction) -> Result<i32, Error>;
}

pub trait Queryable: Representable {
    fn query_statement() -> &'static str;
    fn query_with(transaction: &Transaction, id: i32) -> Result<Self, Error>
    where
        Self: Sized;
    fn load_from(transaction: &Transaction, row: &Row) -> Result<Self, Error>
    where
        Self: Sized;
}
