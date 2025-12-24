//! RustMail - Email Management Plugin for RustPress
//!
//! RustMail provides comprehensive email functionality including:
//!
//! - **SMTP Support**: Send emails via SMTP with TLS
//! - **Email Queue**: Queue emails for async delivery with retries
//! - **Templates**: Handlebars-based email templates with variables
//! - **Tracking**: Open and click tracking
//! - **Logging**: Complete email history and analytics
//! - **Suppression**: Automatic bounce and complaint handling
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rustmail::RustMailPlugin;
//!
//! #[tokio::main]
//! async fn main() {
//!     let plugin = RustMailPlugin::new();
//!
//!     // Configure SMTP (Gmail example)
//!     plugin.configure_gmail("user@gmail.com", "app-password").await.unwrap();
//!
//!     // Set default from address
//!     plugin.set_default_from("noreply@example.com", Some("My Site")).await;
//!
//!     // Initialize (registers system templates)
//!     plugin.initialize().await.unwrap();
//!
//!     // Send a quick email
//!     plugin.send("user@example.com", "Hello", "Welcome!").await.unwrap();
//! }
//! ```
//!
//! ## Using Templates
//!
//! ```rust,ignore
//! use rustmail::RustMailPlugin;
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() {
//!     let plugin = RustMailPlugin::new();
//!     plugin.initialize().await.unwrap();
//!
//!     // Send password reset email
//!     plugin.send_template(
//!         "password-reset",
//!         "user@example.com",
//!         json!({
//!             "user_name": "John",
//!             "reset_link": "https://example.com/reset?token=abc123"
//!         }),
//!     ).await.unwrap();
//! }
//! ```
//!
//! ## Queue Processing
//!
//! ```rust,ignore
//! use rustmail::RustMailPlugin;
//!
//! async fn process_emails(plugin: &RustMailPlugin) {
//!     // Process up to 100 queued emails
//!     let result = plugin.process_queue(100).await;
//!     println!("Sent: {}, Failed: {}", result.sent, result.failed);
//! }
//! ```

pub mod models;
pub mod services;
pub mod handlers;
pub mod plugin;

// Re-exports
pub use models::{
    Email, EmailAddress, EmailBuilder, EmailPriority, Attachment,
    EmailTemplate, TemplateType, TemplateVariable, TemplateBuilder,
    QueueItem, QueueStatus, QueueStats, RetryPolicy,
    EmailLog, EmailEvent, LogFilter, LogStats,
    BounceRecord, BounceType, ComplaintRecord,
};

pub use services::{
    MailerService, TemplateService, QueueService, LogService,
    SmtpTransport, SmtpConfig, TlsMode,
};

pub use handlers::{
    EmailHandler, TemplateHandler, QueueHandler, LogHandler,
};

pub use plugin::{RustMailPlugin, PluginInfo, plugin_info};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the plugin
pub fn init() -> RustMailPlugin {
    RustMailPlugin::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = RustMailPlugin::new();
        assert_eq!(plugin.name(), "RustMail");
    }

    #[test]
    fn test_email_address() {
        let addr = EmailAddress::new("test@example.com");
        assert_eq!(addr.email, "test@example.com");
        assert!(addr.name.is_none());

        let addr_with_name = EmailAddress::with_name("test@example.com", "Test User");
        assert_eq!(addr_with_name.formatted(), "Test User <test@example.com>");
    }

    #[test]
    fn test_email_builder() {
        let email = EmailBuilder::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test Subject")
            .text("Test body")
            .build()
            .unwrap();

        assert_eq!(email.subject, "Test Subject");
        assert_eq!(email.to.len(), 1);
        assert!(email.text_body.is_some());
    }

    #[test]
    fn test_email_builder_validation() {
        // Missing from
        let result = EmailBuilder::new()
            .to("test@example.com")
            .subject("Test")
            .text("Body")
            .build();
        assert!(result.is_err());

        // Missing recipient
        let result = EmailBuilder::new()
            .from("test@example.com")
            .subject("Test")
            .text("Body")
            .build();
        assert!(result.is_err());

        // Missing body
        let result = EmailBuilder::new()
            .from("test@example.com")
            .to("test@example.com")
            .subject("Test")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_email_priority() {
        assert_eq!(EmailPriority::Low.to_header_value(), "5");
        assert_eq!(EmailPriority::Normal.to_header_value(), "3");
        assert_eq!(EmailPriority::High.to_header_value(), "2");
        assert_eq!(EmailPriority::Urgent.to_header_value(), "1");
    }

    #[test]
    fn test_attachment() {
        let att = Attachment::new("test.txt", "text/plain", vec![72, 101, 108, 108, 111]);
        assert_eq!(att.filename, "test.txt");
        assert_eq!(att.size(), 5);
        assert!(!att.inline);
    }

    #[test]
    fn test_template_slugify() {
        use models::template::slugify;

        assert_eq!(slugify("Password Reset"), "password-reset");
        assert_eq!(slugify("Welcome Email!"), "welcome-email");
        assert_eq!(slugify("  Multiple   Spaces  "), "multiple-spaces");
    }

    #[tokio::test]
    async fn test_template_service() {
        let service = TemplateService::new();

        let template = TemplateBuilder::new()
            .name("test-template")
            .subject("Hello {{name}}")
            .text("Welcome, {{name}}!")
            .required_var("name", "User's name")
            .build()
            .unwrap();

        service.register(template).await.unwrap();

        let found = service.get_by_slug("test-template").await;
        assert!(found.is_some());

        let data = serde_json::json!({ "name": "John" });
        let rendered = service.render_by_slug("test-template", &data).await.unwrap();
        assert_eq!(rendered.subject, "Hello John");
        assert_eq!(rendered.text_body.unwrap(), "Welcome, John!");
    }

    #[tokio::test]
    async fn test_queue_service() {
        let service = QueueService::new();

        let email = EmailBuilder::new()
            .from("test@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .text("Body")
            .build()
            .unwrap();

        let item = service.enqueue(email).await.unwrap();
        assert_eq!(item.status, QueueStatus::Pending);

        let found = service.get(item.id).await;
        assert!(found.is_some());

        let pending = service.get_pending(10).await;
        assert!(!pending.is_empty());
    }

    #[tokio::test]
    async fn test_log_service() {
        let service = LogService::new();

        let email_id = uuid::Uuid::now_v7();
        service.log_sent(email_id, "test@example.com", "Test Subject", "smtp", None).await;

        let logs = service.recent(10).await;
        assert!(!logs.is_empty());

        let stats = service.stats(None, None).await;
        assert!(stats.total_sent > 0);
    }

    #[tokio::test]
    async fn test_suppression() {
        let service = LogService::new();

        assert!(!service.is_suppressed("test@example.com").await);

        service.add_to_suppression("test@example.com", crate::services::log::SuppressionReason::Manual).await;
        assert!(service.is_suppressed("test@example.com").await);

        service.remove_from_suppression("test@example.com").await;
        assert!(!service.is_suppressed("test@example.com").await);
    }

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::default();

        assert_eq!(policy.max_attempts, 3);
        assert!(policy.is_retryable("Connection timeout"));
        assert!(!policy.is_retryable("Invalid recipient"));

        let delay = policy.get_delay(0);
        assert_eq!(delay.num_seconds(), 60);

        let delay = policy.get_delay(1);
        assert_eq!(delay.num_seconds(), 120);
    }

    #[test]
    fn test_smtp_config() {
        let config = SmtpConfig::gmail("user@gmail.com", "password");
        assert_eq!(config.host, "smtp.gmail.com");
        assert_eq!(config.port, 587);

        let config = SmtpConfig::sendgrid("api-key");
        assert_eq!(config.host, "smtp.sendgrid.net");

        let config = SmtpConfig::ses("user", "pass", "us-east-1");
        assert!(config.host.contains("us-east-1"));
    }

    #[test]
    fn test_plugin_info() {
        let info = plugin_info();
        assert_eq!(info.name, "RustMail");
        assert!(!info.routes.is_empty());
        assert!(!info.hooks.is_empty());
    }
}
