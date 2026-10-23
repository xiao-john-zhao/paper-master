use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
pub struct TaskProgress {
    pub id: String,
    pub kind: String,
    pub stage: String,
    pub status: String,
    pub completed: Option<u64>,
    pub total: Option<u64>,
    pub unit: Option<String>,
}
impl TaskProgress {
    pub fn running(id: &str, kind: &str, stage: &str) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            stage: stage.into(),
            status: "running".into(),
            completed: None,
            total: None,
            unit: None,
        }
    }
    pub fn counts(mut self, completed: u64, total: Option<u64>, unit: &str) -> Self {
        self.completed = Some(completed);
        self.total = total;
        self.unit = Some(unit.into());
        self
    }
    pub fn finish(mut self, status: &str) -> Self {
        self.status = status.into();
        self
    }
}
pub fn emit(app: &AppHandle, progress: TaskProgress) {
    let _ = app.emit_to("main", "task-progress", progress);
}
