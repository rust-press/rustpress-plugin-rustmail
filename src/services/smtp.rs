//! SMTP Transport Service

use std::time::Duration;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{header::ContentType, Attachment as LettreAttachment, MultiPart, SinglePart},
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
};

use crate::models::{Email, Attachment, EmailPriority};

/// SMTP transport error
#[derive(Debug, thiserror::Error)]
pub enum SmtpError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Authentication error: {0}")]
    Authentication(String),
    #[error("Send error: {0}")]
    Send(String),
    #[error("Invalid email: {0}")]
    InvalidEmail(String),
    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// SMTP configuration
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    /// SMTP host
    pub host: String,
    /// SMTP port
    pub port: u16,
    /// Username
    pub username: Option<String>,
    /// Password
    pub password: Option<String>,
    /// Use TLS
    pub tls: TlsMode,
    /// Connection timeout
    pub timeout_secs: u64,
    /// Max connections in pool
    pub pool_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsMode {
    /// No TLS
    None,
    /// STARTTLS (upgrade connection)
    StartTls,
    /// TLS from start (implicit)
    Tls,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 25,
            username: None,
            password: None,
            tls: TlsMode::StartTls,
            timeout_secs: 30,
            pool_size: 10,
        }
    }
}

impl SmtpConfig {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            ..Default::default()
        }
    }

    pub fn with_credentials(mut self, username: &str, password: &str) -> Self {
        self.username = Some(username.to_string());
        self.password = Some(password.to_string());
        self
    }

    pub fn with_tls(mut self, mode: TlsMode) -> Self {
        self.tls = mode;
        self
    }

    /// Common configurations
    pub fn gmail(username: &str, password: &str) -> Self {
        Self::new("smtp.gmail.com", 587)
            .with_credentials(username, password)
            .with_tls(TlsMode::StartTls)
    }

    pub fn outlook(username: &str, password: &str) -> Self {
        Self::new("smtp.office365.com", 587)
            .with_credentials(username, password)
            .with_tls(TlsMode::StartTls)
    }

    pub fn sendgrid(api_key: &str) -> Self {
        Self::new("smtp.sendgrid.net", 587)
            .with_credentials("apikey", api_key)
            .with_tls(TlsMode::StartTls)
    }

    pub fn mailgun(username: &str, password: &str) -> Self {
        Self::new("smtp.mailgun.org", 587)
            .with_credentials(username, password)
            .with_tls(TlsMode::StartTls)
    }

    pub fn ses(username: &str, password: &str, region: &str) -> Self {
        Self::new(&format!("email-smtp.{}.amazonaws.com", region), 587)
            .with_credentials(username, password)
            .with_tls(TlsMode::StartTls)
    }
}

/// SMTP transport service
pub struct SmtpTransport {
    config: SmtpConfig,
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
}

impl SmtpTransport {
    pub fn new(config: SmtpConfig) -> Self {
        Self {
            config,
            transport: None,
        }
    }

