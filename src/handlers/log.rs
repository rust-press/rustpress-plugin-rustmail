//! Log Handler

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::{EmailLog, EmailEvent, LogFilter, LogStats};
use crate::services::LogService;

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub email_id: Option<String>,
    pub recipient: Option<String>,
    pub event: Option<String>,
    pub template_id: Option<String>,
    pub provider: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub errors_only: Option<bool>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct LogEntryResponse {
    pub id: String,
    pub email_id: String,
    pub queue_id: Option<String>,
    pub event: String,
    pub recipient: String,
    pub subject: String,
    pub template_name: Option<String>,
    pub timestamp: String,
    pub provider: String,
    pub provider_message_id: Option<String>,
    pub error: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub click_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogStatsResponse {
    pub total_sent: u64,
    pub total_delivered: u64,
    pub total_bounced: u64,
    pub total_opened: u64,
    pub total_clicked: u64,
    pub total_spam_complaints: u64,
    pub total_unsubscribes: u64,
    pub total_failed: u64,
    pub delivery_rate: f64,
    pub open_rate: f64,
    pub click_rate: f64,
    pub bounce_rate: f64,
    pub spam_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct SuppressionEntry {
    pub email: String,
    pub reason: String,
}

/// Log handler
pub struct LogHandler {
    log_service: Arc<LogService>,
}

impl LogHandler {
    pub fn new(log_service: Arc<LogService>) -> Self {
        Self { log_service }
    }

    /// Query logs
    pub async fn query(&self, query: LogQuery) -> Vec<LogEntryResponse> {
        let filter = LogFilter {
            email_id: query.email_id.and_then(|s| Uuid::parse_str(&s).ok()),
            recipient: query.recipient,
            event: query.event.and_then(|e| Self::parse_event(&e)),
            template_id: query.template_id.and_then(|s| Uuid::parse_str(&s).ok()),
            provider: query.provider,
            from_date: query.from_date.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
            to_date: query.to_date.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
            errors_only: query.errors_only.unwrap_or(false),
            limit: query.limit.unwrap_or(50),
            offset: query.offset.unwrap_or(0),
        };

        self.log_service.query(filter).await
            .into_iter()
            .map(|e| Self::to_response(&e))
            .collect()
    }

    /// Get logs for email
    pub async fn for_email(&self, email_id: &str) -> Result<Vec<LogEntryResponse>, String> {
        let uuid = Uuid::parse_str(email_id).map_err(|e| e.to_string())?;

        Ok(self.log_service.get_for_email(uuid).await
            .into_iter()
            .map(|e| Self::to_response(&e))
            .collect())
    }

    /// Get logs for recipient
    pub async fn for_recipient(&self, recipient: &str) -> Vec<LogEntryResponse> {
        self.log_service.get_for_recipient(recipient).await
            .into_iter()
            .map(|e| Self::to_response(&e))
            .collect()
    }

    /// Get recent logs
    pub async fn recent(&self, limit: u32) -> Vec<LogEntryResponse> {
        self.log_service.recent(limit).await
            .into_iter()
            .map(|e| Self::to_response(&e))
            .collect()
    }

    /// Get statistics
    pub async fn stats(&self, from_date: Option<String>, to_date: Option<String>) -> LogStatsResponse {
        let from = from_date.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)));
        let to = to_date.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)));

        let stats = self.log_service.stats(from, to).await;

        LogStatsResponse {
            total_sent: stats.total_sent,
            total_delivered: stats.total_delivered,
            total_bounced: stats.total_bounced,
            total_opened: stats.total_opened,
            total_clicked: stats.total_clicked,
            total_spam_complaints: stats.total_spam_complaints,
            total_unsubscribes: stats.total_unsubscribes,
            total_failed: stats.total_failed,
            delivery_rate: stats.delivery_rate,
            open_rate: stats.open_rate,
            click_rate: stats.click_rate,
            bounce_rate: stats.bounce_rate,
            spam_rate: stats.spam_rate,
        }
    }

    /// Get suppression list
    pub async fn suppression_list(&self) -> Vec<SuppressionEntry> {
        self.log_service.get_suppression_list().await
            .into_iter()
            .map(|(email, reason)| SuppressionEntry {
                email,
                reason: format!("{:?}", reason),
            })
            .collect()
    }

    /// Check if email is suppressed
    pub async fn is_suppressed(&self, email: &str) -> bool {
        self.log_service.is_suppressed(email).await
    }

    /// Add to suppression list
    pub async fn suppress(&self, email: &str) {
        self.log_service.add_to_suppression(email, crate::services::log::SuppressionReason::Manual).await;
    }

    /// Remove from suppression list
    pub async fn unsuppress(&self, email: &str) {
        self.log_service.remove_from_suppression(email).await;
    }

    /// Export logs
    pub async fn export(&self, query: LogQuery) -> String {
        let filter = LogFilter {
            email_id: query.email_id.and_then(|s| Uuid::parse_str(&s).ok()),
            recipient: query.recipient,
            event: query.event.and_then(|e| Self::parse_event(&e)),
            template_id: query.template_id.and_then(|s| Uuid::parse_str(&s).ok()),
            provider: query.provider,
            from_date: query.from_date.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
            to_date: query.to_date.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
            errors_only: query.errors_only.unwrap_or(false),
            limit: query.limit.unwrap_or(10000),
            offset: query.offset.unwrap_or(0),
        };

        self.log_service.export(filter).await
    }

    /// Clean up old logs
    pub async fn cleanup(&self, days: i64) -> usize {
        let duration = chrono::Duration::days(days);
        self.log_service.cleanup(duration).await
    }

    fn parse_event(s: &str) -> Option<EmailEvent> {
        match s.to_lowercase().as_str() {
            "queued" => Some(EmailEvent::Queued),
            "sent" => Some(EmailEvent::Sent),
            "delivered" => Some(EmailEvent::Delivered),
            "bounced" => Some(EmailEvent::Bounced),
            "soft_bounce" => Some(EmailEvent::SoftBounce),
            "hard_bounce" => Some(EmailEvent::HardBounce),
            "opened" => Some(EmailEvent::Opened),
            "clicked" => Some(EmailEvent::Clicked),
            "spam" | "spam_complaint" => Some(EmailEvent::SpamComplaint),
            "unsubscribed" => Some(EmailEvent::Unsubscribed),
            "failed" => Some(EmailEvent::Failed),
            "deferred" => Some(EmailEvent::Deferred),
            "cancelled" => Some(EmailEvent::Cancelled),
            _ => None,
        }
    }

    fn to_response(entry: &EmailLog) -> LogEntryResponse {
        LogEntryResponse {
            id: entry.id.to_string(),
            email_id: entry.email_id.to_string(),
            queue_id: entry.queue_id.map(|id| id.to_string()),
            event: format!("{}", entry.event),
            recipient: entry.recipient.clone(),
            subject: entry.subject.clone(),
            template_name: entry.template_name.clone(),
            timestamp: entry.timestamp.to_rfc3339(),
            provider: entry.provider.clone(),
            provider_message_id: entry.provider_message_id.clone(),
            error: entry.error.clone(),
            ip_address: entry.ip_address.clone(),
            user_agent: entry.user_agent.clone(),
            click_url: entry.click_url.clone(),
        }
    }
}
