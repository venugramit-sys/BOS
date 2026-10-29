with open('crates/bosc-runtime/src/lib.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old_listen = """    /// LISTEN loop for `bos_pages_channel`; reconnects to the active database after errors or switches.
    async fn listen_for_page_changes(&self) {
        loop {
            let pool = self.pool().await;
            match sqlx::postgres::PgListener::connect_with(&pool).await {
                Ok(mut listener) => {
                    if listener.listen("bos_pages_channel").await.is_ok() {
                        while let Ok(n) = listener.recv().await {
                            let Some(service) = self.page_service() else { continue };
                            let Some((db, instance)) = n.payload().split_once('|') else { continue };
                            let own = instance == service.instance().to_string();
                            if !own && service.db_key().as_deref() == Some(db) {
                                self.refresh_pages().await;
                            }
                        }
                    }
                }
                Err(e) => tracing::warn!(target: "bos::pages", "page change listener: {e}"),
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }"""

new_listen = """    /// LISTEN loop for `bos_pages_channel` and `bos_meta_channel`; reconnects to the active database after errors or switches.
    async fn listen_for_page_changes(&self) {
        loop {
            let pool = self.pool().await;
            match sqlx::postgres::PgListener::connect_with(&pool).await {
                Ok(mut listener) => {
                    let _ = listener.listen("bos_pages_channel").await;
                    let _ = listener.listen("bos_meta_channel").await;
                    while let Ok(n) = listener.recv().await {
                        let Some(service) = self.page_service() else { continue };
                        let payload = n.payload();
                        if n.channel() == "bos_meta_channel" || payload.contains("external") || payload.starts_with('*') {
                            tracing::info!(target: "bos::pages", "Dynamic package/page update detected. Hot-reloading page catalog...");
                            self.meta_cache.clear();
                            self.refresh_pages().await;
                            continue;
                        }
                        let Some((db, instance)) = payload.split_once('|') else { continue };
                        let own = instance == service.instance().to_string();
                        if !own && (service.db_key().as_deref() == Some(db) || db == "*") {
                            self.refresh_pages().await;
                        }
                    }
                }
                Err(e) => tracing::warn!(target: "bos::pages", "page change listener: {e}"),
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }"""

old_notify = """        let _ = sqlx::query("SELECT pg_notify('bos_meta_channel', '*');")
            .execute(&pool)
            .await;"""

new_notify = """        let _ = sqlx::query("SELECT pg_notify('bos_meta_channel', '*');")
            .execute(&pool)
            .await;
        let _ = sqlx::query("SELECT pg_notify('bos_pages_channel', '*|external');")
            .execute(&pool)
            .await;"""

content_norm = content.replace("\r\n", "\n")
assert old_listen.replace("\r\n", "\n") in content_norm, "old_listen not found"
assert old_notify.replace("\r\n", "\n") in content_norm, "old_notify not found"

content_norm = content_norm.replace(old_listen.replace("\r\n", "\n"), new_listen.replace("\r\n", "\n"), 1)
content_norm = content_norm.replace(old_notify.replace("\r\n", "\n"), new_notify.replace("\r\n", "\n"), 1)

with open('crates/bosc-runtime/src/lib.rs', 'w', encoding='utf-8', newline='\r\n') as f:
    f.write(content_norm)

print("Updated bosc-runtime/src/lib.rs for hot package reloading successfully!")
