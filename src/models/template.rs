//! Email Template Models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// Template type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TemplateType {
    /// System template (password reset, etc.)
    System,
    /// Transactional template (order confirmation, etc.)
    #[default]
    Transactional,
    /// Marketing template (newsletters, promotions)
    Marketing,
    /// Notification template
    Notification,
    /// Custom template
    Custom,
}

impl std::fmt::Display for TemplateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "System"),
            Self::Transactional => write!(f, "Transactional"),
            Self::Marketing => write!(f, "Marketing"),
            Self::Notification => write!(f, "Notification"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

/// Template variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    /// Variable name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Default value
    pub default: Option<String>,
    /// Whether required
    pub required: bool,
    /// Example value
    pub example: Option<String>,
    /// Variable type hint
    pub var_type: VariableType,
}

/// Variable type for validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VariableType {
    #[default]
    String,
    Number,
    Boolean,
    Date,
    Url,
    Email,
    Html,
    Array,
    Object,
}

/// Email template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplate {
    /// Unique identifier
    pub id: Uuid,
    /// Template name/key
    pub name: String,
    /// Template slug for URL/lookup
    pub slug: String,
    /// Human-readable title
    pub title: String,
    /// Description
    pub description: Option<String>,
    /// Template type
    pub template_type: TemplateType,
    /// Subject line template
    pub subject: String,
    /// Plain text body template
    pub text_body: Option<String>,
    /// HTML body template
    pub html_body: Option<String>,
    /// Preheader text (email preview)
    pub preheader: Option<String>,
    /// Parent layout template ID
    pub layout_id: Option<Uuid>,
    /// Variable definitions
    pub variables: Vec<TemplateVariable>,
    /// Default sender address
    pub default_from: Option<String>,
    /// Default reply-to address
    pub default_reply_to: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Whether template is active
    pub active: bool,
    /// Version number
    pub version: u32,
    /// Created by user ID
    pub created_by: Option<Uuid>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl EmailTemplate {
    pub fn new(name: &str, subject: &str) -> Self {
        let slug = slugify(name);
        Self {
            id: Uuid::now_v7(),
            name: name.to_string(),
            slug,
            title: name.to_string(),
            description: None,
            template_type: TemplateType::default(),
            subject: subject.to_string(),
            text_body: None,
            html_body: None,
            preheader: None,
            layout_id: None,
            variables: vec![],
            default_from: None,
            default_reply_to: None,
            tags: vec![],
            active: true,
            version: 1,
            created_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_bodies(mut self, text: Option<&str>, html: Option<&str>) -> Self {
        self.text_body = text.map(|s| s.to_string());
        self.html_body = html.map(|s| s.to_string());
        self
    }

    pub fn with_type(mut self, template_type: TemplateType) -> Self {
        self.template_type = template_type;
        self
    }

    pub fn add_variable(mut self, variable: TemplateVariable) -> Self {
        self.variables.push(variable);
        self
    }

    /// Extract variables from template content
    pub fn extract_variables(&self) -> Vec<String> {
        let mut vars = Vec::new();
        let re = regex::Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}\}").unwrap();

        let content = format!(
            "{} {} {}",
            self.subject,
            self.text_body.as_deref().unwrap_or(""),
            self.html_body.as_deref().unwrap_or("")
        );

        for cap in re.captures_iter(&content) {
            let var_name = cap[1].to_string();
            if !vars.contains(&var_name) {
                vars.push(var_name);
            }
        }

        vars
    }

    /// Check if all required variables are provided
    pub fn validate_data(&self, data: &serde_json::Value) -> Vec<String> {
        let mut missing = Vec::new();

        for var in &self.variables {
            if var.required {
                if let serde_json::Value::Object(map) = data {
                    if !map.contains_key(&var.name) {
                        missing.push(var.name.clone());
                    }
                } else {
                    missing.push(var.name.clone());
                }
            }
        }

        missing
    }
}

/// Email layout template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailLayout {
    /// Unique identifier
    pub id: Uuid,
    /// Layout name
    pub name: String,
    /// Layout slug
    pub slug: String,
    /// HTML template with {{{content}}} placeholder
    pub html: String,
    /// Plain text template with {{{content}}} placeholder
    pub text: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Whether this is the default layout
    pub is_default: bool,
    /// Active status
    pub active: bool,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl EmailLayout {
    pub fn new(name: &str, html: &str) -> Self {
        Self {
            id: Uuid::now_v7(),
            name: name.to_string(),
            slug: slugify(name),
            html: html.to_string(),
            text: None,
            description: None,
            is_default: false,
            active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Apply layout to content
    pub fn apply_html(&self, content: &str) -> String {
        self.html.replace("{{{content}}}", content)
            .replace("{{content}}", content)
    }

    /// Apply layout to text content
    pub fn apply_text(&self, content: &str) -> String {
        match &self.text {
            Some(text) => text.replace("{{{content}}}", content)
                .replace("{{content}}", content),
            None => content.to_string(),
        }
    }
}

/// Template builder
#[derive(Debug, Default)]
pub struct TemplateBuilder {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    template_type: TemplateType,
    subject: Option<String>,
    text_body: Option<String>,
    html_body: Option<String>,
    preheader: Option<String>,
    layout_id: Option<Uuid>,
    variables: Vec<TemplateVariable>,
    default_from: Option<String>,
    default_reply_to: Option<String>,
    tags: Vec<String>,
}

impl TemplateBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn template_type(mut self, t: TemplateType) -> Self {
        self.template_type = t;
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

    pub fn preheader(mut self, text: &str) -> Self {
        self.preheader = Some(text.to_string());
        self
    }

    pub fn layout(mut self, layout_id: Uuid) -> Self {
        self.layout_id = Some(layout_id);
        self
    }

    pub fn variable(mut self, var: TemplateVariable) -> Self {
        self.variables.push(var);
        self
    }

    pub fn required_var(mut self, name: &str, description: &str) -> Self {
        self.variables.push(TemplateVariable {
            name: name.to_string(),
            description: Some(description.to_string()),
            default: None,
            required: true,
            example: None,
            var_type: VariableType::String,
        });
        self
    }

    pub fn optional_var(mut self, name: &str, default: &str) -> Self {
        self.variables.push(TemplateVariable {
            name: name.to_string(),
            description: None,
            default: Some(default.to_string()),
            required: false,
            example: None,
            var_type: VariableType::String,
        });
        self
    }

    pub fn from_address(mut self, from: &str) -> Self {
        self.default_from = Some(from.to_string());
        self
    }

    pub fn reply_to(mut self, reply_to: &str) -> Self {
        self.default_reply_to = Some(reply_to.to_string());
        self
    }

    pub fn tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    pub fn build(self) -> Result<EmailTemplate, String> {
        let name = self.name.ok_or("Template name is required")?;
        let subject = self.subject.ok_or("Subject is required")?;

        if self.text_body.is_none() && self.html_body.is_none() {
            return Err("Template must have a body (text or HTML)".to_string());
        }

        Ok(EmailTemplate {
            id: Uuid::now_v7(),
            slug: slugify(&name),
            name: name.clone(),
            title: self.title.unwrap_or(name),
            description: self.description,
            template_type: self.template_type,
            subject,
            text_body: self.text_body,
            html_body: self.html_body,
            preheader: self.preheader,
            layout_id: self.layout_id,
            variables: self.variables,
            default_from: self.default_from,
            default_reply_to: self.default_reply_to,
            tags: self.tags,
            active: true,
            version: 1,
            created_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
}

/// Convert string to slug
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