    /// Connect to SMTP server
    pub async fn connect(&mut self) -> Result<(), SmtpError> {
        let builder = match self.config.tls {
            TlsMode::None => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.config.host)
            }
            TlsMode::StartTls => {
                let tls = TlsParameters::builder(self.config.host.clone())
                    .build()
                    .map_err(|e| SmtpError::Configuration(e.to_string()))?;

                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.host)
                    .map_err(|e| SmtpError::Connection(e.to_string()))?
                    .tls(Tls::Required(tls))
            }
            TlsMode::Tls => {
                AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.host)
                    .map_err(|e| SmtpError::Connection(e.to_string()))?
            }
        };

        let mut builder = builder.port(self.config.port);

        // Add credentials if provided
        if let (Some(username), Some(password)) = (&self.config.username, &self.config.password) {
            let creds = Credentials::new(username.clone(), password.clone());
            builder = builder.credentials(creds);
        }

        // Set timeout
        builder = builder.timeout(Some(Duration::from_secs(self.config.timeout_secs)));

        let transport = builder.build();

        // Test connection
        transport.test_connection().await
            .map_err(|e| SmtpError::Connection(e.to_string()))?;

        self.transport = Some(transport);
        Ok(())
    }

    /// Send an email
    pub async fn send(&self, email: &Email) -> Result<SendResult, SmtpError> {
        let transport = self.transport.as_ref()
            .ok_or_else(|| SmtpError::Connection("Not connected".to_string()))?;

        let message = self.build_message(email)?;

        let response = transport.send(message).await
            .map_err(|e| SmtpError::Send(e.to_string()))?;

        Ok(SendResult {
            message_id: response.message().map(|m| m.to_string()),
            code: response.code().as_str().to_string(),
            message: response.message().map(|m| m.to_string()),
        })
    }

    /// Build lettre Message from our Email
    fn build_message(&self, email: &Email) -> Result<Message, SmtpError> {
        let from_mailbox: lettre::message::Mailbox = email.from.formatted()
            .parse()
            .map_err(|e: lettre::address::AddressError| SmtpError::InvalidEmail(e.to_string()))?;

        let mut builder = Message::builder()
            .from(from_mailbox)
            .subject(&email.subject);

        // Add recipients
        for to in &email.to {
            let mailbox: lettre::message::Mailbox = to.formatted()
                .parse()
                .map_err(|e: lettre::address::AddressError| SmtpError::InvalidEmail(e.to_string()))?;
            builder = builder.to(mailbox);
        }

        for cc in &email.cc {
            let mailbox: lettre::message::Mailbox = cc.formatted()
                .parse()
                .map_err(|e: lettre::address::AddressError| SmtpError::InvalidEmail(e.to_string()))?;
            builder = builder.cc(mailbox);
        }

        for bcc in &email.bcc {
            let mailbox: lettre::message::Mailbox = bcc.formatted()
                .parse()
                .map_err(|e: lettre::address::AddressError| SmtpError::InvalidEmail(e.to_string()))?;
            builder = builder.bcc(mailbox);
        }

        // Reply-to
        if let Some(reply_to) = &email.reply_to {
            let mailbox: lettre::message::Mailbox = reply_to.formatted()
                .parse()
                .map_err(|e: lettre::address::AddressError| SmtpError::InvalidEmail(e.to_string()))?;
            builder = builder.reply_to(mailbox);
        }

        // Custom headers
        for (name, value) in &email.headers {
            builder = builder.header(lettre::message::header::HeaderName::new_from_ascii_str(name)
                .map_err(|e| SmtpError::InvalidEmail(e.to_string()))?
                .into_owned());
        }

        // Priority header
        if email.priority != EmailPriority::Normal {
            builder = builder.header(lettre::message::header::HeaderName::new_from_ascii_str("X-Priority")
                .map_err(|e| SmtpError::InvalidEmail(e.to_string()))?
                .into_owned());
        }

        // Build body
        let message = if !email.attachments.is_empty() {
            // Multipart with attachments
            let mut multipart = if email.html_body.is_some() && email.text_body.is_some() {
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .content_type(ContentType::TEXT_PLAIN)
                            .body(email.text_body.clone().unwrap_or_default())
                    )
                    .singlepart(
                        SinglePart::builder()
                            .content_type(ContentType::TEXT_HTML)
                            .body(email.html_body.clone().unwrap_or_default())
                    )
            } else if let Some(html) = &email.html_body {
                MultiPart::mixed()
                    .singlepart(
                        SinglePart::builder()
                            .content_type(ContentType::TEXT_HTML)
                            .body(html.clone())
                    )
            } else {
                MultiPart::mixed()
                    .singlepart(
                        SinglePart::builder()
                            .content_type(ContentType::TEXT_PLAIN)
                            .body(email.text_body.clone().unwrap_or_default())
                    )
            };

            // Add attachments
            let mut mixed = MultiPart::mixed().singlepart(
                SinglePart::builder()
                    .content_type(ContentType::TEXT_PLAIN)
                    .body(email.text_body.clone().unwrap_or_default())
            );

            for att in &email.attachments {
                let content_type = att.content_type.parse::<ContentType>()
                    .unwrap_or(ContentType::TEXT_PLAIN);

                let attachment = LettreAttachment::new(att.filename.clone())
                    .body(att.content.clone(), content_type);

                mixed = mixed.singlepart(attachment);
            }

            builder.multipart(mixed)
                .map_err(|e| SmtpError::InvalidEmail(e.to_string()))?
        } else if email.html_body.is_some() && email.text_body.is_some() {
            // Alternative multipart (text + HTML)
            let multipart = MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .content_type(ContentType::TEXT_PLAIN)
                        .body(email.text_body.clone().unwrap_or_default())
                )
                .singlepart(
                    SinglePart::builder()
                        .content_type(ContentType::TEXT_HTML)
                        .body(email.html_body.clone().unwrap_or_default())
                );

            builder.multipart(multipart)
                .map_err(|e| SmtpError::InvalidEmail(e.to_string()))?
        } else if let Some(html) = &email.html_body {
            builder.header(ContentType::TEXT_HTML)
                .body(html.clone())
                .map_err(|e| SmtpError::InvalidEmail(e.to_string()))?
        } else {
            builder.header(ContentType::TEXT_PLAIN)
                .body(email.text_body.clone().unwrap_or_default())
                .map_err(|e| SmtpError::InvalidEmail(e.to_string()))?
        };

        Ok(message)
    }

    /// Test connection
    pub async fn test_connection(&self) -> Result<bool, SmtpError> {
        let transport = self.transport.as_ref()
            .ok_or_else(|| SmtpError::Connection("Not connected".to_string()))?;

        transport.test_connection().await
            .map_err(|e| SmtpError::Connection(e.to_string()))
    }

    /// Get configuration
    pub fn config(&self) -> &SmtpConfig {
        &self.config
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.transport.is_some()
    }
}

/// Result of sending an email
#[derive(Debug, Clone)]
pub struct SendResult {
    /// Message ID assigned by server
    pub message_id: Option<String>,
    /// SMTP response code
    pub code: String,
    /// Response message
    pub message: Option<String>,
}

impl SendResult {
    pub fn is_success(&self) -> bool {
        self.code.starts_with('2')
    }
}
