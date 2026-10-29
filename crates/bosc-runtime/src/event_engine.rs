use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

use crate::db_session::DbSession;
use crate::execution_engine::ExecScope;
use crate::quickjs_engine::{ExecutionLimits, QuickJsEngine, ScriptServices};
use crate::{RuntimeError, SessionContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscriber {
    pub id: Uuid,
    pub publisher: String,
    pub event_name: String,
    pub subscriber_name: String,
    pub method_name: String,
    pub priority: i32,
    pub script: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub company_id: Option<Uuid>,
    pub publisher: String,
    pub event_name: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub retry_count: i32,
    pub max_retries: i32,
    pub last_error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub processed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOutboxSummary {
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub remaining: usize,
}

#[derive(Clone)]
pub struct EventRegistry {
    subscribers: Arc<DashMap<String, Vec<EventSubscriber>>>,
}

impl Default for EventRegistry {
    fn default() -> Self {
        Self::new()
    }
}

static INITIALIZED_DBS: std::sync::LazyLock<dashmap::DashSet<String>> =
    std::sync::LazyLock::new(dashmap::DashSet::new);

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(DashMap::new()),
        }
    }

    fn key(publisher: &str, event_name: &str) -> String {
        format!(
            "{}:{}",
            publisher.trim().to_lowercase(),
            event_name.trim().to_lowercase()
        )
    }

    /// Register an in-memory subscriber, sorted by priority (ascending)
    pub fn register(&self, sub: EventSubscriber) {
        let k = Self::key(&sub.publisher, &sub.event_name);
        let mut list = self.subscribers.entry(k).or_default();
        list.retain(|s| s.id != sub.id);
        list.push(sub);
        list.sort_by_key(|s| s.priority);
    }

    /// Remove a subscriber by ID
    pub fn unregister(&self, id: Uuid) {
        for mut entry in self.subscribers.iter_mut() {
            entry.value_mut().retain(|s| s.id != id);
        }
    }

    /// Retrieve subscribers for a given publisher and event name in priority order
    pub fn get_subscribers(&self, publisher: &str, event_name: &str) -> Vec<EventSubscriber> {
        let k = Self::key(publisher, event_name);
        self.subscribers
            .get(&k)
            .map(|list| list.iter().filter(|s| s.is_active).cloned().collect())
            .unwrap_or_default()
    }

    /// List all registered subscribers across all publishers
    pub fn list_all(&self) -> Vec<EventSubscriber> {
        let mut all = Vec::new();
        for entry in self.subscribers.iter() {
            all.extend(entry.value().clone());
        }
        all.sort_by_key(|s| s.priority);
        all
    }

    /// Invalidate database initialization cache so DDL will re-run on next access
    pub fn invalidate_cache(db_name: &str) {
        INITIALIZED_DBS.remove(db_name);
    }

    /// Ensure the backing persistence table exists in PostgreSQL
    pub async fn init_table(pool: &PgPool) -> Result<(), RuntimeError> {
        let db_name = pool.connect_options().get_database().unwrap_or("default").to_string();
        if INITIALIZED_DBS.contains(&db_name) {
            return Ok(());
        }
        let ddl = r#"
            CREATE TABLE IF NOT EXISTS "_bos_event_subscribers" (
                "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                "publisher" VARCHAR(100) NOT NULL,
                "event_name" VARCHAR(100) NOT NULL,
                "subscriber_name" VARCHAR(100) NOT NULL,
                "method_name" VARCHAR(100) NOT NULL,
                "priority" INT NOT NULL DEFAULT 100,
                "script" TEXT NOT NULL,
                "is_active" BOOLEAN NOT NULL DEFAULT true,
                "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS "idx_bos_event_subscribers_pub_evt"
                ON "_bos_event_subscribers" ("publisher", "event_name");

            CREATE TABLE IF NOT EXISTS "_bos_event_outbox" (
                "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                "tenant_id" UUID NOT NULL,
                "company_id" UUID,
                "publisher" VARCHAR(100) NOT NULL,
                "event_name" VARCHAR(100) NOT NULL,
                "payload" JSONB NOT NULL DEFAULT '{}'::jsonb,
                "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                "retry_count" INT NOT NULL DEFAULT 0,
                "max_retries" INT NOT NULL DEFAULT 5,
                "last_error" TEXT,
                "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                "processed_at" TIMESTAMPTZ
            );
            CREATE INDEX IF NOT EXISTS "idx_bos_event_outbox_status_created"
                ON "_bos_event_outbox" ("status", "created_at");

            ALTER TABLE "_bos_event_outbox" ADD COLUMN IF NOT EXISTS "next_retry_at" TIMESTAMPTZ;
            CREATE INDEX IF NOT EXISTS "idx_bos_event_outbox_retry_poll"
                ON "_bos_event_outbox" ("status", "next_retry_at", "created_at");

            CREATE OR REPLACE FUNCTION notify_bos_event_outbox() RETURNS TRIGGER AS $$
            BEGIN
                PERFORM pg_notify('bos_event_outbox_channel', NEW.id::text);
                RETURN NEW;
            END;
            $$ LANGUAGE plpgsql;

            DROP TRIGGER IF EXISTS trg_bos_event_outbox ON "_bos_event_outbox";
            CREATE TRIGGER trg_bos_event_outbox
                AFTER INSERT ON "_bos_event_outbox"
                FOR EACH ROW EXECUTE FUNCTION notify_bos_event_outbox();
        "#;
        sqlx::raw_sql(ddl).execute(pool).await?;
        INITIALIZED_DBS.insert(db_name);
        Ok(())
    }

    /// Load subscribers from PostgreSQL database
    pub async fn load_from_db(pool: &PgPool) -> Result<Self, RuntimeError> {
        Self::init_table(pool).await?;
        let registry = Self::new();
        let rows = sqlx::query(
            "SELECT id, publisher, event_name, subscriber_name, method_name, priority, script, is_active FROM _bos_event_subscribers WHERE is_active = true ORDER BY priority ASC;"
        )
        .fetch_all(pool)
        .await?;

        for r in rows {
            let sub = EventSubscriber {
                id: r.try_get("id")?,
                publisher: r.try_get("publisher")?,
                event_name: r.try_get("event_name")?,
                subscriber_name: r.try_get("subscriber_name")?,
                method_name: r.try_get("method_name")?,
                priority: r.try_get("priority")?,
                script: r.try_get("script")?,
                is_active: r.try_get("is_active")?,
            };
            registry.register(sub);
        }

        Ok(registry)
    }

    /// Persist a subscriber to PostgreSQL and register in-memory
    pub async fn persist_subscriber(
        &self,
        pool: &PgPool,
        sub: EventSubscriber,
    ) -> Result<Uuid, RuntimeError> {
        Self::init_table(pool).await?;
        let upsert_query = r#"
            INSERT INTO "_bos_event_subscribers" (
                "id", "publisher", "event_name", "subscriber_name", "method_name", "priority", "script", "is_active"
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT ("id") DO UPDATE SET
                "publisher" = EXCLUDED."publisher",
                "event_name" = EXCLUDED."event_name",
                "subscriber_name" = EXCLUDED."subscriber_name",
                "method_name" = EXCLUDED."method_name",
                "priority" = EXCLUDED."priority",
                "script" = EXCLUDED."script",
                "is_active" = EXCLUDED."is_active";
        "#;
        sqlx::query(upsert_query)
            .bind(sub.id)
            .bind(&sub.publisher)
            .bind(&sub.event_name)
            .bind(&sub.subscriber_name)
            .bind(&sub.method_name)
            .bind(sub.priority)
            .bind(&sub.script)
            .bind(sub.is_active)
            .execute(pool)
            .await?;

        let id = sub.id;
        self.register(sub);
        Ok(id)
    }

    /// Dispatch event: executes all registered subscribers sequentially in priority order.
    /// Subscribers run inside `db` (the publisher's transaction), so a failing subscriber
    /// fails the publishing operation.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch(
        &self,
        publisher: &str,
        event_name: &str,
        data: &mut serde_json::Value,
        x_rec: Option<&serde_json::Value>,
        ctx: &SessionContext,
        db: Option<&DbSession>,
        quickjs: &QuickJsEngine,
        logs: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        let subs = self.get_subscribers(publisher, event_name);
        for sub in subs {
            logs.push(format!(
                "Executing event subscriber '{}.{}' (priority {}) on '{}.{}'",
                sub.subscriber_name, sub.method_name, sub.priority, publisher, event_name
            ));

            quickjs.execute_trigger(
                publisher,
                &format!("{}.{}", sub.subscriber_name, sub.method_name),
                &sub.script,
                data,
                x_rec,
                ctx,
                db,
                logs,
            )?;
        }
        Ok(())
    }

    /// Dispatch on a blocking worker thread (QuickJS host calls block on database I/O).
    pub async fn dispatch_in(
        &self,
        publisher: &str,
        event_name: &str,
        data: serde_json::Value,
        x_rec: Option<serde_json::Value>,
        scope: &ExecScope,
    ) -> Result<(serde_json::Value, Vec<String>), RuntimeError> {
        let registry = self.clone();
        let publisher = publisher.to_string();
        let event_name = event_name.to_string();
        let scope = scope.clone();
        tokio::task::spawn_blocking(move || {
            let quickjs =
                QuickJsEngine::with_limits(scope.services.clone(), ExecutionLimits::default())?;
            let mut data = data;
            let mut logs = Vec::new();
            registry.dispatch(
                &publisher,
                &event_name,
                &mut data,
                x_rec.as_ref(),
                &scope.ctx,
                Some(&scope.db),
                &quickjs,
                &mut logs,
            )?;
            Ok((data, logs))
        })
        .await
        .map_err(|e| RuntimeError::Internal(format!("Event dispatch worker join error: {:?}", e)))?
    }

    /// Enqueue an event into the transactional outbox inside an existing unit of work, so the
    /// event is published if and only if the business transaction commits.
    pub async fn enqueue_outbox_in(
        db: &DbSession,
        tenant_id: Uuid,
        company_id: Option<Uuid>,
        publisher: &str,
        event_name: &str,
        payload: &serde_json::Value,
    ) -> Result<Uuid, RuntimeError> {
        let mut conn = db.conn().await?;
        let res = sqlx::query_scalar::<_, Uuid>(
            r#"INSERT INTO "_bos_event_outbox" ("id", "tenant_id", "company_id", "publisher", "event_name", "payload", "status", "retry_count", "created_at")
               VALUES ($1, $2, $3, $4, $5, $6, 'pending', 0, NOW()) RETURNING "id""#,
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(company_id)
        .bind(publisher)
        .bind(event_name)
        .bind(payload)
        .fetch_one(&mut *conn)
        .await;
        db.track(res)
    }

    /// Enqueue an event into the transactional outbox table within an existing database transaction
    pub async fn enqueue_outbox_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tenant_id: Uuid,
        company_id: Option<Uuid>,
        publisher: &str,
        event_name: &str,
        payload: &serde_json::Value,
    ) -> Result<Uuid, RuntimeError> {
        let id = Uuid::new_v4();
        let query = r#"
            INSERT INTO "_bos_event_outbox" (
                "id", "tenant_id", "company_id", "publisher", "event_name", "payload", "status", "retry_count", "created_at"
            ) VALUES ($1, $2, $3, $4, $5, $6, 'pending', 0, NOW())
            RETURNING "id";
        "#;
        let inserted_id: Uuid = sqlx::query_scalar(query)
            .bind(id)
            .bind(tenant_id)
            .bind(company_id)
            .bind(publisher)
            .bind(event_name)
            .bind(payload)
            .fetch_one(&mut **tx)
            .await?;
        Ok(inserted_id)
    }

    /// Enqueue an event directly into the outbox pool (standalone)
    pub async fn enqueue_outbox(
        pool: &PgPool,
        tenant_id: Uuid,
        company_id: Option<Uuid>,
        publisher: &str,
        event_name: &str,
        payload: &serde_json::Value,
    ) -> Result<Uuid, RuntimeError> {
        Self::init_table(pool).await?;
        let mut tx = pool.begin().await?;
        let id = Self::enqueue_outbox_tx(
            &mut tx, tenant_id, company_id, publisher, event_name, payload,
        )
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// List outbox events with optional status filter
    pub async fn list_outbox(
        pool: &PgPool,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<OutboxEvent>, RuntimeError> {
        Self::init_table(pool).await?;
        let query = if let Some(st) = status {
            sqlx::query(
                r#"
                SELECT id, tenant_id, company_id, publisher, event_name, payload, status, retry_count, max_retries, last_error, created_at, processed_at
                FROM "_bos_event_outbox"
                WHERE "status" = $1
                ORDER BY "created_at" DESC
                LIMIT $2;
                "#
            ).bind(st).bind(limit.clamp(1, 100))
        } else {
            sqlx::query(
                r#"
                SELECT id, tenant_id, company_id, publisher, event_name, payload, status, retry_count, max_retries, last_error, created_at, processed_at
                FROM "_bos_event_outbox"
                ORDER BY "created_at" DESC
                LIMIT $1;
                "#
            ).bind(limit.clamp(1, 100))
        };

        let rows = query.fetch_all(pool).await?;
        let mut events = Vec::new();
        for r in rows {
            events.push(OutboxEvent {
                id: r.try_get("id")?,
                tenant_id: r.try_get("tenant_id")?,
                company_id: r.try_get("company_id")?,
                publisher: r.try_get("publisher")?,
                event_name: r.try_get("event_name")?,
                payload: r.try_get("payload")?,
                status: r.try_get("status")?,
                retry_count: r.try_get("retry_count")?,
                max_retries: r.try_get("max_retries")?,
                last_error: r.try_get("last_error")?,
                created_at: r.try_get("created_at")?,
                processed_at: r.try_get("processed_at")?,
            });
        }
        Ok(events)
    }

    /// Process a batch of pending outbox events using FOR UPDATE SKIP LOCKED.
    ///
    /// The batch runs in one transaction; each event gets its own savepoint. A subscriber
    /// failure rolls back only that event's side effects, then the retry bookkeeping for it
    /// is written, so status updates and subscriber writes always commit together.
    pub async fn process_outbox_batch(
        &self,
        pool: &PgPool,
        services: &ScriptServices,
        batch_size: i64,
    ) -> Result<ProcessOutboxSummary, RuntimeError> {
        Self::init_table(pool).await?;
        let db = DbSession::begin(pool).await?;
        let outcome = self.process_outbox_in(&db, services, batch_size).await;
        let (succeeded, failed) = db.finish(outcome).await?;

        let remaining: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*)::bigint FROM "_bos_event_outbox" WHERE "status" = 'pending';"#,
        )
        .fetch_one(pool)
        .await?;

        Ok(ProcessOutboxSummary {
            processed: succeeded + failed,
            succeeded,
            failed,
            remaining: remaining as usize,
        })
    }

    async fn process_outbox_in(
        &self,
        db: &DbSession,
        services: &ScriptServices,
        batch_size: i64,
    ) -> Result<(usize, usize), RuntimeError> {
        let rows = {
            let mut conn = db.conn().await?;
            let res = sqlx::query(
                r#"
                SELECT id, tenant_id, company_id, publisher, event_name, payload, retry_count, max_retries
                FROM "_bos_event_outbox"
                WHERE "status" = 'pending' 
                  AND "retry_count" < "max_retries"
                  AND ("next_retry_at" IS NULL OR "next_retry_at" <= NOW())
                ORDER BY "created_at" ASC
                LIMIT $1
                FOR UPDATE SKIP LOCKED;
                "#,
            )
            .bind(batch_size.clamp(1, 100))
            .fetch_all(&mut *conn)
            .await;
            db.track(res)?
        };

        let mut succeeded = 0;
        let mut failed = 0;
        for r in &rows {
            let id: Uuid = r.try_get("id")?;
            let tenant_id: Uuid = r.try_get("tenant_id")?;
            let company_id: Option<Uuid> = r.try_get("company_id")?;
            let publisher: String = r.try_get("publisher")?;
            let event_name: String = r.try_get("event_name")?;
            let mut payload: serde_json::Value = r.try_get("payload")?;
            if let Some(obj) = payload.as_object_mut() {
                if !obj.contains_key("_event_id") {
                    obj.insert("_event_id".to_string(), serde_json::json!(id.to_string()));
                }
            }
            let retry_count: i32 = r.try_get("retry_count")?;
            let max_retries: i32 = r.try_get("max_retries")?;

            let ctx = SessionContext {
                tenant_id,
                company_id: company_id.unwrap_or_default(),
                user_id: Uuid::nil(),
                roles: vec!["SystemWorker".to_string()],
            };
            let scope = ExecScope::new(db.clone(), ctx.clone(), services.clone());

            let savepoint = format!("bos_outbox_{}", id.simple());
            db.savepoint(&savepoint).await?;
            let dispatch_res = match db.apply_identity(&ctx).await {
                Ok(()) => self
                    .dispatch_in(&publisher, &event_name, payload, None, &scope)
                    .await
                    .map(|_| ()),
                Err(e) => Err(e),
            };
            if dispatch_res.is_err() {
                db.rollback_to_savepoint(&savepoint).await?;
            }
            db.release_savepoint(&savepoint).await?;

            let mut conn = db.conn().await?;
            match dispatch_res {
                Ok(()) => {
                    let res = sqlx::query(
                        r#"UPDATE "_bos_event_outbox" SET "status" = 'completed', "processed_at" = NOW() WHERE "id" = $1"#,
                    )
                    .bind(id)
                    .execute(&mut *conn)
                    .await;
                    db.track(res)?;
                    succeeded += 1;
                }
                Err(e) => {
                    let new_retry = retry_count + 1;
                    let new_status = if new_retry >= max_retries {
                        "failed"
                    } else {
                        "pending"
                    };
                    // Exponential backoff: 2^new_retry seconds (capped at 300s)
                    let backoff_secs = 2_i64.pow(new_retry.min(8) as u32);
                    let res = sqlx::query(
                        r#"UPDATE "_bos_event_outbox" 
                           SET "retry_count" = $2, 
                               "status" = $3, 
                               "last_error" = $4,
                               "next_retry_at" = NOW() + ($5 * INTERVAL '1 second')
                           WHERE "id" = $1"#,
                    )
                    .bind(id)
                    .bind(new_retry)
                    .bind(new_status)
                    .bind(e.to_string())
                    .bind(backoff_secs)
                    .execute(&mut *conn)
                    .await;
                    db.track(res)?;
                    failed += 1;
                }
            }
        }
        Ok((succeeded, failed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_registry_priority_order() {
        let registry = EventRegistry::new();
        let sub1 = EventSubscriber {
            id: Uuid::new_v4(),
            publisher: "Customer".to_string(),
            event_name: "onAfterInsert".to_string(),
            subscriber_name: "AuditLog".to_string(),
            method_name: "LogCust".to_string(),
            priority: 50,
            script: "rec.priority50 = true;".to_string(),
            is_active: true,
        };
        let sub2 = EventSubscriber {
            id: Uuid::new_v4(),
            publisher: "Customer".to_string(),
            event_name: "onAfterInsert".to_string(),
            subscriber_name: "TaxInit".to_string(),
            method_name: "InitTax".to_string(),
            priority: 10,
            script: "rec.priority10 = true;".to_string(),
            is_active: true,
        };

        registry.register(sub1);
        registry.register(sub2);

        let subs = registry.get_subscribers("Customer", "onAfterInsert");
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0].priority, 10);
        assert_eq!(subs[1].priority, 50);
    }

    #[test]
    fn test_event_dispatch_execution() {
        let registry = EventRegistry::new();
        let quickjs = QuickJsEngine::new().expect("QuickJS init");
        let ctx = SessionContext::default();
        let mut logs = Vec::new();

        let sub = EventSubscriber {
            id: Uuid::new_v4(),
            publisher: "SalesHeader".to_string(),
            event_name: "onAfterInsert".to_string(),
            subscriber_name: "CommissionSubscriber".to_string(),
            method_name: "ComputeCommission".to_string(),
            priority: 10,
            script: r#"
                rec.commission = Number(rec.totalAmount) * 0.05;
                rec.subscriberExecuted = true;
            "#
            .to_string(),
            is_active: true,
        };
        registry.register(sub);

        let mut data = serde_json::json!({
            "orderNo": "SO-101",
            "totalAmount": 1000.0,
            "commission": 0.0
        });

        let res = registry.dispatch(
            "SalesHeader",
            "onAfterInsert",
            &mut data,
            None,
            &ctx,
            None,
            &quickjs,
            &mut logs,
        );
        assert!(res.is_ok(), "Dispatch failed: {:?}", res.err());

        assert_eq!(data.get("commission").and_then(|v| v.as_f64()), Some(50.0));
        assert_eq!(
            data.get("subscriberExecuted").and_then(|v| v.as_bool()),
            Some(true)
        );
    }
}
