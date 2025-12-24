//! RustMail Plugin Entry Point

use std::sync::Arc;

use crate::models::EmailAddress;
use crate::services::{
    MailerService, TemplateService, QueueService, LogService,
    SmtpConfig, SmtpTransport,
    mailer::{MailerConfig, ProcessResult},
};
use crate::handlers::{EmailHandler, TemplateHandler, QueueHandler, LogHandler};

/// RustMail Plugin
pub struct RustMailPlugin {
    /// Mailer service
    mailer: Arc<MailerService>,
    /// Template service
    template_service: Arc<TemplateService>,
    /// Queue service
    queue_service: Arc<QueueService>,
    /// Log service
    log_service: Arc<LogService>,
    /// Email handler
    email_handler: EmailHandler,
    /// Template handler
    template_handler: TemplateHandler,
    /// Queue handler
    queue_handler: QueueHandler,
    /// Log handler
    log_handler: LogHandler,
}

impl RustMailPlugin {
    /// Create a new RustMail plugin instance
    pub fn new() -> Self {
        let mailer = Arc::new(MailerService::new());
        let template_service = Arc::clone(mailer.templates());
        let queue_service = Arc::clone(mailer.queue());
        let log_service = Arc::clone(mailer.logs());

        let email_handler = EmailHandler::new(Arc::clone(&mailer));
        let template_handler = TemplateHandler::new(Arc::clone(&template_service));
        let queue_handler = QueueHandler::new(Arc::clone(&queue_service));
        let log_handler = LogHandler::new(Arc::clone(&log_service));

        Self {
            mailer,
            template_service,
            queue_service,
            log_service,
            email_handler,
            template_handler,
            queue_handler,
            log_handler,
        }
    }

    /// Initialize the plugin
    pub async fn initialize(&self) -> Result<(), String> {
        // Register system templates
        self.mailer.initialize().await;
        Ok(())
    }

    /// Configure SMTP
    pub async fn configure_smtp(&self, config: SmtpConfig) -> Result<(), String> {
        self.mailer.configure_smtp(config).await.map_err(|e| e.to_string())
    }

    /// Configure with Gmail
    pub async fn configure_gmail(&self, username: &str, password: &str) -> Result<(), String> {
        self.configure_smtp(SmtpConfig::gmail(username, password)).await
    }

    /// Configure with SendGrid
    pub async fn configure_sendgrid(&self, api_key: &str) -> Result<(), String> {
        self.configure_smtp(SmtpConfig::sendgrid(api_key)).await
    }

    /// Configure with Amazon SES
    pub async fn configure_ses(&self, username: &str, password: &str, region: &str) -> Result<(), String> {
        self.configure_smtp(SmtpConfig::ses(username, password, region)).await
    }

    /// Set default from address
    pub async fn set_default_from(&self, email: &str, name: Option<&str>) {
        let address = match name {
            Some(n) => EmailAddress::with_name(email, n),
            None => EmailAddress::new(email),
        };

        let mut config = MailerConfig::default();
        config.default_from = Some(address);
        self.mailer.configure(config).await;
    }

    /// Get plugin name
    pub fn name(&self) -> &'static str {
        "RustMail"
    }

    /// Get plugin version
    pub fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Get plugin description
    pub fn description(&self) -> &'static str {
        "Email management for RustPress"
    }

    // Service accessors
    pub fn mailer(&self) -> &Arc<MailerService> {
        &self.mailer
    }

    pub fn templates(&self) -> &Arc<TemplateService> {
        &self.template_service
    }

    pub fn queue(&self) -> &Arc<QueueService> {
        &self.queue_service
    }

    pub fn logs(&self) -> &Arc<LogService> {
        &self.log_service
    }

    // Handler accessors
    pub fn email_handler(&self) -> &EmailHandler {
        &self.email_handler
    }

    pub fn template_handler(&self) -> &TemplateHandler {
        &self.template_handler
    }

    pub fn queue_handler(&self) -> &QueueHandler {
        &self.queue_handler
    }

    pub fn log_handler(&self) -> &LogHandler {
        &self.log_handler
    }

    // Convenience methods

    /// Send a quick email
    pub async fn send(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), String> {
        self.mailer.quick_send(to, subject, body).await.map_err(|e| e.to_string())
    }

    /// Send email using template
    pub async fn send_template(
        &self,
        template: &str,
        to: &str,
        data: serde_json::Value,
    ) -> Result<(), String> {
        self.mailer.send_template(template, EmailAddress::new(to), data)
            .await
            .map_err(|e| e.to_string())
    }

    /// Process the email queue
    pub async fn process_queue(&self, batch_size: usize) -> ProcessResult {
        self.mailer.process_queue(batch_size).await
    }

    /// Test email configuration
    pub async fn test_connection(&self) -> Result<bool, String> {
        self.mailer.test_connection().await.map_err(|e| e.to_string())
    }

    /// Get statistics
    pub async fn stats(&self) -> crate::services::mailer::MailerStats {
        self.mailer.stats().await
    }

    /// Check if email is suppressed
    pub async fn is_suppressed(&self, email: &str) -> bool {
        self.log_service.is_suppressed(email).await
    }
}

impl Default for RustMailPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin metadata for registration
pub fn plugin_info() -> PluginInfo {
    PluginInfo {
        name: "RustMail",
        version: env!("CARGO_PKG_VERSION"),
        description: "Email management for RustPress",
        author: "RustPress Team",
        homepage: "https://rustpress.dev/plugins/rustmail",
        license: "MIT",
        dependencies: vec![],
        hooks: vec![
            "email.send",
            "email.queued",
            "email.sent",
            "email.failed",
            "email.bounced",
            "email.opened",
            "email.clicked",
        ],
        routes: vec![
            "/admin/mail",
            "/admin/mail/compose",
            "/admin/mail/templates",
            "/admin/mail/queue",
            "/admin/mail/logs",
            "/admin/mail/settings",
            "/api/mail/send",
            "/api/mail/templates",
            "/api/mail/queue",
            "/api/mail/logs",
        ],
    }
}

/// Plugin information
#[derive(Debug)]
pub struct PluginInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub author: &'static str,
    pub homepage: &'static str,
    pub license: &'static str,
    pub dependencies: Vec<&'static str>,
    pub hooks: Vec<&'static str>,
    pub routes: Vec<&'static str>,
}
