//! Email Handler

use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::models::{Email, EmailAddress, EmailBuilder, EmailPriority, Attachment};
use crate::services::MailerService;

#[derive(Debug, Deserialize)]
pub struct SendEmailRequest {
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub reply_to: Option<String>,
    pub priority: Option<String>,
    pub tags: Option<Vec<String>>,
    pub attachments: Option<Vec<AttachmentData>>,
}

#[derive(Debug, Deserialize)]
pub struct AttachmentData {
    pub filename: String,
    pub content_type: String,
    pub content_base64: String,
}

#[derive(Debug, Deserialize)]
pub struct SendTemplateRequest {
    pub template: String,
    pub to: String,
    pub to_name: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct BulkTemplateRequest {
    pub template: String,
    pub recipients: Vec<BulkRecipient>,
}

#[derive(Debug, Deserialize)]
pub struct BulkRecipient {
    pub email: String,
    pub name: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SendResponse {
    pub success: bool,
    pub message: String,
    pub email_id: Option<String>,
    pub queue_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkSendResponse {
    pub total: usize,
    pub sent: usize,
    pub queued: usize,
    pub failed: usize,
    pub errors: Vec<BulkError>,
}

#[derive(Debug, Serialize)]
pub struct BulkError {
    pub index: usize,
    pub email: String,
    pub error: String,
}

/// Email handler
pub struct EmailHandler {
    mailer: Arc<MailerService>,
}

impl EmailHandler {
    pub fn new(mailer: Arc<MailerService>) -> Self {
        Self { mailer }
    }

    /// Send email
    pub async fn send(&self, request: SendEmailRequest) -> Result<SendResponse, String> {
        // Build email
        let mut builder = self.mailer.builder().await
            .subject(&request.subject);

        // Add recipients
        for to in &request.to {
            builder = builder.to(to.as_str());
        }

        if let Some(cc) = request.cc {
            for addr in cc {
                builder = builder.cc(addr.as_str());
            }
        }

        if let Some(bcc) = request.bcc {
            for addr in bcc {
                builder = builder.bcc(addr.as_str());
            }
        }

        // Body
        if let Some(text) = request.text_body {
            builder = builder.text(&text);
        }
        if let Some(html) = request.html_body {
            builder = builder.html(&html);
        }

        // Reply-to
        if let Some(reply_to) = request.reply_to {
            builder = builder.reply_to(reply_to.as_str());
        }

        // Priority
        if let Some(priority) = request.priority {
            let p = match priority.to_lowercase().as_str() {
                "low" => EmailPriority::Low,
                "high" => EmailPriority::High,
                "urgent" => EmailPriority::Urgent,
                _ => EmailPriority::Normal,
            };
            builder = builder.priority(p);
        }

        // Tags
        if let Some(tags) = request.tags {
            for tag in tags {
                builder = builder.tag(&tag);
            }
        }

        // Attachments
        if let Some(attachments) = request.attachments {
            for att in attachments {
                let content = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    &att.content_base64
                ).map_err(|e| format!("Invalid attachment encoding: {}", e))?;

                let attachment = Attachment::new(&att.filename, &att.content_type, content);
                builder = builder.attach(attachment);
            }
        }

        let email = builder.build().map_err(|e| e)?;
        let email_id = email.id.to_string();

        // Queue or send
        match self.mailer.queue_email(email).await {
            Ok(item) => Ok(SendResponse {
                success: true,
                message: "Email queued for delivery".to_string(),
                email_id: Some(email_id),
                queue_id: Some(item.id.to_string()),
            }),
            Err(e) => Ok(SendResponse {
                success: false,
                message: e.to_string(),
                email_id: Some(email_id),
                queue_id: None,
            }),
        }
    }

    /// Send using template
    pub async fn send_template(&self, request: SendTemplateRequest) -> Result<SendResponse, String> {
        let to = match request.to_name {
            Some(name) => EmailAddress::with_name(&request.to, &name),
            None => EmailAddress::new(&request.to),
        };

        match self.mailer.send_template(&request.template, to, request.data).await {
            Ok(()) => Ok(SendResponse {
                success: true,
                message: "Email sent/queued successfully".to_string(),
                email_id: None,
                queue_id: None,
            }),
            Err(e) => Ok(SendResponse {
                success: false,
                message: e.to_string(),
                email_id: None,
                queue_id: None,
            }),
        }
    }

    /// Send bulk using template
    pub async fn send_bulk(&self, request: BulkTemplateRequest) -> BulkSendResponse {
        let recipients: Vec<(EmailAddress, serde_json::Value)> = request.recipients
            .into_iter()
            .map(|r| {
                let addr = match r.name {
                    Some(name) => EmailAddress::with_name(&r.email, &name),
                    None => EmailAddress::new(&r.email),
                };
                (addr, r.data)
            })
            .collect();

        let total = recipients.len();
        let results = self.mailer.send_template_bulk(&request.template, recipients).await;

        let mut sent = 0;
        let mut queued = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for (index, result) in results.into_iter().enumerate() {
            match result {
                Ok(()) => {
                    queued += 1;
                }
                Err(e) => {
                    errors.push(BulkError {
                        index,
                        email: "unknown".to_string(),
                        error: e.to_string(),
                    });
                    failed += 1;
                }
            }
        }

        BulkSendResponse {
            total,
            sent,
            queued,
            failed,
            errors,
        }
    }

    /// Test email configuration
    pub async fn test(&self, to: &str) -> Result<SendResponse, String> {
        match self.mailer.quick_send(
            to,
            "Test Email from RustMail",
            "This is a test email to verify your email configuration is working correctly.",
        ).await {
            Ok(()) => Ok(SendResponse {
                success: true,
                message: "Test email sent successfully".to_string(),
                email_id: None,
                queue_id: None,
            }),
            Err(e) => Ok(SendResponse {
                success: false,
                message: e.to_string(),
                email_id: None,
                queue_id: None,
            }),
        }
    }
}
