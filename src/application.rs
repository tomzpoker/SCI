use crate::domain::{DashboardSnapshot, TaskItem, TaskState};
use chrono::{Duration, Utc};

pub struct AnticipationEngine;

impl AnticipationEngine {
    pub fn build_priority_queue(mut tasks: Vec<TaskItem>) -> Vec<TaskItem> {
        tasks.sort_by_key(|t| {
            let state_rank = match t.state {
                TaskState::Blocked => 0,
                TaskState::Ready => 1,
                TaskState::Running => 2,
                TaskState::Planned => 3,
                TaskState::Done | TaskState::Skipped => 9,
            };
            (state_rank, -t.priority, t.due_at)
        });
        tasks
    }

    pub fn horizon_label(days: i64) -> String {
        let now = Utc::now();
        format!(
            "{} → {}",
            now.date_naive(),
            (now + Duration::days(days.max(1))).date_naive()
        )
    }

    pub fn derive_risk(snapshot: &DashboardSnapshot) -> String {
        if snapshot.overdue_tasks > 0 || snapshot.forecast_min_cash_cents < 0 {
            "CRITICAL"
        } else if snapshot.tasks_due_30d > 8
            || snapshot.vat_to_prepare_cents > snapshot.cash_cents / 3
        {
            "WATCH"
        } else {
            "NORMAL"
        }
        .into()
    }
}
