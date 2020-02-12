use futures::lock::Mutex;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio_postgres::Statement;
use tokio_postgres::Transaction;

pub use futures::future::join_all;
pub use futures::future::try_join_all;
pub use tokio::join;
pub use tokio::try_join;
pub use tokio_postgres::Error;

#[derive(Clone)]
enum StatementState {
    Awaiting(Arc<Mutex<()>>),
    Ready(Statement),
}

pub struct Context<'a> {
    client: Transaction<'a>,
    fast: HashMap<&'static str, Statement>,
    slow: Mutex<HashMap<&'static str, StatementState>>,
}

impl<'i> Context<'i> {
    pub fn new(transaction: Transaction<'i>) -> Self {
        Self {
            client: transaction,
            fast: Default::default(),
            slow: Default::default(),
        }
    }

    pub fn replace_transaction(&mut self, transaction: Transaction<'i>) -> Transaction<'i> {
        std::mem::replace(&mut self.client, transaction)
    }

    pub fn optimize_cache(&mut self) {
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

    pub async fn prepared(&self, statement_str: &'static str) -> Result<Statement, Error> {
        loop {
            if let Some(statement) = self.fast.get(statement_str) {
                return Ok(statement.clone());
            }

            let mut slow_locked = self.slow.lock().await;
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
                let statement = self.client.prepare(statement_str).await?;

                // Cache the received statement so that any following call to the cache will
                // have access to the result immediately
                self.slow
                    .lock()
                    .await
                    .insert(statement_str, StatementState::Ready(statement.clone()));

                drop(lock); // this will "notify" all waiting futures
                return Ok(statement);
            }
        }
    }

    pub fn transaction(&self) -> &Transaction<'i> {
        &self.client
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
}
