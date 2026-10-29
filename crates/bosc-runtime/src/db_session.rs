use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex as StdMutex};

use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use tokio::sync::{Mutex, OwnedMappedMutexGuard, OwnedMutexGuard};

use crate::{RuntimeError, SessionContext};

type TxSlot = Option<Transaction<'static, Postgres>>;

/// One PostgreSQL transaction shared by every statement of a single unit of work
/// (an API write, a codeunit run, a test method, an outbox event).
///
/// Triggers, event subscribers, nested codeunit calls and script host functions all
/// receive a clone of the same session, so the whole operation commits or rolls back
/// as one. The first database error poisons the session: PostgreSQL has already
/// aborted the transaction, so `commit` rolls back and reports that root cause even if
/// a script caught the exception and carried on.
#[derive(Clone)]
pub struct DbSession {
    tx: Arc<Mutex<TxSlot>>,
    pool: PgPool,
    failure: Arc<StdMutex<Option<String>>>,
}

/// Exclusive access to the session connection. Hold it only for the statements being
/// executed: triggers and nested codeunits need to acquire it themselves.
pub struct SessionConn(OwnedMappedMutexGuard<TxSlot, Transaction<'static, Postgres>>);

impl Deref for SessionConn {
    type Target = PgConnection;
    fn deref(&self) -> &PgConnection {
        &self.0
    }
}

impl DerefMut for SessionConn {
    fn deref_mut(&mut self) -> &mut PgConnection {
        &mut self.0
    }
}

impl DbSession {
    pub async fn begin(pool: &PgPool) -> Result<Self, RuntimeError> {
        let tx = pool.begin().await?;
        Ok(Self {
            tx: Arc::new(Mutex::new(Some(tx))),
            pool: pool.clone(),
            failure: Arc::new(StdMutex::new(None)),
        })
    }

    /// Begin a session and publish the caller identity as transaction-local settings
    /// (`bos.current_tenant_id`, `bos.current_company_id`, `bos.current_user_id`) for RLS policies.
    pub async fn begin_with_context(
        pool: &PgPool,
        ctx: &SessionContext,
    ) -> Result<Self, RuntimeError> {
        let session = Self::begin(pool).await?;
        session.apply_identity(ctx).await?;
        Ok(session)
    }

    pub async fn apply_identity(&self, ctx: &SessionContext) -> Result<(), RuntimeError> {
        let mut conn = self.conn().await?;
        let tenant_s = if ctx.tenant_id.is_nil() {
            String::new()
        } else {
            ctx.tenant_id.to_string()
        };
        let company_s = if ctx.company_id.is_nil() {
            String::new()
        } else {
            ctx.company_id.to_string()
        };
        let user_s = if ctx.user_id.is_nil() {
            String::new()
        } else {
            ctx.user_id.to_string()
        };
        let res = sqlx::query(
            "SELECT set_config('bos.current_tenant_id', $1, true), set_config('bos.current_company_id', $2, true), set_config('bos.current_user_id', $3, true)",
        )
        .bind(tenant_s)
        .bind(company_s)
        .bind(user_s)
        .execute(&mut *conn)
        .await;
        self.track(res).map(|_| ())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Acquire the session connection. Fails fast with the original root cause once the
    /// session has been poisoned by an earlier database error.
    pub async fn conn(&self) -> Result<SessionConn, RuntimeError> {
        if let Some(msg) = self.failure() {
            return Err(RuntimeError::TransactionAborted(msg));
        }
        self.raw_conn().await
    }

    async fn raw_conn(&self) -> Result<SessionConn, RuntimeError> {
        let guard = self.tx.clone().lock_owned().await;
        OwnedMutexGuard::try_map(guard, |slot| slot.as_mut())
            .map(SessionConn)
            .map_err(|_| {
                RuntimeError::Internal(
                    "Database session has already been committed or rolled back".to_string(),
                )
            })
    }

    /// Convert a sqlx result into a runtime result, poisoning the session on error.
    pub fn track<T>(&self, res: Result<T, sqlx::Error>) -> Result<T, RuntimeError> {
        res.map_err(|e| {
            self.mark_failed(e.to_string());
            RuntimeError::DatabaseError(e)
        })
    }

    pub fn mark_failed(&self, msg: impl Into<String>) {
        let mut slot = self.failure.lock().unwrap_or_else(|p| p.into_inner());
        if slot.is_none() {
            *slot = Some(msg.into());
        }
    }

    pub fn failure(&self) -> Option<String> {
        self.failure
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    fn clear_failure(&self) {
        *self.failure.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }

    pub async fn commit(&self) -> Result<(), RuntimeError> {
        let tx = self.tx.lock().await.take();
        let Some(tx) = tx else {
            return Err(RuntimeError::Internal(
                "Database session has already been committed or rolled back".to_string(),
            ));
        };
        if let Some(msg) = self.failure() {
            tx.rollback().await?;
            return Err(RuntimeError::TransactionAborted(msg));
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn rollback(&self) -> Result<(), RuntimeError> {
        if let Some(tx) = self.tx.lock().await.take() {
            tx.rollback().await?;
        }
        Ok(())
    }

    /// Commit when `outcome` succeeded, otherwise roll back and return the original error.
    pub async fn finish<T>(&self, outcome: Result<T, RuntimeError>) -> Result<T, RuntimeError> {
        match outcome {
            Ok(value) => {
                self.commit().await?;
                Ok(value)
            }
            Err(e) => {
                if let Err(rb) = self.rollback().await {
                    tracing::error!(target: "bos::tx", "Rollback failed after error '{}': {}", e, rb);
                }
                Err(e)
            }
        }
    }

    pub async fn savepoint(&self, name: &str) -> Result<(), RuntimeError> {
        let ident = crate::sanitize_identifier(name)?;
        let mut conn = self.conn().await?;
        let res = sqlx::query(&format!("SAVEPOINT \"{}\"", ident))
            .execute(&mut *conn)
            .await;
        self.track(res).map(|_| ())
    }

    pub async fn release_savepoint(&self, name: &str) -> Result<(), RuntimeError> {
        let ident = crate::sanitize_identifier(name)?;
        let mut conn = self.conn().await?;
        let res = sqlx::query(&format!("RELEASE SAVEPOINT \"{}\"", ident))
            .execute(&mut *conn)
            .await;
        self.track(res).map(|_| ())
    }

    /// Undo everything since `savepoint(name)` and make the session usable again.
    pub async fn rollback_to_savepoint(&self, name: &str) -> Result<(), RuntimeError> {
        let ident = crate::sanitize_identifier(name)?;
        let mut conn = self.raw_conn().await?;
        sqlx::query(&format!("ROLLBACK TO SAVEPOINT \"{}\"", ident))
            .execute(&mut *conn)
            .await?;
        drop(conn);
        self.clear_failure();
        Ok(())
    }
}

/// Drive an async database call to completion from synchronous code (QuickJS host
/// functions) running on a Tokio multi-thread worker or blocking thread.
pub fn run_blocking<F: Future>(fut: F) -> Result<F::Output, RuntimeError> {
    let handle = tokio::runtime::Handle::try_current().map_err(|_| {
        RuntimeError::Internal("No Tokio runtime available for database access".to_string())
    })?;
    Ok(tokio::task::block_in_place(|| handle.block_on(fut)))
}
