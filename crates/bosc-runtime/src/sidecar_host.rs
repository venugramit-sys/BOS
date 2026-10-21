use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::RuntimeError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SidecarStatus {
    Registered,
    Running,
    Unhealthy,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarDescriptor {
    pub name: String,
    pub endpoint_url: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub status: SidecarStatus,
    pub description: Option<String>,
}

pub struct SidecarHost {
    services: Arc<DashMap<String, SidecarDescriptor>>,
    client: reqwest::Client,
    children: Mutex<HashMap<String, Child>>,
}

impl Default for SidecarHost {
    fn default() -> Self {
        Self::new()
    }
}

impl SidecarHost {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        Self {
            services: Arc::new(DashMap::new()),
            client,
            children: Mutex::new(HashMap::new()),
        }
    }

    /// Register a loopback sidecar. A command is started. A URL with no command stays registered
    /// until a health check reaches it. Anything else is refused.
    pub fn register_service(
        &self,
        name: &str,
        endpoint_url: &str,
        command: Option<String>,
        args: Vec<String>,
        description: Option<String>,
    ) -> Result<(), RuntimeError> {
        let name_key = name.trim().to_lowercase();
        if name_key.is_empty() {
            return Err(RuntimeError::SecurityError(
                "A sidecar needs a name".to_string(),
            ));
        }
        let endpoint_url = endpoint_url.trim().to_string();
        if !is_loopback_http(&endpoint_url) {
            return Err(RuntimeError::SecurityError(
                "A sidecar URL must be http or https on localhost or 127.0.0.1".to_string(),
            ));
        }
        let mut status = SidecarStatus::Registered;
        if let Some(command) = command.as_ref() {
            let command = command.trim();
            if command.is_empty() {
                return Err(RuntimeError::SecurityError(
                    "A sidecar command is empty".to_string(),
                ));
            }
            let child = Command::new(command).args(&args).spawn().map_err(|err| {
                RuntimeError::Internal(format!("Sidecar '{name_key}' did not start: {err}"))
            })?;
            if let Ok(mut children) = self.children.lock() {
                if let Some(mut previous) = children.remove(&name_key) {
                    let _ = previous.kill();
                }
                children.insert(name_key.clone(), child);
            }
            status = SidecarStatus::Running;
        }
        let descriptor = SidecarDescriptor {
            name: name.trim().to_string(),
            endpoint_url,
            command,
            args,
            status,
            description,
        };
        self.services.insert(name_key, descriptor);
        Ok(())
    }

    /// Check if a sidecar service is registered
    pub fn has_service(&self, name: &str) -> bool {
        self.services.contains_key(&name.trim().to_lowercase())
    }

    /// List all registered sidecars
    pub fn list_services(&self) -> Vec<SidecarDescriptor> {
        self.services.iter().map(|kv| kv.value().clone()).collect()
    }

    /// Invoke an endpoint on a sidecar microservice over high-speed local IPC / loopback HTTP
    pub async fn invoke(
        &self,
        service_name: &str,
        path: &str,
        payload: &JsonValue,
    ) -> Result<JsonValue, RuntimeError> {
        let descriptor = self
            .services
            .get(&service_name.trim().to_lowercase())
            .ok_or_else(|| {
                RuntimeError::NotFound("SidecarService".to_string(), service_name.to_string())
            })?;

        let clean_path = path.trim_start_matches('/');
        let base = descriptor.endpoint_url.trim_end_matches('/');
        let full_url = format!("{}/{}", base, clean_path);

        let res = self
            .client
            .post(&full_url)
            .json(payload)
            .header("X-BOS-Sidecar-Client", "bosc-runtime")
            .send()
            .await
            .map_err(|e| {
                RuntimeError::Internal(format!(
                    "Failed to communicate with sidecar '{}' at {}: {:?}",
                    service_name, full_url, e
                ))
            })?;

        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().await.unwrap_or_default();
            return Err(RuntimeError::Internal(format!(
                "Sidecar '{}' responded with error status {}: {}",
                service_name, status, err_body
            )));
        }

        let body: JsonValue = res.json().await.map_err(|e| {
            RuntimeError::Internal(format!(
                "Failed to parse JSON response from sidecar '{}': {:?}",
                service_name, e
            ))
        })?;

        Ok(body)
    }

    /// Check the health status of a sidecar microservice
    pub async fn check_health(&self, service_name: &str) -> bool {
        let key = service_name.trim().to_lowercase();
        let descriptor_opt = self.services.get(&key).map(|d| d.value().clone());

        if let Some(mut descriptor) = descriptor_opt {
            let base = descriptor.endpoint_url.trim_end_matches('/');
            let health_url = format!("{}/health", base);
            let ok = self
                .client
                .get(&health_url)
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false);

            descriptor.status = if ok {
                SidecarStatus::Running
            } else {
                SidecarStatus::Unhealthy
            };
            self.services.insert(key, descriptor);
            ok
        } else {
            false
        }
    }
}

impl Drop for SidecarHost {
    fn drop(&mut self) {
        if let Ok(mut children) = self.children.lock() {
            for (_, mut child) in children.drain() {
                let _ = child.kill();
            }
        }
    }
}

fn is_loopback_http(url: &str) -> bool {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    let Some(rest) = rest else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let hostport = authority.rsplit('@').next().unwrap_or(authority);
    let host = if let Some(inner) = hostport.strip_prefix('[') {
        inner.split(']').next().unwrap_or("")
    } else {
        hostport.split(':').next().unwrap_or("")
    };
    let host = host.trim().to_ascii_lowercase();
    host == "localhost" || host == "127.0.0.1" || host == "::1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidecar_registration_and_lookup() {
        let host = SidecarHost::new();
        host.register_service(
            "ai_forecaster",
            "http://127.0.0.1:5005",
            None,
            vec![],
            Some("Demand forecasting service already listening".to_string()),
        )
        .expect("loopback registration");

        assert!(host.has_service("ai_forecaster"));
        assert!(
            host.register_service("remote", "https://example.com/hook", None, vec![], None)
                .is_err()
        );
        let services = host.list_services();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "ai_forecaster");
        assert_eq!(services[0].endpoint_url, "http://127.0.0.1:5005");
    }
}
