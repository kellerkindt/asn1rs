use postgres::rows::Row;
use postgres::rows::Rows;

pub use postgres::Error as PostgresError;
pub use postgres::transaction::Transaction;


#[derive(Debug)]
pub enum Error {
    Postgres(PostgresError),
    MissingReturnedIndex
}

impl Error {
    pub fn expect_returned_index(rows: Rows) -> Result<i32, Error> {
        if rows.is_empty() {
            Err(Error::MissingReturnedIndex)
        } else {
            let row = rows.get(0);
            if row.is_empty() {
                Err(Error::MissingReturnedIndex)
            } else {
                if let Some(value) = row.get_opt(0) {
                    Ok(value?)
                } else {
                    Err(Error::MissingReturnedIndex)
                }
            }
        }
    }
}

impl From<PostgresError> for Error {
    fn from(e: PostgresError) -> Self {
        Error::Postgres(e)
    }
}

pub trait PsqlInsertable {
    fn insert_statement() -> &'static str;
    fn insert_with(&self, transaction: &Transaction) -> Result<i32, Error>;
}