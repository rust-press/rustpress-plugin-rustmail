//! Template Handler

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{EmailTemplate, TemplateType, TemplateVariable, VariableType};
use crate::services::TemplateService;

#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub template_type: Option<String>,
    pub subject: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub preheader: Option<String>,
    pub layout_id: Option<String>,
    pub variables: Option<Vec<VariableDefinition>>,
    pub default_from: Option<String>,
    pub default_reply_to: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct VariableDefinition {
    pub name: String,
    pub description: Option<String>,
    pub default: Option<String>,
    pub required: Option<bool>,
    pub var_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub subject: Option<String>,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub preheader: Option<String>,
    pub variables: Option<Vec<VariableDefinition>>,
    pub active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewRequest {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub title: String,
    pub description: Option<String>,
    pub template_type: String,
    pub subject: String,
    pub has_text: bool,
    pub has_html: bool,
    pub variables: Vec<String>,
    pub active: bool,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TemplateDetailResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub title: String,
    pub description: Option<String>,
    pub template_type: String,
    pub subject: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub preheader: Option<String>,
    pub layout_id: Option<String>,
    pub variables: Vec<TemplateVariable>,
    pub default_from: Option<String>,
    pub default_reply_to: Option<String>,
    pub tags: Vec<String>,
    pub active: bool,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    pub subject: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
}

/// Template handler
pub struct TemplateHandler {
    template_service: Arc<TemplateService>,
}

impl TemplateHandler {
    pub fn new(template_service: Arc<TemplateService>) -> Self {
        Self { template_service }
    }

    /// Create template
    pub async fn create(&self, request: CreateTemplateRequest) -> Result<TemplateResponse, String> {
        let template_type = request.template_type
            .map(|t| match t.to_lowercase().as_str() {
                "system" => TemplateType::System,
                "marketing" => TemplateType::Marketing,
                "notification" => TemplateType::Notification,
                "custom" => TemplateType::Custom,
                _ => TemplateType::Transactional,
            })
            .unwrap_or(TemplateType::Transactional);

        let variables: Vec<TemplateVariable> = request.variables
            .unwrap_or_default()
            .into_iter()
            .map(|v| TemplateVariable {
                name: v.name,
                description: v.description,
                default: v.default,
                required: v.required.unwrap_or(false),
                example: None,
                var_type: v.var_type
                    .map(|t| match t.to_lowercase().as_str() {
                        "number" => VariableType::Number,
                        "boolean" => VariableType::Boolean,
                        "date" => VariableType::Date,
                        "url" => VariableType::Url,
                        "email" => VariableType::Email,
                        "html" => VariableType::Html,
                        "array" => VariableType::Array,
                        "object" => VariableType::Object,
                        _ => VariableType::String,
                    })
                    .unwrap_or(VariableType::String),
            })
            .collect();

        let layout_id = request.layout_id
            .and_then(|s| Uuid::parse_str(&s).ok());

        let template = EmailTemplate {
            id: Uuid::now_v7(),
            name: request.name.clone(),
            slug: crate::models::template::slugify(&request.name),
            title: request.title.unwrap_or(request.name),
            description: request.description,
            template_type,
            subject: request.subject,
            text_body: request.text_body,
            html_body: request.html_body,
            preheader: request.preheader,
            layout_id,
            variables,
            default_from: request.default_from,
            default_reply_to: request.default_reply_to,
            tags: request.tags.unwrap_or_default(),
            active: true,
            version: 1,
            created_by: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.template_service.register(template.clone()).await
            .map_err(|e| e.to_string())?;

        Ok(Self::to_response(&template))
    }

    /// Get template by ID
    pub async fn get(&self, id: &str) -> Result<TemplateDetailResponse, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;

        let template = self.template_service.get(uuid).await
            .ok_or_else(|| "Template not found".to_string())?;

        Ok(Self::to_detail_response(&template))
    }

    /// Get template by slug
    pub async fn get_by_slug(&self, slug: &str) -> Result<TemplateDetailResponse, String> {
        let template = self.template_service.get_by_slug(slug).await
            .ok_or_else(|| "Template not found".to_string())?;

        Ok(Self::to_detail_response(&template))
    }

    /// List templates
    pub async fn list(&self) -> Vec<TemplateResponse> {
        self.template_service.list().await
            .into_iter()
            .map(|t| Self::to_response(&t))
            .collect()
    }

    /// Delete template
    pub async fn delete(&self, id: &str) -> Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        self.template_service.delete(uuid).await.map_err(|e| e.to_string())
    }

    /// Preview template
    pub async fn preview(&self, id: &str, request: PreviewRequest) -> Result<PreviewResponse, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;

        let rendered = self.template_service.render(uuid, &request.data).await
            .map_err(|e| e.to_string())?;

        Ok(PreviewResponse {
            subject: rendered.subject,
            text_body: rendered.text_body,
            html_body: rendered.html_body,
        })
    }

    /// Extract variables from template
    pub async fn extract_variables(&self, id: &str) -> Result<Vec<String>, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;

        let template = self.template_service.get(uuid).await
            .ok_or_else(|| "Template not found".to_string())?;

        Ok(template.extract_variables())
    }

    fn to_response(template: &EmailTemplate) -> TemplateResponse {
        TemplateResponse {
            id: template.id.to_string(),
            name: template.name.clone(),
            slug: template.slug.clone(),
            title: template.title.clone(),
            description: template.description.clone(),
            template_type: format!("{}", template.template_type),
            subject: template.subject.clone(),
            has_text: template.text_body.is_some(),
            has_html: template.html_body.is_some(),
            variables: template.variables.iter().map(|v| v.name.clone()).collect(),
            active: template.active,
            version: template.version,
            created_at: template.created_at.to_rfc3339(),
            updated_at: template.updated_at.to_rfc3339(),
        }
    }

    fn to_detail_response(template: &EmailTemplate) -> TemplateDetailResponse {
        TemplateDetailResponse {
            id: template.id.to_string(),
            name: template.name.clone(),
            slug: template.slug.clone(),
            title: template.title.clone(),
            description: template.description.clone(),
            template_type: format!("{}", template.template_type),
            subject: template.subject.clone(),
            text_body: template.text_body.clone(),
            html_body: template.html_body.clone(),
            preheader: template.preheader.clone(),
            layout_id: template.layout_id.map(|id| id.to_string()),
            variables: template.variables.clone(),
            default_from: template.default_from.clone(),
            default_reply_to: template.default_reply_to.clone(),
            tags: template.tags.clone(),
            active: template.active,
            version: template.version,
            created_at: template.created_at.to_rfc3339(),
            updated_at: template.updated_at.to_rfc3339(),
        }
    }
}
