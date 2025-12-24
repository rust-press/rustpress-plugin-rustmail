//! Queue Handler

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{QueueItem, QueueStatus, QueueStats};
use crate::services::QueueService;

#[derive(Debug, Deserialize)]
pub struct QueueListQuery {
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueueItemResponse {
    pub id: String,
    pub email_id: String,
    pub subject: String,
    pub recipients: Vec<String>,
    pub status: String,
    pub attempts: u32,
    pub max_attempts: u32,
    pub last_error: Option<String>,
    pub scheduled_at: String,
    pub next_retry_at: Option<String>,
    pub created_at: String,
    pub priority: i32,
}

#[derive(Debug, Serialize)]
pub struct QueueStatsResponse {
    pub pending: u64,
    pub processing: u64,
    pub sent: u64,
    pub failed: u64,
    pub deferred: u64,
    pub success_rate: f64,
    pub throughput: f64,
}

/// Queue handler
pub struct QueueHandler {
    queue_service: Arc<QueueService>,
}

impl QueueHandler {
    pub fn new(queue_service: Arc<QueueService>) -> Self {
        Self { queue_service }
    }

    /// List queue items
    pub async fn list(&self, query: QueueListQuery) -> Vec<QueueItemResponse> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        let items = if let Some(search) = query.search {
            self.queue_service.search(&search, limit).await
        } else if let Some(status_str) = query.status {
            let status = match status_str.to_lowercase().as_str() {
                "pending" => QueueStatus::Pending,
                "processing" => QueueStatus::Processing,
                "sent" => QueueStatus::Sent,
                "failed" => QueueStatus::Failed,
                "deferred" => QueueStatus::Deferred,
                "cancelled" => QueueStatus::Cancelled,
                _ => QueueStatus::Pending,
            };
            self.queue_service.list_by_status(status, limit, offset).await
        } else {
            self.queue_service.get_pending(limit).await
        };

        items.into_iter().map(|i| Self::to_response(&i)).collect()
    }

    /// Get queue item
    pub async fn get(&self, id: &str) -> Result<QueueItemResponse, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;

        let item = self.queue_service.get(uuid).await
            .ok_or_else(|| "Queue item not found".to_string())?;

        Ok(Self::to_response(&item))
    }

    /// Cancel queue item
    pub async fn cancel(&self, id: &str) -> Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        self.queue_service.cancel(uuid).await.map_err(|e| e.to_string())
    }

    /// Retry queue item
    pub async fn retry(&self, id: &str) -> Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        self.queue_service.retry(uuid).await.map_err(|e| e.to_string())
    }

    /// Set priority
    pub async fn set_priority(&self, id: &str, priority: i32) -> Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        self.queue_service.set_priority(uuid, priority).await.map_err(|e| e.to_string())
    }

    /// Get queue statistics
    pub async fn stats(&self) -> QueueStatsResponse {
        let stats = self.queue_service.stats().await;

        QueueStatsResponse {
            pending: stats.pending,
            processing: stats.processing,
            sent: stats.sent,
            failed: stats.failed,
            deferred: stats.deferred,
            success_rate: stats.success_rate,
            throughput: stats.throughput,
        }
    }

    /// Get queue size
    pub async fn size(&self) -> usize {
        self.queue_service.size().await
    }

    /// Clean up old items
    pub async fn cleanup(&self, days: i64) -> usize {
        let duration = chrono::Duration::days(days);
        self.queue_service.cleanup(duration).await
    }

    fn to_response(item: &QueueItem) -> QueueItemResponse {
        QueueItemResponse {
            id: item.id.to_string(),
            email_id: item.email.id.to_string(),
            subject: item.email.subject.clone(),
            recipients: item.email.to.iter().map(|a| a.email.clone()).collect(),
            status: format!("{}", item.status),
            attempts: item.attempts,
            max_attempts: item.max_attempts,
            last_error: item.last_error.clone(),
            scheduled_at: item.scheduled_at.to_rfc3339(),
            next_retry_at: item.next_retry_at.map(|t| t.to_rfc3339()),
            created_at: item.created_at.to_rfc3339(),
            priority: item.priority,
        }
    }
}
