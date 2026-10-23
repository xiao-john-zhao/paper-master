use crate::error::{AppError, AppResult, ErrorCode};
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::{Semaphore, SemaphorePermit};
use tokio_util::sync::CancellationToken;

pub struct AIRequestRegistry {
    requests: Mutex<HashMap<String, CancellationToken>>,
    inference: Semaphore,
    progress_app: Mutex<Option<tauri::AppHandle>>,
}

impl AIRequestRegistry {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            inference: Semaphore::new(1),
            progress_app: Mutex::new(None),
        }
    }

    pub fn attach_progress(&self, app: tauri::AppHandle) {
        if let Ok(mut handle) = self.progress_app.lock() {
            *handle = Some(app);
        }
    }
    pub fn report(&self, event: crate::services::progress::TaskProgress) {
        if let Ok(handle) = self.progress_app.lock() {
            if let Some(app) = handle.as_ref() {
                crate::services::progress::emit(app, event);
            }
        }
    }

    pub async fn acquire(&self, token: &CancellationToken) -> AppResult<SemaphorePermit<'_>> {
        tokio::select! {
            biased;
            _ = token.cancelled() => Err(AppError::new(ErrorCode::AiCancelled, "AI 请求已取消。")),
            permit = self.inference.acquire() => permit.map_err(|_| AppError::new(ErrorCode::Internal, "AI 队列不可用。")),
        }
    }

    pub fn register(&self, request_id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        if let Ok(mut requests) = self.requests.lock() {
            requests.insert(request_id.to_string(), token.clone());
        }
        token
    }

    pub fn cancel(&self, request_id: &str) -> bool {
        self.requests
            .lock()
            .ok()
            .and_then(|requests| requests.get(request_id).cloned())
            .map(|token| {
                token.cancel();
                true
            })
            .unwrap_or(false)
    }

    pub fn remove(&self, request_id: &str) {
        if let Ok(mut requests) = self.requests.lock() {
            requests.remove(request_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_request_can_be_cancelled_and_removed() {
        let registry = AIRequestRegistry::new();
        let token = registry.register("analysis-test-001");

        assert!(registry.cancel("analysis-test-001"));
        assert!(token.is_cancelled());

        registry.remove("analysis-test-001");
        assert!(!registry.cancel("analysis-test-001"));
    }

    #[tokio::test]
    async fn cancellation_interrupts_a_request_waiting_for_inference() {
        let registry = AIRequestRegistry::new();
        let token = registry.register("running");
        let _permit = registry.acquire(&token).await.unwrap();
        let waiting = registry.register("waiting");
        assert!(registry.cancel("waiting"));
        assert_eq!(
            registry.acquire(&waiting).await.unwrap_err().code_string(),
            "AI_CANCELLED"
        );
    }
}
