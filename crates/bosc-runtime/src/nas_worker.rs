use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::BosEngine;
use crate::node_config::NasWorkerSection;

pub struct NasWorkerHandle {
    shutdown_tx: watch::Sender<bool>,
    join_handle: JoinHandle<()>,
    is_running: Arc<AtomicBool>,
}

impl NasWorkerHandle {
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub async fn stop(self) {
        let _ = self.shutdown_tx.send(true);
        let _ = self.join_handle.await;
    }
}

pub struct NasWorkerDaemon;

impl NasWorkerDaemon {
    pub fn start(engine: Arc<BosEngine>, config: NasWorkerSection) -> NasWorkerHandle {
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_clone = is_running.clone();

        let join_handle = tokio::spawn(async move {
            info!(
                "🚀 Navision Application Server (NAS) background worker started (poll: {}ms, batch: {})",
                config.poll_interval_ms, config.batch_size
            );

            let poll_duration = Duration::from_millis(config.poll_interval_ms.max(100));

            let mut pg_listener: Option<sqlx::postgres::PgListener> = None;
            let mut reconnect_countdown: u32 = 0;

            loop {
                // Check shutdown signal
                if *shutdown_rx.borrow() {
                    info!("NAS background worker received graceful shutdown signal");
                    break;
                }

                // If listener is not active, attempt reconnection periodically
                if pg_listener.is_none() {
                    if reconnect_countdown == 0 {
                        let pool = engine.pool().await;
                        match sqlx::postgres::PgListener::connect_with(&pool).await {
                            Ok(mut l) => {
                                if let Err(e) = l.listen("bos_event_outbox_channel").await {
                                    warn!("NAS worker failed to register LISTEN on bos_event_outbox_channel: {}", e);
                                } else {
                                    info!("⚡ NAS worker registered LISTEN on bos_event_outbox_channel (zero-latency event wakeups enabled)");
                                    pg_listener = Some(l);
                                }
                            }
                            Err(e) => {
                                warn!("NAS worker could not establish PG listener connection: {}", e);
                            }
                        }
                        reconnect_countdown = 10;
                    } else {
                        reconnect_countdown = reconnect_countdown.saturating_sub(1);
                    }
                }

                if config.process_outbox {
                    let pool = engine.pool().await;
                    let script_services = engine.script_services();
                    match engine
                        .event_registry
                        .process_outbox_batch(&pool, &script_services, config.batch_size)
                        .await
                    {
                        Ok(summary) => {
                            if summary.processed > 0 {
                                info!(
                                    "NAS worker processed outbox batch: {} processed, {} succeeded, {} failed ({} remaining)",
                                    summary.processed,
                                    summary.succeeded,
                                    summary.failed,
                                    summary.remaining
                                );
                            } else {
                                debug!("NAS worker idle; 0 outbox events pending");
                            }
                        }
                        Err(err) => {
                            let err_msg = err.to_string();
                            if err_msg.contains("does not exist") {
                                let db_name = pool.connect_options().get_database().unwrap_or("default").to_string();
                                crate::event_engine::EventRegistry::invalidate_cache(&db_name);
                            }
                            warn!("NAS worker encountered outbox processing error: {}", err);
                        }
                    }
                }

                if config.process_job_queue {
                    let ctx = crate::SessionContext {
                        tenant_id: uuid::Uuid::nil(),
                        company_id: uuid::Uuid::nil(),
                        user_id: uuid::Uuid::nil(),
                        roles: vec!["SystemWorker".to_string(), "Admin".to_string()],
                    };
                    match engine.execute_next_job(&ctx).await {
                        Ok(Some(job_id)) => {
                            info!("NAS worker executed background job from queue: {}", job_id);
                        }
                        Ok(None) => {
                            debug!("NAS worker job queue idle; 0 jobs ready");
                        }
                        Err(err) => {
                            warn!("NAS worker encountered job execution error: {}", err);
                        }
                    }
                }

                tokio::select! {
                    notif = async {
                        if let Some(ref mut l) = pg_listener {
                            match l.recv().await {
                                Ok(n) => Some(n),
                                Err(err) => {
                                    warn!("NAS worker PG listener connection dropped: {}. Falling back to polling.", err);
                                    None
                                }
                            }
                        } else {
                            std::future::pending::<Option<sqlx::postgres::PgNotification>>().await
                        }
                    } => {
                        match notif {
                            Some(n) => {
                                debug!("NAS worker woke up immediately on outbox notification: channel={}, payload={}", n.channel(), n.payload());
                            }
                            None => {
                                pg_listener = None;
                                tokio::time::sleep(poll_duration).await;
                            }
                        }
                    }
                    _ = tokio::time::sleep(poll_duration) => {}
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            info!("NAS background worker shutting down");
                            break;
                        }
                    }
                }
            }

            is_running_clone.store(false, Ordering::SeqCst);
            info!("NAS background worker stopped cleanly");
        });

        NasWorkerHandle {
            shutdown_tx,
            join_handle,
            is_running,
        }
    }
}
