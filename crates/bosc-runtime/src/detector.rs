use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    PostgreSQL,
    MySQL,
    MariaDB,
    SqlServer,
    SQLite,
    ClickHouse,
    Redis,
    MongoDB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedDatabase {
    pub provider: ProviderKind,
    pub display_name: String,
    pub is_running: bool,
    pub host: String,
    pub port: u16,
    pub default_url_template: String,
    pub binary_path: Option<PathBuf>,
    pub details: String,
}

pub struct DatabaseDetector;

impl DatabaseDetector {
    /// The kernel opens PostgreSQL only. Other engines are not reported as usable.
    pub fn scan_all() -> Vec<DetectedDatabase> {
        vec![Self::detect_postgresql()]
    }

    /// Specifically probe PostgreSQL
    pub fn detect_postgresql() -> DetectedDatabase {
        let port = 5432;
        let is_running = Self::probe_tcp_port("127.0.0.1", port);
        let binary_path = Self::find_binary_candidates(&[
            r"C:\Program Files\PostgreSQL\18\bin\psql.exe",
            r"C:\Program Files\PostgreSQL\17\bin\psql.exe",
            r"C:\Program Files\PostgreSQL\16\bin\psql.exe",
            r"C:\Program Files\PostgreSQL\15\bin\psql.exe",
            "/usr/bin/psql",
            "/usr/local/bin/psql",
        ]);

        let details = if is_running {
            "Active and listening on TCP 5432 (Ready for BOS)".to_string()
        } else if binary_path.is_some() {
            "Installed locally but service is stopped".to_string()
        } else {
            "Not detected".to_string()
        };

        DetectedDatabase {
            provider: ProviderKind::PostgreSQL,
            display_name: "PostgreSQL (Default BOS Engine)".to_string(),
            is_running,
            host: "127.0.0.1".to_string(),
            port,
            default_url_template: "postgres://USER:PASSWORD@127.0.0.1:5432/DATABASE".to_string(),
            binary_path,
            details,
        }
    }

    /// Fast non-blocking TCP socket connect probe
    fn probe_tcp_port(host: &str, port: u16) -> bool {
        let Ok(addr) = format!("{}:{}", host, port).parse::<SocketAddr>() else {
            return false;
        };
        TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
    }

    /// Search common file system paths for database client binaries
    fn find_binary_candidates(paths: &[&str]) -> Option<PathBuf> {
        for p in paths {
            let path = PathBuf::from(p);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }
}
