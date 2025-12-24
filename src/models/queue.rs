//! Email Queue Models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Email;

/// Queue item status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QueueStatus {
    /// Waiting to be sent
    #[default]
    Pending,
    /// Currently being processed
    Processing,
    /// Successfully sent
    Sent,
    /// Failed to send
    Failed,
    /// Deferred for later retry
    Deferred,
    /// Cancelled
    Cancelled,
}

impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Processing => write!(f, "Processing"),
            Self::Sent => write!(f, "Sent"),
            Self::Failed => write!(f, "Failed"),
            Self::Deferred => write!(f, "Deferred"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Email queue item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    /// Queue item ID
    pub id: Uuid,
    /// Email to send
    pub email: Email,
    /// Current status
    pub status: QueueStatus,
    /// Number of send attempts
    pub attempts: u32,
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Last error message
    pub last_error: Option<String>,
    /// Scheduled send time
    pub scheduled_at: DateTime<Utc>,
    /// Next retry time
    pub next_retry_at: Option<DateTime<Utc>>,
    /// Processing started at
    pub started_at: Option<DateTime<Utc>>,
    /// Completed at (sent or failed)
    pub completed_at: Option<DateTime<Utc>>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Priority (higher = more important)
    pub priority: i32,
    /// Worker ID processing this item
    pub worker_id: Option<String>,
}

impl QueueItem {
    pub fn new(email: Email) -> Self {
        Self {
            id: Uuid::now_v7(),
            email,
            status: QueueStatus::Pending,
            attempts: 0,
            max_attempts: 3,
            last_error: None,
            scheduled_at: Utc::now(),
            next_retry_at: None,
            started_at: None,
            completed_at: None,
            created_at: Utc::now(),
            priority: 0,
            worker_id: None,
        }
    }

    pub fn scheduled(email: Email, send_at: DateTime<Utc>) -> Self {
        let mut item = Self::new(email);
        item.scheduled_at = send_at;
        item
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }

    /// Check if item is ready to process
    pub fn is_ready(&self) -> bool {
        matches!(self.status, QueueStatus::Pending | QueueStatus::Deferred)
            && self.scheduled_at <= Utc::now()
            && self.next_retry_at.map_or(true, |t| t <= Utc::now())
    }

    /// Check if can retry
    pub fn can_retry(&self) -> bool {
        self.attempts < self.max_attempts
    }

    /// Mark as processing
    pub fn start_processing(&mut self, worker_id: &str) {
        self.status = QueueStatus::Processing;
        self.started_at = Some(Utc::now());
        self.worker_id = Some(worker_id.to_string());
        self.attempts += 1;
    }

    /// Mark as sent
    pub fn mark_sent(&mut self) {
        self.status = QueueStatus::Sent;
        self.completed_at = Some(Utc::now());
        self.worker_id = None;
    }

    /// Mark as failed
    pub fn mark_failed(&mut self, error: &str) {
        self.last_error = Some(error.to_string());
        self.worker_id = None;

        if self.can_retry() {
            self.status = QueueStatus::Deferred;
            // Exponential backoff: 1min, 5min, 15min, etc.
            let delay = chrono::Duration::seconds(60 * (1 << self.attempts.min(5)));
            self.next_retry_at = Some(Utc::now() + delay);
        } else {
            self.status = QueueStatus::Failed;
            self.completed_at = Some(Utc::now());
        }
    }

    /// Cancel the queue item
    pub fn cancel(&mut self) {
        self.status = QueueStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }
}

/// Queue statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueueStats {
    /// Number of pending items
    pub pending: u64,
    /// Number of processing items
    pub processing: u64,
    /// Number of sent items (last 24h)
    pub sent: u64,
    /// Number of failed items (last 24h)
    pub failed: u64,
    /// Number of deferred items
    pub deferred: u64,
    /// Average send time in ms
    pub avg_send_time_ms: f64,
    /// Success rate (percentage)
    pub success_rate: f64,
    /// Items per hour throughput
    pub throughput: f64,
}

/// Batch send request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSendRequest {
    /// Emails to send
    pub emails: Vec<Email>,
    /// Schedule for specific time
    pub scheduled_at: Option<DateTime<Utc>>,
    /// Priority
    pub priority: Option<i32>,
    /// Tags to apply
    pub tags: Vec<String>,
    /// Maximum retries per email
    pub max_attempts: Option<u32>,
}

/// Batch send result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSendResult {
    /// Number of emails queued successfully
    pub queued: usize,
    /// Number of emails that failed to queue
    pub failed: usize,
    /// Queue item IDs
    pub queue_ids: Vec<Uuid>,
    /// Errors (if any)
    pub errors: Vec<BatchError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchError {
    /// Index in the batch
    pub index: usize,
    /// Error message
    pub message: String,
}

/// Retry policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum attempts
    pub max_attempts: u32,
    /// Initial delay in seconds
    pub initial_delay_secs: u64,
    /// Maximum delay in seconds
    pub max_delay_secs: u64,
    /// Multiplier for exponential backoff
    pub multiplier: f64,
    /// Errors to retry on
    pub retryable_errors: Vec<String>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_secs: 60,
            max_delay_secs: 3600,
            multiplier: 2.0,
            retryable_errors: vec![
                "connection".to_string(),
                "timeout".to_string(),
                "temporary".to_string(),
                "rate limit".to_string(),
            ],
        }
    }
}

impl RetryPolicy {
    /// Calculate delay for attempt number
    pub fn get_delay(&self, attempt: u32) -> chrono::Duration {
        let delay = (self.initial_delay_secs as f64 * self.multiplier.powi(attempt as i32)) as u64;
        let delay = delay.min(self.max_delay_secs);
        chrono::Duration::seconds(delay as i64)
    }

    /// Check if error is retryable
    pub fn is_retryable(&self, error: &str) -> bool {
        let error_lower = error.to_lowercase();
        self.retryable_errors.iter().any(|e| error_lower.contains(&e.to_lowercase()))
    }
}
