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

            loop {
                // Check shutdown signal
                if *shutdown_rx.borrow() {
                    info!("NAS background worker received graceful shutdown signal");
                    break;
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
