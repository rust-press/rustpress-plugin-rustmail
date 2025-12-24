//! Email Models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// Email address with optional name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    /// Email address
    pub email: String,
    /// Display name (optional)
    pub name: Option<String>,
}

impl EmailAddress {
    pub fn new(email: &str) -> Self {
        Self {
            email: email.to_string(),
            name: None,
        }
    }

    pub fn with_name(email: &str, name: &str) -> Self {
        Self {
            email: email.to_string(),
            name: Some(name.to_string()),
        }
    }

    pub fn formatted(&self) -> String {
        match &self.name {
            Some(name) => format!("{} <{}>", name, self.email),
            None => self.email.clone(),
        }
    }
}

impl From<&str> for EmailAddress {
    fn from(email: &str) -> Self {
        Self::new(email)
    }
}

impl From<String> for EmailAddress {
    fn from(email: String) -> Self {
        Self::new(&email)
    }
}

/// Email attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Filename
    pub filename: String,
    /// MIME type
    pub content_type: String,
    /// Content (base64 encoded for serialization)
    pub content: Vec<u8>,
    /// Whether to embed inline
    pub inline: bool,
    /// Content ID for inline attachments
    pub content_id: Option<String>,
}

impl Attachment {
    pub fn new(filename: &str, content_type: &str, content: Vec<u8>) -> Self {
        Self {
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            content,
            inline: false,
            content_id: None,
        }
    }

    pub fn inline(filename: &str, content_type: &str, content: Vec<u8>, cid: &str) -> Self {
        Self {
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            content,
            inline: true,
            content_id: Some(cid.to_string()),
        }
    }

    pub fn from_file(path: &str) -> Result<Self, std::io::Error> {
        let content = std::fs::read(path)?;
        let filename = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment")
            .to_string();

        let content_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        Ok(Self {
            filename,
            content_type,
            content,
            inline: false,
            content_id: None,
        })
    }

    pub fn size(&self) -> usize {
        self.content.len()
    }
}

/// Email priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EmailPriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl EmailPriority {
    pub fn to_header_value(&self) -> &'static str {
        match self {
            Self::Low => "5",
            Self::Normal => "3",
            Self::High => "2",
            Self::Urgent => "1",
        }
    }
}

