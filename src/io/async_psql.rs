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

/// Combines a [`Cache`] and a [`Transaction`] with useful methods
/// to cache re-occurring statements efficiently.
///
/// [`Cache`]: Cache
/// [`Transaction`]: Transaction
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

    /// This function will try to retrieve a prepared [`Statement`] from the [`Cache`].
    /// In doing so, it will first try to retrieve the prepared [`Statement`] from the
    /// fast lookup map of the [`Cache`]. If this does not provide a result, the `slow`
    /// lookup map will be engaged. This will either yield
    ///  - no result again. In this case, first an [`Awaiting`] marker will be stored
    ///    so that further calls to this method for the same `statement_str` will not issue
    ///    further prepare requests on the backend. Then, the backend will be requested to
    ///    create a new prepared statement. The result will be stored in the lookup map as
    ///    [`Ready`] and all waiting calls for the same `statement_str` will be notified.  
    ///  - an [`Awaiting`] marker. In this case, the notification will be awaited. Once notified,
    ///    the lookup map will be re-engaged, hopefully resulting in:
    ///  - a [`Ready`] marker. A clone of the prepared [`Statement`] will be returned.
    ///
    /// [`Statement`]: Statement
    /// [`Cache`]: Cache
    /// [`Awaiting`]: StatementState::Awaiting
    /// [`Ready`]: StatementState::Ready
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

    /// Disassembles this [`Context`] and returns the underlying
    /// [`Cache`] and [`Transaction`]. The [`Cache`] will be optimized
    /// before returning, if you do not wish for that, see [`split_unoptimized`]
    /// instead.
    ///
    /// [`Context`]: Context
    /// [`Cache`]: Cache
    /// [`Transaction`]: Transaction
    /// [`split_unoptimized`]: Context::split_unoptimized
    pub fn split(mut self) -> (Cache, Transaction<'i>) {
        self.optimize_cache();
        self.split_unoptimized()
    }

    /// Disassembles this [`Context`] and returns the underlying
    /// [`Cache`] and [`Transaction`]. The [`Cache`] will not be optimized
    /// before returning. See also [`split`] for automatically optimizing the
    /// [`Cache`] before returning.
    ///
    /// [`Context`]: Context
    /// [`Cache`]: Cache
    /// [`Transaction`]: Transaction
    /// [`split`]: Context::split
    pub fn split_unoptimized(self) -> (Cache, Transaction<'i>) {
        (self.cache, self.transaction)
    }

    /// Optimizes the [`Cache`] without disassembling
    ///
    /// [`Cache`]: Cache
    pub fn optimize_cache(&mut self) {
        self.cache.optimize();
    }
}

/// A cache for prepared statements. It has two lookup maps:
///  - one `fast` map, which is not protected by a Mutex and allows
///    very fast concurrent read access
///  - one `slow` map, which is protected by a Mutex and allows the
///    cache to grow as it is being used.
/// In regular intervals (for example once a [`Transaction`] is
/// going to be submitted) the cache should be optimized. Optimizing
/// the cache will require exclusive access to it and and it will move
/// all prepared statements from the `slow` to the `fast` lookup map.
/// This allows a further [`Context`] to access all currently known
/// prepared statements without locking.
///
/// The idea is, that in a system with re-occurring statements, the [`Cache`]
/// is warming-up once, and then retrieves these prepared statements very fast.
///
/// [`Transaction`]: Transaction
/// [`Context`]: Context
/// [`Cache`]: Cache
#[derive(Default)]
pub struct Cache {
    fast: HashMap<&'static str, Statement>,
    slow: Mutex<HashMap<&'static str, StatementState>>,
}

impl Cache {
    /// Creates a new [`Context`] by using the given [`Transaction`] and this instance of [`Cache`].
    ///
    /// [`Context`]: Context
    /// [`Transaction`]: Transaction
    /// [`Cache`]: Cache
    pub fn into_context(self, transaction: Transaction) -> Context {
        Context {
            transaction,
            cache: self,
        }
    }

    /// The fast but read-only lookup map
    pub fn fast(&self) -> &HashMap<&'static str, Statement> {
        &self.fast
    }

    // The slow but read+write lookup map
    pub fn slow(&self) -> &Mutex<HashMap<&'static str, StatementState>> {
        &self.slow
    }

    /// This moves all new prepared statements to the read-only but fast
    /// lookup map.
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
