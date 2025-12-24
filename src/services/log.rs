//! Email Log Service

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::{
    EmailLog, EmailEvent, LogFilter, LogStats,
    BounceRecord, BounceType, ComplaintRecord, ComplaintType,
};

/// Log service error
#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("Log entry not found: {0}")]
    NotFound(String),
    #[error("Storage error: {0}")]
    Storage(String),
}

/// Email log service
pub struct LogService {
    /// Log entries
    logs: Arc<RwLock<Vec<EmailLog>>>,
    /// Bounce records by email
    bounces: Arc<RwLock<HashMap<String, BounceRecord>>>,
    /// Complaint records by email
    complaints: Arc<RwLock<HashMap<String, ComplaintRecord>>>,
    /// Suppression list (emails that should not receive mail)
    suppression_list: Arc<RwLock<HashMap<String, SuppressionReason>>>,
    /// Max log entries to keep in memory
    max_entries: usize,
}

#[derive(Debug, Clone)]
pub enum SuppressionReason {
    HardBounce,
    SpamComplaint,
    Unsubscribed,
    Manual,
}

impl LogService {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
            bounces: Arc::new(RwLock::new(HashMap::new())),
            complaints: Arc::new(RwLock::new(HashMap::new())),
            suppression_list: Arc::new(RwLock::new(HashMap::new())),
            max_entries: 100_000,
        }
    }

    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Log an email event
    pub async fn log(&self, entry: EmailLog) {
        let mut logs = self.logs.write().await;

        // Handle special events
        match entry.event {
            EmailEvent::Bounced | EmailEvent::HardBounce | EmailEvent::SoftBounce => {
                self.record_bounce(&entry).await;
            }
            EmailEvent::SpamComplaint => {
                self.record_complaint(&entry).await;
            }
            EmailEvent::Unsubscribed => {
                self.add_to_suppression(&entry.recipient, SuppressionReason::Unsubscribed).await;
            }
            _ => {}
        }

        logs.push(entry);

        // Trim if over limit
        if logs.len() > self.max_entries {
            let remove_count = logs.len() - self.max_entries;
            logs.drain(0..remove_count);
        }
    }

    /// Log email queued
    pub async fn log_queued(&self, email_id: Uuid, recipient: &str, subject: &str) {
        let entry = EmailLog::new(email_id, EmailEvent::Queued, recipient, subject);
        self.log(entry).await;
    }

    /// Log email sent
    pub async fn log_sent(&self, email_id: Uuid, recipient: &str, subject: &str, provider: &str, message_id: Option<&str>) {
        let entry = EmailLog::new(email_id, EmailEvent::Sent, recipient, subject)
            .with_provider(provider, message_id);
        self.log(entry).await;
    }

    /// Log email failed
    pub async fn log_failed(&self, email_id: Uuid, recipient: &str, subject: &str, error: &str) {
        let entry = EmailLog::new(email_id, EmailEvent::Failed, recipient, subject)
            .with_error(error);
        self.log(entry).await;
    }

    /// Log email opened
    pub async fn log_opened(&self, email_id: Uuid, recipient: &str, ip: Option<&str>, user_agent: Option<&str>) {
        let entry = EmailLog::new(email_id, EmailEvent::Opened, recipient, "")
            .with_tracking(ip, user_agent);
        self.log(entry).await;
    }

    /// Log link clicked
    pub async fn log_clicked(&self, email_id: Uuid, recipient: &str, url: &str, ip: Option<&str>, user_agent: Option<&str>) {
        let entry = EmailLog::new(email_id, EmailEvent::Clicked, recipient, "")
            .with_click(url)
            .with_tracking(ip, user_agent);
        self.log(entry).await;
    }

    /// Get logs with filter
    pub async fn query(&self, filter: LogFilter) -> Vec<EmailLog> {
        let logs = self.logs.read().await;

        logs.iter()
            .filter(|log| {
                // Filter by email ID
                if let Some(email_id) = filter.email_id {
                    if log.email_id != email_id {
                        return false;
                    }
                }

                // Filter by recipient
                if let Some(ref recipient) = filter.recipient {
                    if !log.recipient.to_lowercase().contains(&recipient.to_lowercase()) {
                        return false;
                    }
                }

                // Filter by event
                if let Some(event) = filter.event {
                    if log.event != event {
                        return false;
                    }
                }

                // Filter by template
                if let Some(template_id) = filter.template_id {
                    if log.template_id != Some(template_id) {
                        return false;
                    }
                }

                // Filter by provider
                if let Some(ref provider) = filter.provider {
                    if &log.provider != provider {
                        return false;
                    }
                }

                // Filter by date range
                if let Some(from_date) = filter.from_date {
                    if log.timestamp < from_date {
                        return false;
                    }
                }

                if let Some(to_date) = filter.to_date {
                    if log.timestamp > to_date {
                        return false;
                    }
                }

                // Filter errors only
                if filter.errors_only && log.error.is_none() {
                    return false;
                }

                true
            })
            .skip(filter.offset as usize)
            .take(filter.limit as usize)
            .cloned()
            .collect()
    }

    /// Get logs for specific email
    pub async fn get_for_email(&self, email_id: Uuid) -> Vec<EmailLog> {
        self.query(LogFilter::for_email(email_id)).await
    }

    /// Get logs for recipient
    pub async fn get_for_recipient(&self, recipient: &str) -> Vec<EmailLog> {
        self.query(LogFilter::for_recipient(recipient)).await
    }

    /// Get recent logs
    pub async fn recent(&self, limit: u32) -> Vec<EmailLog> {
        let logs = self.logs.read().await;
        logs.iter()
            .rev()
            .take(limit as usize)
            .cloned()
            .collect()
    }

    /// Get statistics
    pub async fn stats(&self, from_date: Option<DateTime<Utc>>, to_date: Option<DateTime<Utc>>) -> LogStats {
        let logs = self.logs.read().await;
        let mut stats = LogStats::default();

        let from = from_date.unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
        let to = to_date.unwrap_or_else(Utc::now);

        for log in logs.iter() {
            if log.timestamp < from || log.timestamp > to {
                continue;
            }

            match log.event {
                EmailEvent::Sent => stats.total_sent += 1,
                EmailEvent::Delivered => stats.total_delivered += 1,
                EmailEvent::Bounced | EmailEvent::HardBounce | EmailEvent::SoftBounce => {
                    stats.total_bounced += 1;
                }
                EmailEvent::Opened => stats.total_opened += 1,
                EmailEvent::Clicked => stats.total_clicked += 1,
                EmailEvent::SpamComplaint => stats.total_spam_complaints += 1,
                EmailEvent::Unsubscribed => stats.total_unsubscribes += 1,
                EmailEvent::Failed => stats.total_failed += 1,
                _ => {}
            }
        }

        stats.calculate_rates();
        stats
    }

    /// Record a bounce
    async fn record_bounce(&self, log: &EmailLog) {
        let email = log.recipient.to_lowercase();
        let bounce_type = match log.event {
            EmailEvent::HardBounce => BounceType::Hard,
            EmailEvent::SoftBounce => BounceType::Soft,
            _ => BounceType::General,
        };

        let mut bounces = self.bounces.write().await;

        if let Some(record) = bounces.get_mut(&email) {
            record.add_bounce();
            if record.bounce_type == BounceType::Soft && bounce_type == BounceType::Hard {
                record.bounce_type = BounceType::Hard;
            }
            record.reason = log.error.clone();
        } else {
            let mut record = BounceRecord::new(&email, bounce_type);
            record.reason = log.error.clone();
            bounces.insert(email.clone(), record);
        }

        // Add hard bounces to suppression list
        if bounce_type == BounceType::Hard {
            self.add_to_suppression(&email, SuppressionReason::HardBounce).await;
        }
    }

    /// Record a complaint
    async fn record_complaint(&self, log: &EmailLog) {
        let email = log.recipient.to_lowercase();

        let mut complaints = self.complaints.write().await;

        let mut record = ComplaintRecord::new(&email, ComplaintType::Abuse);
        record.email_id = Some(log.email_id);
        record.user_agent = log.user_agent.clone();

        complaints.insert(email.clone(), record);

        // Add to suppression list
        self.add_to_suppression(&email, SuppressionReason::SpamComplaint).await;
    }

    /// Add email to suppression list
    pub async fn add_to_suppression(&self, email: &str, reason: SuppressionReason) {
        let mut list = self.suppression_list.write().await;
        list.insert(email.to_lowercase(), reason);
    }

    /// Remove from suppression list
    pub async fn remove_from_suppression(&self, email: &str) {
        let mut list = self.suppression_list.write().await;
        list.remove(&email.to_lowercase());
    }

    /// Check if email is suppressed
    pub async fn is_suppressed(&self, email: &str) -> bool {
        let list = self.suppression_list.read().await;
        list.contains_key(&email.to_lowercase())
    }

    /// Get suppression reason
    pub async fn get_suppression_reason(&self, email: &str) -> Option<SuppressionReason> {
        let list = self.suppression_list.read().await;
        list.get(&email.to_lowercase()).cloned()
    }

    /// Get bounce record
    pub async fn get_bounce(&self, email: &str) -> Option<BounceRecord> {
        let bounces = self.bounces.read().await;
        bounces.get(&email.to_lowercase()).cloned()
    }

    /// Get complaint record
    pub async fn get_complaint(&self, email: &str) -> Option<ComplaintRecord> {
        let complaints = self.complaints.read().await;
        complaints.get(&email.to_lowercase()).cloned()
    }

    /// Get all suppressed emails
    pub async fn get_suppression_list(&self) -> Vec<(String, SuppressionReason)> {
        let list = self.suppression_list.read().await;
        list.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Count logs by event type
    pub async fn count_by_event(&self) -> HashMap<EmailEvent, u64> {
        let logs = self.logs.read().await;
        let mut counts = HashMap::new();

        for log in logs.iter() {
            *counts.entry(log.event).or_insert(0) += 1;
        }

        counts
    }

    /// Clear old logs
    pub async fn cleanup(&self, older_than: chrono::Duration) -> usize {
        let mut logs = self.logs.write().await;
        let cutoff = Utc::now() - older_than;
        let original_len = logs.len();

        logs.retain(|log| log.timestamp > cutoff);

        original_len - logs.len()
    }

    /// Export logs to JSON
    pub async fn export(&self, filter: LogFilter) -> String {
        let logs = self.query(filter).await;
        serde_json::to_string_pretty(&logs).unwrap_or_default()
    }
}

impl Default for LogService {
    fn default() -> Self {
        Self::new()
    }
}