/// Email message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    /// Unique identifier
    pub id: Uuid,
    /// From address
    pub from: EmailAddress,
    /// Reply-to address
    pub reply_to: Option<EmailAddress>,
    /// To recipients
    pub to: Vec<EmailAddress>,
    /// CC recipients
    pub cc: Vec<EmailAddress>,
    /// BCC recipients
    pub bcc: Vec<EmailAddress>,
    /// Subject line
    pub subject: String,
    /// Plain text body
    pub text_body: Option<String>,
    /// HTML body
    pub html_body: Option<String>,
    /// Attachments
    pub attachments: Vec<Attachment>,
    /// Custom headers
    pub headers: HashMap<String, String>,
    /// Priority
    pub priority: EmailPriority,
    /// Template ID (if rendered from template)
    pub template_id: Option<Uuid>,
    /// Template variables used
    pub template_data: Option<serde_json::Value>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Metadata
    pub metadata: HashMap<String, String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl Email {
    pub fn new(from: EmailAddress, to: EmailAddress, subject: &str) -> Self {
        Self {
            id: Uuid::now_v7(),
            from,
            reply_to: None,
            to: vec![to],
            cc: vec![],
            bcc: vec![],
            subject: subject.to_string(),
            text_body: None,
            html_body: None,
            attachments: vec![],
            headers: HashMap::new(),
            priority: EmailPriority::Normal,
            template_id: None,
            template_data: None,
            tags: vec![],
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn reply_to(mut self, address: EmailAddress) -> Self {
        self.reply_to = Some(address);
        self
    }

    pub fn add_to(mut self, address: EmailAddress) -> Self {
        self.to.push(address);
        self
    }

    pub fn cc(mut self, address: EmailAddress) -> Self {
        self.cc.push(address);
        self
    }

    pub fn bcc(mut self, address: EmailAddress) -> Self {
        self.bcc.push(address);
        self
    }

    pub fn text(mut self, body: &str) -> Self {
        self.text_body = Some(body.to_string());
        self
    }

    pub fn html(mut self, body: &str) -> Self {
        self.html_body = Some(body.to_string());
        self
    }

    pub fn attach(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }

    pub fn priority(mut self, priority: EmailPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    pub fn meta(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Get total number of recipients
    pub fn recipient_count(&self) -> usize {
        self.to.len() + self.cc.len() + self.bcc.len()
    }

    /// Check if email has content
    pub fn has_body(&self) -> bool {
        self.text_body.is_some() || self.html_body.is_some()
    }

    /// Get total attachment size
    pub fn total_attachment_size(&self) -> usize {
        self.attachments.iter().map(|a| a.size()).sum()
    }
}

/// Email builder for fluent API
#[derive(Debug, Default)]
pub struct EmailBuilder {
    from: Option<EmailAddress>,
    reply_to: Option<EmailAddress>,
    to: Vec<EmailAddress>,
    cc: Vec<EmailAddress>,
    bcc: Vec<EmailAddress>,
    subject: Option<String>,
    text_body: Option<String>,
    html_body: Option<String>,
    attachments: Vec<Attachment>,
    headers: HashMap<String, String>,
    priority: EmailPriority,
    tags: Vec<String>,
    metadata: HashMap<String, String>,
}

impl EmailBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(mut self, address: impl Into<EmailAddress>) -> Self {
        self.from = Some(address.into());
        self
    }

    pub fn from_name(mut self, email: &str, name: &str) -> Self {
        self.from = Some(EmailAddress::with_name(email, name));
        self
    }

    pub fn reply_to(mut self, address: impl Into<EmailAddress>) -> Self {
        self.reply_to = Some(address.into());
        self
    }

    pub fn to(mut self, address: impl Into<EmailAddress>) -> Self {
        self.to.push(address.into());
        self
    }

    pub fn to_many(mut self, addresses: Vec<impl Into<EmailAddress>>) -> Self {
        self.to.extend(addresses.into_iter().map(|a| a.into()));
        self
    }

    pub fn cc(mut self, address: impl Into<EmailAddress>) -> Self {
        self.cc.push(address.into());
        self
    }

    pub fn bcc(mut self, address: impl Into<EmailAddress>) -> Self {
        self.bcc.push(address.into());
        self
    }

    pub fn subject(mut self, subject: &str) -> Self {
        self.subject = Some(subject.to_string());
        self
    }

    pub fn text(mut self, body: &str) -> Self {
        self.text_body = Some(body.to_string());
        self
    }

    pub fn html(mut self, body: &str) -> Self {
        self.html_body = Some(body.to_string());
        self
    }

    pub fn attach(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn attach_file(self, path: &str) -> Result<Self, std::io::Error> {
        let attachment = Attachment::from_file(path)?;
        Ok(self.attach(attachment))
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }

    pub fn priority(mut self, priority: EmailPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    pub fn meta(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn build(self) -> Result<Email, String> {
        let from = self.from.ok_or("From address is required")?;
        let subject = self.subject.ok_or("Subject is required")?;

        if self.to.is_empty() && self.cc.is_empty() && self.bcc.is_empty() {
            return Err("At least one recipient is required".to_string());
        }

        if self.text_body.is_none() && self.html_body.is_none() {
            return Err("Email must have a body (text or HTML)".to_string());
        }

        Ok(Email {
            id: Uuid::now_v7(),
            from,
            reply_to: self.reply_to,
            to: self.to,
            cc: self.cc,
            bcc: self.bcc,
            subject,
            text_body: self.text_body,
            html_body: self.html_body,
            attachments: self.attachments,
            headers: self.headers,
            priority: self.priority,
            template_id: None,
            template_data: None,
            tags: self.tags,
            metadata: self.metadata,
            created_at: Utc::now(),
        })
    }
}
