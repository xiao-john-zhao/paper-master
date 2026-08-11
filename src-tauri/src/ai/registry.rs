use std::{collections::HashMap, sync::Mutex};
use tokio_util::sync::CancellationToken;

pub struct AIRequestRegistry {
    requests: Mutex<HashMap<String, CancellationToken>>,
}

impl AIRequestRegistry {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
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
}
