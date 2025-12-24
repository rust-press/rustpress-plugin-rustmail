//! Email Queue Service

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::{
    Email, QueueItem, QueueStatus, QueueStats,
    BatchSendRequest, BatchSendResult, BatchError, RetryPolicy,
};

/// Queue service error
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Item not found: {0}")]
    NotFound(String),
    #[error("Queue full")]
    QueueFull,
    #[error("Invalid operation: {0}")]
    Invalid(String),
}

/// Queue service
pub struct QueueService {
    /// Queue items
    items: Arc<RwLock<HashMap<Uuid, QueueItem>>>,
    /// Retry policy
    retry_policy: RetryPolicy,
    /// Maximum queue size
    max_size: usize,
}

impl QueueService {
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(HashMap::new())),
            retry_policy: RetryPolicy::default(),
            max_size: 100_000,
        }
    }

    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Add email to queue
    pub async fn enqueue(&self, email: Email) -> Result<QueueItem, QueueError> {
        let items = self.items.read().await;
        if items.len() >= self.max_size {
            return Err(QueueError::QueueFull);
        }
        drop(items);

        let item = QueueItem::new(email)
            .with_max_attempts(self.retry_policy.max_attempts);

        let mut items = self.items.write().await;
        items.insert(item.id, item.clone());

        Ok(item)
    }

    /// Schedule email for later
    pub async fn schedule(&self, email: Email, send_at: DateTime<Utc>) -> Result<QueueItem, QueueError> {
        let items = self.items.read().await;
        if items.len() >= self.max_size {
            return Err(QueueError::QueueFull);
        }
        drop(items);

        let item = QueueItem::scheduled(email, send_at)
            .with_max_attempts(self.retry_policy.max_attempts);

        let mut items = self.items.write().await;
        items.insert(item.id, item.clone());

        Ok(item)
    }

    /// Add batch of emails
    pub async fn enqueue_batch(&self, request: BatchSendRequest) -> BatchSendResult {
        let mut queued = 0;
        let mut failed = 0;
        let mut queue_ids = Vec::new();
        let mut errors = Vec::new();

        for (index, mut email) in request.emails.into_iter().enumerate() {
            // Apply tags
            email.tags.extend(request.tags.clone());

            let result = if let Some(scheduled_at) = request.scheduled_at {
                self.schedule(email, scheduled_at).await
            } else {
                self.enqueue(email).await
            };

            match result {
                Ok(mut item) => {
                    if let Some(priority) = request.priority {
                        item.priority = priority;
                    }
                    if let Some(max) = request.max_attempts {
                        item.max_attempts = max;
                    }
                    queue_ids.push(item.id);
                    queued += 1;
                }
                Err(e) => {
                    errors.push(BatchError {
                        index,
                        message: e.to_string(),
                    });
                    failed += 1;
                }
            }
        }

        BatchSendResult {
            queued,
            failed,
            queue_ids,
            errors,
        }
    }

    /// Get item by ID
    pub async fn get(&self, id: Uuid) -> Option<QueueItem> {
        let items = self.items.read().await;
        items.get(&id).cloned()
    }

    /// Get next items to process
    pub async fn get_pending(&self, limit: usize) -> Vec<QueueItem> {
        let items = self.items.read().await;
        let now = Utc::now();

        let mut pending: Vec<_> = items.values()
            .filter(|item| {
                matches!(item.status, QueueStatus::Pending | QueueStatus::Deferred)
                    && item.scheduled_at <= now
                    && item.next_retry_at.map_or(true, |t| t <= now)
            })
            .cloned()
            .collect();

        // Sort by priority (descending) then scheduled time (ascending)
        pending.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then(a.scheduled_at.cmp(&b.scheduled_at))
        });

        pending.truncate(limit);
        pending
    }

    /// Claim item for processing
    pub async fn claim(&self, id: Uuid, worker_id: &str) -> Result<QueueItem, QueueError> {
        let mut items = self.items.write().await;

        let item = items.get_mut(&id)
            .ok_or_else(|| QueueError::NotFound(id.to_string()))?;

        if !matches!(item.status, QueueStatus::Pending | QueueStatus::Deferred) {
            return Err(QueueError::Invalid(format!("Item status is {:?}", item.status)));
        }

        item.start_processing(worker_id);
        Ok(item.clone())
    }

    /// Mark item as sent
    pub async fn mark_sent(&self, id: Uuid) -> Result<(), QueueError> {
        let mut items = self.items.write().await;

        let item = items.get_mut(&id)
            .ok_or_else(|| QueueError::NotFound(id.to_string()))?;

        item.mark_sent();
        Ok(())
    }

    /// Mark item as failed
    pub async fn mark_failed(&self, id: Uuid, error: &str) -> Result<(), QueueError> {
        let mut items = self.items.write().await;

        let item = items.get_mut(&id)
            .ok_or_else(|| QueueError::NotFound(id.to_string()))?;

        item.mark_failed(error);
        Ok(())
    }

    /// Cancel item
    pub async fn cancel(&self, id: Uuid) -> Result<(), QueueError> {
        let mut items = self.items.write().await;

        let item = items.get_mut(&id)
            .ok_or_else(|| QueueError::NotFound(id.to_string()))?;

        if matches!(item.status, QueueStatus::Sent) {
            return Err(QueueError::Invalid("Cannot cancel sent item".to_string()));
        }

        item.cancel();
        Ok(())
    }

    /// Retry a failed item
    pub async fn retry(&self, id: Uuid) -> Result<(), QueueError> {
        let mut items = self.items.write().await;

        let item = items.get_mut(&id)
            .ok_or_else(|| QueueError::NotFound(id.to_string()))?;

        if !matches!(item.status, QueueStatus::Failed | QueueStatus::Cancelled) {
            return Err(QueueError::Invalid("Item must be failed or cancelled".to_string()));
        }

        item.status = QueueStatus::Pending;
        item.attempts = 0;
        item.last_error = None;
        item.next_retry_at = None;
        item.scheduled_at = Utc::now();

        Ok(())
    }

    /// Get queue statistics
    pub async fn stats(&self) -> QueueStats {
        let items = self.items.read().await;
        let now = Utc::now();
        let day_ago = now - chrono::Duration::hours(24);

        let mut stats = QueueStats::default();

        for item in items.values() {
            match item.status {
                QueueStatus::Pending => stats.pending += 1,
                QueueStatus::Processing => stats.processing += 1,
                QueueStatus::Sent => {
                    if item.completed_at.map_or(false, |t| t > day_ago) {
                        stats.sent += 1;
                    }
                }
                QueueStatus::Failed => {
                    if item.completed_at.map_or(false, |t| t > day_ago) {
                        stats.failed += 1;
                    }
                }
                QueueStatus::Deferred => stats.deferred += 1,
                QueueStatus::Cancelled => {}
            }
        }

        // Calculate rates
        let total = stats.sent + stats.failed;
        if total > 0 {
            stats.success_rate = (stats.sent as f64 / total as f64) * 100.0;
        }

        stats
    }

    /// List items by status
    pub async fn list_by_status(&self, status: QueueStatus, limit: usize, offset: usize) -> Vec<QueueItem> {
        let items = self.items.read().await;

        items.values()
            .filter(|item| item.status == status)
            .skip(offset)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Search items
    pub async fn search(&self, query: &str, limit: usize) -> Vec<QueueItem> {
        let items = self.items.read().await;
        let query_lower = query.to_lowercase();

        items.values()
            .filter(|item| {
                item.email.subject.to_lowercase().contains(&query_lower)
                    || item.email.to.iter().any(|a| a.email.to_lowercase().contains(&query_lower))
            })
            .take(limit)
            .cloned()
            .collect()
    }

    /// Clear completed items older than duration
    pub async fn cleanup(&self, older_than: chrono::Duration) -> usize {
        let mut items = self.items.write().await;
        let cutoff = Utc::now() - older_than;

        let to_remove: Vec<Uuid> = items.iter()
            .filter(|(_, item)| {
                matches!(item.status, QueueStatus::Sent | QueueStatus::Failed | QueueStatus::Cancelled)
                    && item.completed_at.map_or(false, |t| t < cutoff)
            })
            .map(|(id, _)| *id)
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            items.remove(&id);
        }

        count
    }

    /// Get retry policy
    pub fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }

    /// Update item priority
    pub async fn set_priority(&self, id: Uuid, priority: i32) -> Result<(), QueueError> {
        let mut items = self.items.write().await;

        let item = items.get_mut(&id)
            .ok_or_else(|| QueueError::NotFound(id.to_string()))?;

        item.priority = priority;
        Ok(())
    }

    /// Get queue size
    pub async fn size(&self) -> usize {
        let items = self.items.read().await;
        items.len()
    }

    /// Check if queue has capacity
    pub async fn has_capacity(&self, count: usize) -> bool {
        let items = self.items.read().await;
        items.len() + count <= self.max_size
    }
}

impl Default for QueueService {
    fn default() -> Self {
        Self::new()
    }
}
