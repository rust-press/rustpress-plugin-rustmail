//! Email Log Models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Email event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmailEvent {
    /// Email queued for sending
    Queued,
    /// Email sent successfully
    Sent,
    /// Email delivery confirmed
    Delivered,
    /// Email bounced
    Bounced,
    /// Soft bounce (temporary)
    SoftBounce,
    /// Hard bounce (permanent)
    HardBounce,
    /// Email opened
    Opened,
    /// Link clicked
    Clicked,
    /// Marked as spam
    SpamComplaint,
    /// Unsubscribed
    Unsubscribed,
    /// Failed to send
    Failed,
    /// Deferred for retry
    Deferred,
    /// Cancelled
    Cancelled,
}

impl std::fmt::Display for EmailEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "Queued"),
            Self::Sent => write!(f, "Sent"),
            Self::Delivered => write!(f, "Delivered"),
            Self::Bounced => write!(f, "Bounced"),
            Self::SoftBounce => write!(f, "Soft Bounce"),
            Self::HardBounce => write!(f, "Hard Bounce"),
            Self::Opened => write!(f, "Opened"),
            Self::Clicked => write!(f, "Clicked"),
            Self::SpamComplaint => write!(f, "Spam Complaint"),
            Self::Unsubscribed => write!(f, "Unsubscribed"),
            Self::Failed => write!(f, "Failed"),
            Self::Deferred => write!(f, "Deferred"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Email log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailLog {
    /// Log entry ID
    pub id: Uuid,
    /// Email ID
    pub email_id: Uuid,
    /// Queue item ID
    pub queue_id: Option<Uuid>,
    /// Event type
    pub event: EmailEvent,
    /// Recipient email
    pub recipient: String,
    /// Subject line
    pub subject: String,
    /// Template used
    pub template_id: Option<Uuid>,
    /// Template name
    pub template_name: Option<String>,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Message ID from provider
    pub provider_message_id: Option<String>,
    /// Provider name (smtp, ses, sendgrid, etc.)
    pub provider: String,
    /// Response from provider
    pub provider_response: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// IP address (for tracking)
    pub ip_address: Option<String>,
    /// User agent (for tracking)
    pub user_agent: Option<String>,
    /// Click URL (for click events)
    pub click_url: Option<String>,
    /// Metadata
    pub metadata: serde_json::Value,
}

impl EmailLog {
    pub fn new(email_id: Uuid, event: EmailEvent, recipient: &str, subject: &str) -> Self {
        Self {
            id: Uuid::now_v7(),
            email_id,
            queue_id: None,
            event,
            recipient: recipient.to_string(),
            subject: subject.to_string(),
            template_id: None,
            template_name: None,
            timestamp: Utc::now(),
            provider_message_id: None,
            provider: "smtp".to_string(),
            provider_response: None,
            error: None,
            ip_address: None,
            user_agent: None,
            click_url: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_provider(mut self, provider: &str, message_id: Option<&str>) -> Self {
        self.provider = provider.to_string();
        self.provider_message_id = message_id.map(|s| s.to_string());
        self
    }

    pub fn with_error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    pub fn with_queue(mut self, queue_id: Uuid) -> Self {
        self.queue_id = Some(queue_id);
        self
    }

    pub fn with_template(mut self, template_id: Uuid, template_name: &str) -> Self {
        self.template_id = Some(template_id);
        self.template_name = Some(template_name.to_string());
        self
    }

    pub fn with_tracking(mut self, ip: Option<&str>, user_agent: Option<&str>) -> Self {
        self.ip_address = ip.map(|s| s.to_string());
        self.user_agent = user_agent.map(|s| s.to_string());
        self
    }

    pub fn with_click(mut self, url: &str) -> Self {
        self.click_url = Some(url.to_string());
        self
    }
}

/// Log filter for queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogFilter {
    /// Filter by email ID
    pub email_id: Option<Uuid>,
    /// Filter by recipient
    pub recipient: Option<String>,
    /// Filter by event type
    pub event: Option<EmailEvent>,
    /// Filter by template
    pub template_id: Option<Uuid>,
    /// Filter by provider
    pub provider: Option<String>,
    /// Filter from date
    pub from_date: Option<DateTime<Utc>>,
    /// Filter to date
    pub to_date: Option<DateTime<Utc>>,
    /// Include errors only
    pub errors_only: bool,
    /// Pagination offset
    pub offset: u32,
    /// Page size
    pub limit: u32,
}

impl LogFilter {
    pub fn new() -> Self {
        Self {
            limit: 50,
            ..Default::default()
        }
    }

    pub fn for_email(email_id: Uuid) -> Self {
        Self {
            email_id: Some(email_id),
            limit: 100,
            ..Default::default()
        }
    }

    pub fn for_recipient(recipient: &str) -> Self {
        Self {
            recipient: Some(recipient.to_string()),
            limit: 100,
            ..Default::default()
        }
    }

    pub fn recent(limit: u32) -> Self {
        Self {
            limit,
            ..Default::default()
        }
    }

    pub fn errors() -> Self {
        Self {
            errors_only: true,
            limit: 100,
            ..Default::default()
        }
    }
}

/// Log statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogStats {
    /// Total emails sent
    pub total_sent: u64,
    /// Total delivered
    pub total_delivered: u64,
    /// Total bounced
    pub total_bounced: u64,
    /// Total opened
    pub total_opened: u64,
    /// Total clicked
    pub total_clicked: u64,
    /// Total spam complaints
    pub total_spam_complaints: u64,
    /// Total unsubscribes
    pub total_unsubscribes: u64,
    /// Total failed
    pub total_failed: u64,
    /// Delivery rate (percentage)
    pub delivery_rate: f64,
    /// Open rate (percentage)
    pub open_rate: f64,
    /// Click rate (percentage)
    pub click_rate: f64,
    /// Bounce rate (percentage)
    pub bounce_rate: f64,
    /// Spam rate (percentage)
    pub spam_rate: f64,
}

impl LogStats {
    pub fn calculate_rates(&mut self) {
        if self.total_sent > 0 {
            self.delivery_rate = (self.total_delivered as f64 / self.total_sent as f64) * 100.0;
            self.bounce_rate = (self.total_bounced as f64 / self.total_sent as f64) * 100.0;
            self.spam_rate = (self.total_spam_complaints as f64 / self.total_sent as f64) * 100.0;
        }

        if self.total_delivered > 0 {
            self.open_rate = (self.total_opened as f64 / self.total_delivered as f64) * 100.0;
        }

        if self.total_opened > 0 {
            self.click_rate = (self.total_clicked as f64 / self.total_opened as f64) * 100.0;
        }
    }
}

/// Bounce record for suppression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BounceRecord {
    /// Record ID
    pub id: Uuid,
    /// Email address that bounced
    pub email: String,
    /// Bounce type
    pub bounce_type: BounceType,
    /// Bounce reason
    pub reason: Option<String>,
    /// Provider diagnostic
    pub diagnostic: Option<String>,
    /// First bounce timestamp
    pub first_bounce: DateTime<Utc>,
    /// Last bounce timestamp
    pub last_bounce: DateTime<Utc>,
    /// Number of bounces
    pub bounce_count: u32,
    /// Whether address is suppressed
    pub suppressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BounceType {
    /// Hard bounce - permanent delivery failure
    Hard,
    /// Soft bounce - temporary delivery failure
    Soft,
    /// General/unknown
    General,
}

impl BounceRecord {
    pub fn new(email: &str, bounce_type: BounceType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            email: email.to_lowercase(),
            bounce_type,
            reason: None,
            diagnostic: None,
            first_bounce: now,
            last_bounce: now,
            bounce_count: 1,
            suppressed: bounce_type == BounceType::Hard,
        }
    }

    /// Record another bounce
    pub fn add_bounce(&mut self) {
        self.last_bounce = Utc::now();
        self.bounce_count += 1;

        // Auto-suppress after multiple soft bounces
        if self.bounce_type == BounceType::Soft && self.bounce_count >= 3 {
            self.suppressed = true;
        }
    }
}

/// Complaint record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplaintRecord {
    /// Record ID
    pub id: Uuid,
    /// Email address
    pub email: String,
    /// Complaint type
    pub complaint_type: ComplaintType,
    /// Original email ID
    pub email_id: Option<Uuid>,
    /// Feedback ID from ISP
    pub feedback_id: Option<String>,
    /// Complaint timestamp
    pub timestamp: DateTime<Utc>,
    /// User agent
    pub user_agent: Option<String>,
    /// Whether address is suppressed
    pub suppressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplaintType {
    /// Marked as spam
    Abuse,
    /// Authentication failure
    AuthFailure,
    /// Fraud report
    Fraud,
    /// Not spam (false positive)
    NotSpam,
    /// Other
    Other,
    /// Virus
    Virus,
}

impl ComplaintRecord {
    pub fn new(email: &str, complaint_type: ComplaintType) -> Self {
        Self {
            id: Uuid::now_v7(),
            email: email.to_lowercase(),
            complaint_type,
            email_id: None,
            feedback_id: None,
            timestamp: Utc::now(),
            user_agent: None,
            suppressed: complaint_type == ComplaintType::Abuse,
        }
    }
}
