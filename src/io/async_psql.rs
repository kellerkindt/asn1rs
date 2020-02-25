use futures::lock::Mutex;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::future::Future;
use std::sync::Arc;
use tokio_postgres::Statement;
use tokio_postgres::Transaction;

pub use futures::future::join_all;
pub use futures::future::try_join_all;
pub use tokio::join;
pub use tokio::try_join;
pub use tokio_postgres::Error as PsqlError;
pub use tokio_postgres::Row;

#[derive(Clone)]
enum StatementState {
    Awaiting(Arc<Mutex<()>>),
    Ready(Statement),
}

pub struct Context<'a> {
    cache: Cache,
    transaction: Transaction<'a>,
}

impl<'i> Context<'i> {
    pub fn new(transaction: Transaction<'i>) -> Self {
        Self {
            cache: Default::default(),
            transaction,
        }
    }

    pub fn optimize_cache(&mut self) {
        self.cache.optimize();
    }

    pub async fn prepared(&self, statement_str: &'static str) -> Result<Statement, PsqlError> {
        loop {
            if let Some(statement) = self.cache.fast.get(statement_str) {
                return Ok(statement.clone());
            }

            let mut slow_locked = self.cache.slow.lock().await;
            if let Some(statement) = slow_locked.get(statement_str) {
                match statement {
                    StatementState::Awaiting(mutex) => {
                        let local = mutex.clone();
                        drop(slow_locked); // allow others to access the map
                        drop(local.lock().await); // await the lock to be released
                        continue; // this should now result in StatementState::Ready(_)
                    }
                    StatementState::Ready(statement) => return Ok(statement.clone()),
                };
            } else {
                let mutex = Arc::new(Mutex::new(()));
                let lock = mutex.lock().await; // this will complete immediately

                // insert placeholder so that further calls of prepared() for the same statement_str
                // will not prepare further Statements for the same content
                slow_locked.insert(statement_str, StatementState::Awaiting(mutex.clone()));
                drop(slow_locked); // allow others to access the map

                // Actually prepare the statement. Because this is awaiting the response,
                // it is possible for another future on the same thread to access the cache
                // in the meantime. Because of the placeholder Mutex, the other future will
                // not prepare a second/third/... Statement with the same content
                let statement = self.transaction.prepare(statement_str).await?;

                // Cache the received statement so that any following call to the cache will
                // have access to the result immediately
                self.cache
                    .slow
                    .lock()
                    .await
                    .insert(statement_str, StatementState::Ready(statement.clone()));

                drop(lock); // this will "notify" all waiting futures
                return Ok(statement);
            }
        }
    }

    pub const fn transaction(&self) -> &Transaction<'i> {
        &self.transaction
    }

    pub async fn batch<
        'a,
        'b: 'a,
        'c: 'a + 'b,
        's: 'a + 'b + 'c,
        R,
        E,
        F: Future<Output = Result<R, E>> + 'b,
        V: 'static,
        M: Fn(&'b V, &'a Context<'c>) -> F,
    >(
        &'s self,
        values: impl Iterator<Item = &'b V>,
        mapper: M,
    ) -> Vec<Result<R, E>> {
        futures::future::join_all(values.map(|v| mapper(v, self))).await
    }

    pub async fn try_batch<
        'a,
        'b: 'a,
        'c: 'a + 'b,
        's: 'a + 'b + 'c,
        R,
        E,
        F: Future<Output = Result<R, E>> + 'b,
        V: 'static,
        M: Fn(&'b V, &'a Context<'c>) -> F,
    >(
        &'s self,
        values: impl Iterator<Item = &'b V>,
        mapper: M,
    ) -> Result<Vec<R>, E> {
        futures::future::try_join_all(values.map(|v| mapper(v, self))).await
    }

    pub fn split(self) -> (Cache, Transaction<'i>) {
        (self.cache, self.transaction)
    }
}

#[derive(Default)]
pub struct Cache {
    fast: HashMap<&'static str, Statement>,
    slow: Mutex<HashMap<&'static str, StatementState>>,
}

impl Cache {
    pub fn into_context(self, transaction: Transaction) -> Context {
        Context {
            transaction,
            cache: self,
        }
    }

    pub fn fast(&self) -> impl Iterator<Item = (&&'static str, &Statement)> {
        self.fast.iter()
    }

    pub fn optimize(&mut self) {
        // Removes all key-value pairs. Keeps the allocated memory for re-use
        self.fast.extend(
            self.slow
                .get_mut()
                .drain()
                .flat_map(|(key, value)| match value {
                    StatementState::Ready(statement) => Some((key, statement)),
                    StatementState::Awaiting(_) => None, // failed, so drop it
                }),
        );
    }
}

#[derive(Debug)]
pub enum Error {
    Psql(PsqlError),
    UnexpectedVariant(usize),
    NoEntryFoundForId(i32),
    RowUnloadable,
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        #[allow(clippy::match_same_arms)]
        match self {
            Error::Psql(psql) => psql.source(),
            Error::UnexpectedVariant(_) => None,
            Error::NoEntryFoundForId(_) => None,
            Error::RowUnloadable => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Psql(psql) => psql.fmt(f),
            Error::UnexpectedVariant(index) => write!(f, "Unexpected variant index: {}", index),
            Error::NoEntryFoundForId(id) => write!(f, "Id {} is unknown", id),
            Error::RowUnloadable => write!(f, "The row has an error and cannot be loaded"),
        }
    }
}

impl From<PsqlError> for Error {
    fn from(psql: PsqlError) -> Self {
        Error::Psql(psql)
    }
}
