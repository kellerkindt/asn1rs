use backtrace::Backtrace;
pub use postgres::row::Row;
pub use postgres::Error as PostgresError;
pub use postgres::Transaction;
use std::fmt::{Display, Formatter};

pub mod bit_vec_impl;

#[derive(Debug)]
pub enum Error {
    Postgres(Backtrace, PostgresError),
    MissingReturnedIndex(Backtrace),
    MissingRow(usize, Backtrace),
    MissingColumn(usize, Backtrace),
    NoResult(Backtrace),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

impl Error {
    #[inline]
    pub fn no_result() -> Self {
        Error::NoResult(Backtrace::new())
    }

    pub fn expect_returned_index(row: &Row) -> Result<i32, Error> {
        Ok(row.try_get::<_, i32>(0)?)
    }

    pub fn expect_returned_index_in_rows(rows: &[Row]) -> Result<i32, Error> {
        if rows.is_empty() {
            Err(Error::MissingReturnedIndex(Backtrace::new()))
        } else if let Some(row) = rows.get(0) {
            Self::expect_returned_index(row)
        } else {
            Err(Error::MissingReturnedIndex(Backtrace::new()))
        }
    }

    pub fn value_at_column<'a, T: postgres::types::FromSql<'a>>(
        row: &'a Row,
        column: usize,
    ) -> Result<T, Error> {
        Ok(row.try_get::<'a>(column)?)
    }

    pub fn value_at<'a, T: postgres::types::FromSql<'a>>(
        rows: &'a [Row],
        row: usize,
        column: usize,
    ) -> Result<T, Error> {
        if rows.is_empty() || rows.len() <= row {
            Err(Error::MissingRow(row, Backtrace::new()))
        } else if let Some(r) = rows.get(row) {
            Ok(r.try_get::<'a>(column)?)
        } else {
            Err(Error::MissingColumn(column, Backtrace::new()))
        }
    }

    pub fn first_present(row: &Row, columns: &[usize]) -> Result<usize, Error> {
        for column in columns {
            if row.try_get::<_, &[u8]>(column).is_ok() {
                return Ok(*column);
            }
        }
        Err(Error::no_result())
    }

    pub fn first_not_null<'a, T: postgres::types::FromSql<'a>>(
        row: &'a Row,
        columns: &[usize],
    ) -> Result<(usize, T), Error> {
        for column in columns {
            match row.try_get::<'a, _, Option<T>>(*column) {
                Ok(Some::<T>(value)) => return Ok((*column, value)),
                Ok(None::<T>) => {} // null in db, ignore
                Err(e) => return Err(Error::from(e)),
            };
        }
        Err(Error::no_result())
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
    fn insert_with(&self, transaction: &mut Transaction) -> Result<i32, Error>;
}

pub trait Queryable: Representable {
    fn query_statement() -> &'static str;
    fn query_with(transaction: &mut Transaction, id: i32) -> Result<Self, Error>
    where
        Self: Sized;
    fn load_from(transaction: &mut Transaction, row: &Row) -> Result<Self, Error>
    where
        Self: Sized;
}
