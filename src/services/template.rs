//! Template Service

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use handlebars::Handlebars;

use crate::models::{EmailTemplate, EmailLayout, Email, EmailAddress, TemplateBuilder};

/// Template service error
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    NotFound(String),
    #[error("Layout not found: {0}")]
    LayoutNotFound(String),
    #[error("Render error: {0}")]
    RenderError(String),
    #[error("Invalid template: {0}")]
    Invalid(String),
    #[error("Missing variable: {0}")]
    MissingVariable(String),
}

/// Template service
pub struct TemplateService {
    /// Templates by ID
    templates: Arc<RwLock<HashMap<Uuid, EmailTemplate>>>,
    /// Templates by slug (for lookup)
    templates_by_slug: Arc<RwLock<HashMap<String, Uuid>>>,
    /// Layouts
    layouts: Arc<RwLock<HashMap<Uuid, EmailLayout>>>,
    /// Default layout ID
    default_layout: Arc<RwLock<Option<Uuid>>>,
    /// Handlebars engine
    handlebars: Arc<RwLock<Handlebars<'static>>>,
}

impl TemplateService {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);

        // Register helpers
        Self::register_helpers(&mut handlebars);

        Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
            templates_by_slug: Arc::new(RwLock::new(HashMap::new())),
            layouts: Arc::new(RwLock::new(HashMap::new())),
            default_layout: Arc::new(RwLock::new(None)),
            handlebars: Arc::new(RwLock::new(handlebars)),
        }
    }

    fn register_helpers(handlebars: &mut Handlebars<'static>) {
        // Date formatting helper
        handlebars.register_helper(
            "date",
            Box::new(|h: &handlebars::Helper,
                      _: &Handlebars,
                      _: &handlebars::Context,
                      _: &mut handlebars::RenderContext,
                      out: &mut dyn handlebars::Output|
             -> handlebars::HelperResult {
                if let Some(param) = h.param(0) {
                    let format = h.param(1)
                        .and_then(|p| p.value().as_str())
                        .unwrap_or("%Y-%m-%d");

                    if let Some(date_str) = param.value().as_str() {
                        if let Ok(date) = chrono::DateTime::parse_from_rfc3339(date_str) {
                            out.write(&date.format(format).to_string())?;
                        } else {
                            out.write(date_str)?;
                        }
                    }
                }
                Ok(())
            }),
        );

        // Currency formatting helper
        handlebars.register_helper(
            "currency",
            Box::new(|h: &handlebars::Helper,
                      _: &Handlebars,
                      _: &handlebars::Context,
                      _: &mut handlebars::RenderContext,
                      out: &mut dyn handlebars::Output|
             -> handlebars::HelperResult {
                if let Some(param) = h.param(0) {
                    let symbol = h.param(1)
                        .and_then(|p| p.value().as_str())
                        .unwrap_or("$");

                    if let Some(amount) = param.value().as_f64() {
                        out.write(&format!("{}{:.2}", symbol, amount))?;
                    }
                }
                Ok(())
            }),
        );

        // Uppercase helper
        handlebars.register_helper(
            "uppercase",
            Box::new(|h: &handlebars::Helper,
                      _: &Handlebars,
                      _: &handlebars::Context,
                      _: &mut handlebars::RenderContext,
                      out: &mut dyn handlebars::Output|
             -> handlebars::HelperResult {
                if let Some(param) = h.param(0) {
                    if let Some(s) = param.value().as_str() {
                        out.write(&s.to_uppercase())?;
                    }
                }
                Ok(())
            }),
        );

        // Lowercase helper
        handlebars.register_helper(
            "lowercase",
            Box::new(|h: &handlebars::Helper,
                      _: &Handlebars,
                      _: &handlebars::Context,
                      _: &mut handlebars::RenderContext,
                      out: &mut dyn handlebars::Output|
             -> handlebars::HelperResult {
                if let Some(param) = h.param(0) {
                    if let Some(s) = param.value().as_str() {
                        out.write(&s.to_lowercase())?;
                    }
                }
                Ok(())
            }),
        );

        // Truncate helper
        handlebars.register_helper(
            "truncate",
            Box::new(|h: &handlebars::Helper,
                      _: &Handlebars,
                      _: &handlebars::Context,
                      _: &mut handlebars::RenderContext,
                      out: &mut dyn handlebars::Output|
             -> handlebars::HelperResult {
                if let Some(param) = h.param(0) {
                    let len = h.param(1)
                        .and_then(|p| p.value().as_u64())
                        .unwrap_or(50) as usize;

                    if let Some(s) = param.value().as_str() {
                        if s.len() > len {
                            out.write(&format!("{}...", &s[..len]))?;
                        } else {
                            out.write(s)?;
                        }
                    }
                }
                Ok(())
            }),
        );
    }

    /// Register a template
    pub async fn register(&self, template: EmailTemplate) -> Result<(), TemplateError> {
        // Validate template
        if template.text_body.is_none() && template.html_body.is_none() {
            return Err(TemplateError::Invalid("Template must have a body".to_string()));
        }

        let id = template.id;
        let slug = template.slug.clone();

        let mut templates = self.templates.write().await;
        let mut by_slug = self.templates_by_slug.write().await;

        templates.insert(id, template);
        by_slug.insert(slug, id);

        Ok(())
    }

    /// Get template by ID
    pub async fn get(&self, id: Uuid) -> Option<EmailTemplate> {
        let templates = self.templates.read().await;
        templates.get(&id).cloned()
    }

    /// Get template by slug
    pub async fn get_by_slug(&self, slug: &str) -> Option<EmailTemplate> {
        let by_slug = self.templates_by_slug.read().await;
        if let Some(id) = by_slug.get(slug) {
            let templates = self.templates.read().await;
            return templates.get(id).cloned();
        }
        None
    }

    /// List all templates
    pub async fn list(&self) -> Vec<EmailTemplate> {
        let templates = self.templates.read().await;
        templates.values().cloned().collect()
    }

    /// Delete template
    pub async fn delete(&self, id: Uuid) -> Result<(), TemplateError> {
        let mut templates = self.templates.write().await;
        let mut by_slug = self.templates_by_slug.write().await;

        if let Some(template) = templates.remove(&id) {
            by_slug.remove(&template.slug);
            Ok(())
        } else {
            Err(TemplateError::NotFound(id.to_string()))
        }
    }

    /// Register a layout
    pub async fn register_layout(&self, layout: EmailLayout) {
        let id = layout.id;
        let is_default = layout.is_default;

        let mut layouts = self.layouts.write().await;
        layouts.insert(id, layout);

        if is_default {
            let mut default = self.default_layout.write().await;
            *default = Some(id);
        }
    }

    /// Get layout by ID
    pub async fn get_layout(&self, id: Uuid) -> Option<EmailLayout> {
        let layouts = self.layouts.read().await;
        layouts.get(&id).cloned()
    }

    /// Render a template with data
    pub async fn render(
        &self,
        template_id: Uuid,
        data: &serde_json::Value,
    ) -> Result<RenderedEmail, TemplateError> {
        let template = self.get(template_id).await
            .ok_or_else(|| TemplateError::NotFound(template_id.to_string()))?;

        self.render_template(&template, data).await
    }

    /// Render a template by slug
    pub async fn render_by_slug(
        &self,
        slug: &str,
        data: &serde_json::Value,
    ) -> Result<RenderedEmail, TemplateError> {
        let template = self.get_by_slug(slug).await
            .ok_or_else(|| TemplateError::NotFound(slug.to_string()))?;

        self.render_template(&template, data).await
    }

    /// Render template
    async fn render_template(
        &self,
        template: &EmailTemplate,
        data: &serde_json::Value,
    ) -> Result<RenderedEmail, TemplateError> {
        // Check required variables
        let missing = template.validate_data(data);
        if !missing.is_empty() {
            return Err(TemplateError::MissingVariable(missing.join(", ")));
        }

        let handlebars = self.handlebars.read().await;

        // Render subject
        let subject = handlebars.render_template(&template.subject, data)
            .map_err(|e| TemplateError::RenderError(e.to_string()))?;

        // Render text body
        let text_body = if let Some(text) = &template.text_body {
            Some(handlebars.render_template(text, data)
                .map_err(|e| TemplateError::RenderError(e.to_string()))?)
        } else {
            None
        };

        // Render HTML body
        let mut html_body = if let Some(html) = &template.html_body {
            Some(handlebars.render_template(html, data)
                .map_err(|e| TemplateError::RenderError(e.to_string()))?)
        } else {
            None
        };

        // Apply layout if set
        if let Some(layout_id) = template.layout_id {
            if let Some(layout) = self.get_layout(layout_id).await {
                if let Some(html) = &html_body {
                    html_body = Some(layout.apply_html(html));
                }
            }
        } else {
            // Try default layout
            let default = self.default_layout.read().await;
            if let Some(layout_id) = *default {
                if let Some(layout) = self.get_layout(layout_id).await {
                    if let Some(html) = &html_body {
                        html_body = Some(layout.apply_html(html));
                    }
                }
            }
        }

        // Render preheader
        let preheader = if let Some(ph) = &template.preheader {
            Some(handlebars.render_template(ph, data)
                .map_err(|e| TemplateError::RenderError(e.to_string()))?)
        } else {
            None
        };

        Ok(RenderedEmail {
            template_id: template.id,
            template_name: template.name.clone(),
            subject,
            text_body,
            html_body,
            preheader,
        })
    }

    /// Build an email from a rendered template
    pub fn build_email(
        &self,
        rendered: RenderedEmail,
        from: EmailAddress,
        to: EmailAddress,
    ) -> Email {
        let mut email = Email::new(from, to, &rendered.subject);

        email.template_id = Some(rendered.template_id);

        if let Some(text) = rendered.text_body {
            email.text_body = Some(text);
        }

        if let Some(html) = rendered.html_body {
            // Insert preheader if present
            let final_html = if let Some(preheader) = rendered.preheader {
                format!(
                    r#"<div style="display:none;max-height:0;overflow:hidden;">{}</div>{}"#,
                    preheader, html
                )
            } else {
                html
            };
            email.html_body = Some(final_html);
        }

        email
    }

    /// Register system templates
    pub async fn register_system_templates(&self) {
        // Password reset template
        let password_reset = TemplateBuilder::new()
            .name("password-reset")
            .title("Password Reset")
            .template_type(crate::models::TemplateType::System)
            .subject("Reset Your Password")
            .required_var("reset_link", "Password reset URL")
            .required_var("user_name", "User's name")
            .optional_var("expiry_hours", "24")
            .text(r#"Hi {{user_name}},

You requested to reset your password. Click the link below to reset it:

{{reset_link}}

This link will expire in {{expiry_hours}} hours.

If you didn't request this, please ignore this email.

Thanks,
The Team"#)
            .html(r#"<!DOCTYPE html>
<html>
<head><title>Reset Your Password</title></head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2>Reset Your Password</h2>
        <p>Hi {{user_name}},</p>
        <p>You requested to reset your password. Click the button below to reset it:</p>
        <p style="text-align: center; margin: 30px 0;">
            <a href="{{reset_link}}" style="background: #2563eb; color: white; padding: 12px 24px; text-decoration: none; border-radius: 4px; display: inline-block;">Reset Password</a>
        </p>
        <p style="color: #666; font-size: 14px;">This link will expire in {{expiry_hours}} hours.</p>
        <p style="color: #666; font-size: 14px;">If you didn't request this, please ignore this email.</p>
        <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
        <p style="color: #999; font-size: 12px;">Thanks,<br>The Team</p>
    </div>
</body>
</html>"#)
            .build()
            .unwrap();

        let _ = self.register(password_reset).await;

        // Email verification template
        let email_verify = TemplateBuilder::new()
            .name("email-verification")
            .title("Email Verification")
            .template_type(crate::models::TemplateType::System)
            .subject("Verify Your Email Address")
            .required_var("verify_link", "Verification URL")
            .required_var("user_name", "User's name")
            .text(r#"Hi {{user_name}},

Please verify your email address by clicking the link below:

{{verify_link}}

Thanks,
The Team"#)
            .html(r#"<!DOCTYPE html>
<html>
<head><title>Verify Your Email</title></head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2>Verify Your Email Address</h2>
        <p>Hi {{user_name}},</p>
        <p>Please verify your email address by clicking the button below:</p>
        <p style="text-align: center; margin: 30px 0;">
            <a href="{{verify_link}}" style="background: #22c55e; color: white; padding: 12px 24px; text-decoration: none; border-radius: 4px; display: inline-block;">Verify Email</a>
        </p>
        <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
        <p style="color: #999; font-size: 12px;">Thanks,<br>The Team</p>
    </div>
</body>
</html>"#)
            .build()
            .unwrap();

        let _ = self.register(email_verify).await;

        // Welcome email template
        let welcome = TemplateBuilder::new()
            .name("welcome")
            .title("Welcome Email")
            .template_type(crate::models::TemplateType::Transactional)
            .subject("Welcome to {{site_name}}!")
            .required_var("user_name", "User's name")
            .required_var("site_name", "Site name")
            .optional_var("login_link", "/login")
            .text(r#"Welcome {{user_name}}!

Thanks for joining {{site_name}}. We're excited to have you.

Get started by logging in: {{login_link}}

Best regards,
The {{site_name}} Team"#)
            .html(r#"<!DOCTYPE html>
<html>
<head><title>Welcome!</title></head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h1 style="color: #2563eb;">Welcome to {{site_name}}!</h1>
        <p>Hi {{user_name}},</p>
        <p>Thanks for joining us. We're excited to have you!</p>
        <p style="text-align: center; margin: 30px 0;">
            <a href="{{login_link}}" style="background: #2563eb; color: white; padding: 12px 24px; text-decoration: none; border-radius: 4px; display: inline-block;">Get Started</a>
        </p>
        <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
        <p style="color: #999; font-size: 12px;">Best regards,<br>The {{site_name}} Team</p>
    </div>
</body>
</html>"#)
            .build()
            .unwrap();

        let _ = self.register(welcome).await;
    }
}

impl Default for TemplateService {
    fn default() -> Self {
        Self::new()
    }
}

/// Rendered email content
#[derive(Debug, Clone)]
pub struct RenderedEmail {
    pub template_id: Uuid,
    pub template_name: String,
    pub subject: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub preheader: Option<String>,
}
