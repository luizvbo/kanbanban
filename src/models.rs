use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataStore {
    pub boards: Vec<Board>,
    pub categories: Vec<Category>,
    pub audit_logs: Vec<LogEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: u64,
    pub name: String,
    pub icon: Option<String>,
    pub columns: Vec<Column>,
    // Logic Triggers
    pub reset_column_id: Option<u64>,
    pub start_column_id: Option<u64>,
    pub finish_column_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub description: Option<String>,
    pub category_id: Option<u64>,
    pub creation_date: NaiveDateTime,
    pub start_date: Option<NaiveDateTime>,
    pub finish_date: Option<NaiveDateTime>,
    pub due_date: Option<NaiveDateTime>,
}

impl Task {
    pub fn days_left(&self) -> Option<i64> {
        if let Some(due) = self.due_date {
            let now = Local::now().naive_local();
            let diff = due.date().signed_duration_since(now.date());
            Some(diff.num_days())
        } else {
            None
        }
    }

    pub fn days_since_creation(&self) -> i64 {
        let now = Local::now().naive_local();
        now.date()
            .signed_duration_since(self.creation_date.date())
            .num_days()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Category {
    pub id: u64,
    pub name: String,
    pub color: String, // Hex code
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub timestamp: NaiveDateTime,
    pub event_type: String, // CREATE, UPDATE, DELETE
    pub object_type: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Local};

    #[test]
    fn test_task_days_left_future() {
        let now = Local::now().naive_local();
        let due = now + Duration::days(5);
        let task = Task {
            due_date: Some(due),
            ..Default::default()
        };
        assert!(task.days_left().unwrap() >= 4);
    }

    #[test]
    fn test_task_days_left_overdue() {
        let now = Local::now().naive_local();
        let due = now - Duration::days(2);
        let task = Task {
            due_date: Some(due),
            ..Default::default()
        };
        assert!(task.days_left().unwrap() <= -1);
    }

    #[test]
    fn test_task_days_left_none() {
        let task = Task {
            due_date: None,
            ..Default::default()
        };
        assert!(task.days_left().is_none());
    }

    #[test]
    fn test_days_since_creation() {
        let now = Local::now().naive_local();
        let created = now - Duration::days(10);
        let task = Task {
            creation_date: created,
            ..Default::default()
        };
        assert_eq!(task.days_since_creation(), 10);
    }
}
