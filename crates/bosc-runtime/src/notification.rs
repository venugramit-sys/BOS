use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

/// Notification severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationSeverity {
    Info,
    Success,
    Warning,
    Error,
}

/// Where and how the notification is delivered and rendered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationTarget {
    /// Transient floating toast (auto-dismiss)
    Toast,
    /// Blocking or alert modal dialog
    Popup,
    /// Targeted form input validation message attached to a specific field
    Infield,
    /// Persistent or dismissible page-top banner
    Banner,
}

/// Optional action attached to a notification (e.g. retry, navigate, dismiss)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub label: String,
    pub action_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

/// Unified notification message representation across backend and frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub severity: NotificationSeverity,
    pub target: NotificationTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<NotificationAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
}

impl Notification {
    pub fn new(
        severity: NotificationSeverity,
        target: NotificationTarget,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            severity,
            target,
            title: None,
            message: message.into(),
            field: None,
            code: None,
            details: None,
            timestamp: Utc::now(),
            duration_ms: match target {
                NotificationTarget::Toast => Some(5000),
                _ => None,
            },
            actions: Vec::new(),
            tenant_id: None,
            company_id: None,
            user_id: None,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(
            NotificationSeverity::Info,
            NotificationTarget::Toast,
            message,
        )
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::new(
            NotificationSeverity::Success,
            NotificationTarget::Toast,
            message,
        )
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(
            NotificationSeverity::Warning,
            NotificationTarget::Toast,
            message,
        )
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(
            NotificationSeverity::Error,
            NotificationTarget::Toast,
            message,
        )
    }

    pub fn popup(
        severity: NotificationSeverity,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let mut n = Self::new(severity, NotificationTarget::Popup, message);
        n.title = Some(title.into());
        n
    }

    pub fn infield(field: impl Into<String>, message: impl Into<String>) -> Self {
        let mut n = Self::new(
            NotificationSeverity::Error,
            NotificationTarget::Infield,
            message,
        );
        n.field = Some(field.into());
        n
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_scope(mut self, tenant_id: Uuid, company_id: Uuid) -> Self {
        self.tenant_id = Some(tenant_id);
        self.company_id = Some(company_id);
        self
    }

    pub fn with_action(mut self, action: NotificationAction) -> Self {
        self.actions.push(action);
        self
    }
}

/// Central pub/sub notification hub managing active broadcast channels and historical buffer
pub struct NotificationHub {
    sender: broadcast::Sender<Notification>,
    history: RwLock<VecDeque<Notification>>,
    max_history: usize,
}

impl NotificationHub {
    pub fn new(capacity: usize, max_history: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            history: RwLock::new(VecDeque::with_capacity(max_history)),
            max_history,
        }
    }

    pub fn default_hub() -> Self {
        Self::new(256, 100)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.sender.subscribe()
    }

    pub async fn broadcast(&self, notification: Notification) {
        {
            let mut hist = self.history.write().await;
            if hist.len() >= self.max_history {
                hist.pop_front();
            }
            hist.push_back(notification.clone());
        }
        let _ = self.sender.send(notification);
    }

    pub async fn get_history(&self) -> Vec<Notification> {
        self.history.read().await.iter().cloned().collect()
    }

    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }
}

impl Default for NotificationHub {
    fn default() -> Self {
        Self::default_hub()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_constructors() {
        let n_info = Notification::info("Operation complete");
        assert_eq!(n_info.severity, NotificationSeverity::Info);
        assert_eq!(n_info.target, NotificationTarget::Toast);
        assert_eq!(n_info.message, "Operation complete");
        assert_eq!(n_info.duration_ms, Some(5000));

        let n_error = Notification::error("Critical error");
        assert_eq!(n_error.severity, NotificationSeverity::Error);
        assert_eq!(n_error.target, NotificationTarget::Toast);

        let n_popup = Notification::popup(
            NotificationSeverity::Warning,
            "System Restart",
            "Rebooting in 5m",
        );
        assert_eq!(n_popup.target, NotificationTarget::Popup);
        assert_eq!(n_popup.title.as_deref(), Some("System Restart"));
        assert_eq!(n_popup.message, "Rebooting in 5m");

        let n_infield = Notification::infield("customer_code", "Code is required")
            .with_code("VALIDATION_ERROR");
        assert_eq!(n_infield.target, NotificationTarget::Infield);
        assert_eq!(n_infield.field.as_deref(), Some("customer_code"));
        assert_eq!(n_infield.code.as_deref(), Some("VALIDATION_ERROR"));
    }

    #[tokio::test]
    async fn test_notification_hub_broadcast_and_history() {
        let hub = NotificationHub::new(10, 5);
        let mut rx = hub.subscribe();

        let notif1 = Notification::info("Message 1");
        hub.broadcast(notif1.clone()).await;

        let received = rx.recv().await.expect("Should receive notification");
        assert_eq!(received.message, "Message 1");

        let history = hub.get_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].message, "Message 1");

        // Broadcast more than max_history (5) to verify ring buffer behavior
        for i in 2..=8 {
            hub.broadcast(Notification::info(format!("Message {}", i)))
                .await;
        }

        let updated_history = hub.get_history().await;
        assert_eq!(updated_history.len(), 5);
        assert_eq!(updated_history[0].message, "Message 4");
        assert_eq!(updated_history[4].message, "Message 8");

        hub.clear_history().await;
        assert_eq!(hub.get_history().await.len(), 0);
    }
}
