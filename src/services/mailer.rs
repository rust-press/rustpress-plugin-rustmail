//! Mailer Service - Main email sending service

use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::models::{Email, EmailAddress, EmailBuilder, QueueItem, QueueStatus};
use crate::services::{
    SmtpTransport, SmtpConfig, SmtpError,
    TemplateService, QueueService, LogService,
    template::RenderedEmail,
};

/// Mailer error
#[derive(Debug, thiserror::Error)]
pub enum MailerError {
    #[error("SMTP error: {0}")]
    Smtp(#[from] SmtpError),
    #[error("Template error: {0}")]
    Template(#[from] crate::services::template::TemplateError),
    #[error("Queue error: {0}")]
    Queue(#[from] crate::services::queue::QueueError),
    #[error("Recipient suppressed: {0}")]
    Suppressed(String),
    #[error("Invalid email: {0}")]
    Invalid(String),
    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Mailer configuration
#[derive(Debug, Clone)]
pub struct MailerConfig {
    /// Default from address
    pub default_from: Option<EmailAddress>,
    /// Default reply-to address
    pub default_reply_to: Option<EmailAddress>,
    /// Site name for templates
    pub site_name: String,
    /// Site URL for templates
    pub site_url: String,
    /// Track opens
    pub track_opens: bool,
    /// Track clicks
    pub track_clicks: bool,
    /// Queue emails by default
    pub queue_by_default: bool,
}

impl Default for MailerConfig {
    fn default() -> Self {
        Self {
            default_from: None,
            default_reply_to: None,
            site_name: "RustPress".to_string(),
            site_url: "http://localhost".to_string(),
            track_opens: false,
            track_clicks: false,
            queue_by_default: true,
        }
    }
}

/// Main mailer service
pub struct MailerService {
    /// Configuration
    config: Arc<RwLock<MailerConfig>>,
    /// SMTP transport
    transport: Arc<RwLock<Option<SmtpTransport>>>,
    /// Template service
    template_service: Arc<TemplateService>,
    /// Queue service
    queue_service: Arc<QueueService>,
    /// Log service
    log_service: Arc<LogService>,
}

impl MailerService {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(MailerConfig::default())),
            transport: Arc::new(RwLock::new(None)),
            template_service: Arc::new(TemplateService::new()),
            queue_service: Arc::new(QueueService::new()),
            log_service: Arc::new(LogService::new()),
        }
    }

    /// Configure mailer
    pub async fn configure(&self, config: MailerConfig) {
        let mut current = self.config.write().await;
        *current = config;
    }

    /// Configure SMTP
    pub async fn configure_smtp(&self, smtp_config: SmtpConfig) -> Result<(), MailerError> {
        let mut transport = SmtpTransport::new(smtp_config);
        transport.connect().await?;

        let mut current = self.transport.write().await;
        *current = Some(transport);

        Ok(())
    }

    /// Get template service
    pub fn templates(&self) -> &Arc<TemplateService> {
        &self.template_service
    }

    /// Get queue service
    pub fn queue(&self) -> &Arc<QueueService> {
        &self.queue_service
    }

    /// Get log service
    pub fn logs(&self) -> &Arc<LogService> {
        &self.log_service
    }

    /// Send email immediately
    pub async fn send(&self, email: Email) -> Result<(), MailerError> {
        // Check suppression
        for recipient in email.to.iter().chain(email.cc.iter()).chain(email.bcc.iter()) {
            if self.log_service.is_suppressed(&recipient.email).await {
                return Err(MailerError::Suppressed(recipient.email.clone()));
            }
        }

        let transport = self.transport.read().await;
        let transport = transport.as_ref()
            .ok_or_else(|| MailerError::Configuration("SMTP not configured".to_string()))?;

        // Log send attempt
        for recipient in &email.to {
            self.log_service.log_queued(email.id, &recipient.email, &email.subject).await;
        }

        // Send
        let result = transport.send(&email).await;

        match result {
            Ok(send_result) => {
                for recipient in &email.to {
                    self.log_service.log_sent(
                        email.id,
                        &recipient.email,
                        &email.subject,
                        "smtp",
                        send_result.message_id.as_deref(),
                    ).await;
                }
                Ok(())
            }
            Err(e) => {
                for recipient in &email.to {
                    self.log_service.log_failed(
                        email.id,
                        &recipient.email,
                        &email.subject,
                        &e.to_string(),
                    ).await;
                }
                Err(MailerError::Smtp(e))
            }
        }
    }

    /// Queue email for sending
    pub async fn queue_email(&self, email: Email) -> Result<QueueItem, MailerError> {
        // Check suppression
        for recipient in email.to.iter().chain(email.cc.iter()).chain(email.bcc.iter()) {
            if self.log_service.is_suppressed(&recipient.email).await {
                return Err(MailerError::Suppressed(recipient.email.clone()));
            }
        }

        let item = self.queue_service.enqueue(email).await?;

        // Log
        for recipient in &item.email.to {
            self.log_service.log_queued(item.email.id, &recipient.email, &item.email.subject).await;
        }

        Ok(item)
    }

    /// Send or queue based on config
    pub async fn deliver(&self, email: Email) -> Result<(), MailerError> {
        let config = self.config.read().await;

        if config.queue_by_default {
            self.queue_email(email).await?;
            Ok(())
        } else {
            self.send(email).await
        }
    }

    /// Send email using template
    pub async fn send_template(
        &self,
        template_slug: &str,
        to: EmailAddress,
        data: serde_json::Value,
    ) -> Result<(), MailerError> {
        let config = self.config.read().await;

        let from = config.default_from.clone()
            .ok_or_else(|| MailerError::Configuration("Default from address not set".to_string()))?;

        let rendered = self.template_service.render_by_slug(template_slug, &data).await?;
        let email = self.template_service.build_email(rendered, from, to);

        drop(config);
        self.deliver(email).await
    }

    /// Send email to multiple recipients using template
    pub async fn send_template_bulk(
        &self,
        template_slug: &str,
        recipients: Vec<(EmailAddress, serde_json::Value)>,
    ) -> Vec<Result<(), MailerError>> {
        let config = self.config.read().await;

        let from = match &config.default_from {
            Some(f) => f.clone(),
            None => {
                return vec![Err(MailerError::Configuration("Default from address not set".to_string()))];
            }
        };

        drop(config);

        let mut results = Vec::new();

        for (to, data) in recipients {
            let result = async {
                let rendered = self.template_service.render_by_slug(template_slug, &data).await?;
                let email = self.template_service.build_email(rendered, from.clone(), to);
                self.deliver(email).await
            }.await;

            results.push(result);
        }

        results
    }

    /// Process queue (call this periodically)
    pub async fn process_queue(&self, batch_size: usize) -> ProcessResult {
        let items = self.queue_service.get_pending(batch_size).await;

        let mut sent = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for item in items {
            // Claim item
            let claimed = match self.queue_service.claim(item.id, "worker").await {
                Ok(item) => item,
                Err(e) => {
                    errors.push((item.id, e.to_string()));
                    failed += 1;
                    continue;
                }
            };

            // Send
            match self.send(claimed.email.clone()).await {
                Ok(()) => {
                    let _ = self.queue_service.mark_sent(item.id).await;
                    sent += 1;
                }
                Err(e) => {
                    let _ = self.queue_service.mark_failed(item.id, &e.to_string()).await;
                    errors.push((item.id, e.to_string()));
                    failed += 1;
                }
            }
        }

        ProcessResult { sent, failed, errors }
    }

    /// Create an email builder with defaults
    pub async fn builder(&self) -> EmailBuilder {
        let config = self.config.read().await;

        let mut builder = EmailBuilder::new();

        if let Some(from) = &config.default_from {
            builder = builder.from(from.clone());
        }

        if let Some(reply_to) = &config.default_reply_to {
            builder = builder.reply_to(reply_to.clone());
        }

        builder
    }

    /// Quick send (simple API)
    pub async fn quick_send(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), MailerError> {
        let config = self.config.read().await;

        let from = config.default_from.clone()
            .ok_or_else(|| MailerError::Configuration("Default from address not set".to_string()))?;

        drop(config);

        let email = EmailBuilder::new()
            .from(from)
            .to(to)
            .subject(subject)
            .text(body)
            .build()
            .map_err(|e| MailerError::Invalid(e))?;

        self.deliver(email).await
    }

    /// Test connection
    pub async fn test_connection(&self) -> Result<bool, MailerError> {
        let transport = self.transport.read().await;
        let transport = transport.as_ref()
            .ok_or_else(|| MailerError::Configuration("SMTP not configured".to_string()))?;

        transport.test_connection().await.map_err(MailerError::Smtp)
    }

    /// Get statistics
    pub async fn stats(&self) -> MailerStats {
        let queue_stats = self.queue_service.stats().await;
        let log_stats = self.log_service.stats(None, None).await;

        MailerStats {
            queue_pending: queue_stats.pending,
            queue_processing: queue_stats.processing,
            queue_deferred: queue_stats.deferred,
            sent_24h: log_stats.total_sent,
            delivered_24h: log_stats.total_delivered,
            bounced_24h: log_stats.total_bounced,
            failed_24h: log_stats.total_failed,
            open_rate: log_stats.open_rate,
            click_rate: log_stats.click_rate,
            bounce_rate: log_stats.bounce_rate,
        }
    }

    /// Initialize with system templates
    pub async fn initialize(&self) {
        self.template_service.register_system_templates().await;
    }
}

impl Default for MailerService {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of queue processing
#[derive(Debug)]
pub struct ProcessResult {
    pub sent: usize,
    pub failed: usize,
    pub errors: Vec<(Uuid, String)>,
}

/// Mailer statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct MailerStats {
    pub queue_pending: u64,
    pub queue_processing: u64,
    pub queue_deferred: u64,
    pub sent_24h: u64,
    pub delivered_24h: u64,
    pub bounced_24h: u64,
    pub failed_24h: u64,
    pub open_rate: f64,
    pub click_rate: f64,
    pub bounce_rate: f64,
}
