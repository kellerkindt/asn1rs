use postgres::rows::Rows;
use backtrace::Backtrace;

pub use postgres::Error as PostgresError;
pub use postgres::transaction::Transaction;


#[derive(Debug)]
pub enum Error {
    Postgres(Backtrace, PostgresError),
    MissingReturnedIndex(Backtrace)
}

impl Error {
    pub fn expect_returned_index(rows: Rows) -> Result<i32, Error> {
        if rows.is_empty() {
            Err(Error::MissingReturnedIndex(Backtrace::new()))
        } else {
            let row = rows.get(0);
            if row.is_empty() {
                Err(Error::MissingReturnedIndex(Backtrace::new()))
            } else {
                if let Some(value) = row.get_opt(0) {
                    Ok(value?)
                } else {
                    Err(Error::MissingReturnedIndex(Backtrace::new()))
                }
            }
        }
    }
}

impl From<PostgresError> for Error {
    fn from(e: PostgresError) -> Self {
        Error::Postgres(Backtrace::new(), e)
    }
}

pub trait PsqlInsertable {
    fn insert_statement() -> &'static str;
    fn insert_with(&self, transaction: &Transaction) -> Result<i32, Error>;
}