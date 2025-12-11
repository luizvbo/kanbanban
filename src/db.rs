use crate::models::{Board, Category, Column, DataStore, LogEvent, Task};
use anyhow::Result;
use chrono::Local;
use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct Database {
    file_path: PathBuf,
    log_file_path: PathBuf,
    pub data: DataStore,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file_path = path.as_ref().to_path_buf();
        let file_stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("kanbanban");
        let log_file_name = format!("{file_stem}_audit.jsonl");
        let log_file_path = file_path.with_file_name(log_file_name);

        let mut data = if file_path.exists() {
            let content = fs::read_to_string(&file_path)?;
            if content.trim().is_empty() {
                DataStore::default()
            } else {
                serde_yaml::from_str(&content).unwrap_or_default()
            }
        } else {
            DataStore::default()
        };

        if data.boards.is_empty() {
            data.boards.push(Board {
                id: Self::gen_id(),
                name: "Kanban Board".to_string(),
                icon: Some("📝".to_string()),
                columns: vec![
                    Column {
                        id: Self::gen_id(),
                        name: "Ready".to_string(),
                        visible: true,
                        tasks: vec![],
                    },
                    Column {
                        id: Self::gen_id(),
                        name: "Doing".to_string(),
                        visible: true,
                        tasks: vec![],
                    },
                    Column {
                        id: Self::gen_id(),
                        name: "Done".to_string(),
                        visible: true,
                        tasks: vec![],
                    },
                ],
                reset_column_id: None,
                start_column_id: None,
                finish_column_id: None,
            });
        }

        let db = Self {
            file_path,
            log_file_path,
            data,
        };
        db.save()?;
        Ok(db)
    }

    pub fn save(&self) -> Result<()> {
        let content = serde_yaml::to_string(&self.data)?;
        fs::write(&self.file_path, content)?;
        Ok(())
    }

    pub fn gen_id() -> u64 {
        let start = SystemTime::now();
        let since_epoch = start.duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
        since_epoch.wrapping_add(ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    fn log(&mut self, event: &str, obj: &str, desc: &str) {
        let event = LogEvent {
            timestamp: Local::now().naive_local(),
            event_type: event.to_string(),
            object_type: obj.to_string(),
            description: desc.to_string(),
        };

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
            && let Ok(json_line) = serde_json::to_string(&event)
        {
            let _ = writeln!(file, "{json_line}");
        }
    }

    pub fn get_recent_logs(&self, limit: usize) -> Vec<LogEvent> {
        if !self.log_file_path.exists() {
            return vec![];
        }

        let file = match File::open(&self.log_file_path) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let reader = BufReader::new(file);
        let mut buffer: VecDeque<LogEvent> = VecDeque::with_capacity(limit);

        for line in reader.lines().map_while(Result::ok) {
            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<LogEvent>(&line) {
                if buffer.len() == limit {
                    buffer.pop_front();
                }
                buffer.push_back(event);
            }
        }

        let mut logs: Vec<LogEvent> = buffer.into();
        logs.reverse();
        logs
    }

    pub fn create_board(&mut self, name: String, icon: String) -> Result<()> {
        let board = Board {
            id: Self::gen_id(),
            name: name.clone(),
            icon: Some(icon),
            columns: vec![
                Column {
                    id: Self::gen_id(),
                    name: "Ready".to_string(),
                    visible: true,
                    tasks: vec![],
                },
                Column {
                    id: Self::gen_id(),
                    name: "Doing".to_string(),
                    visible: true,
                    tasks: vec![],
                },
                Column {
                    id: Self::gen_id(),
                    name: "Done".to_string(),
                    visible: true,
                    tasks: vec![],
                },
            ],
            reset_column_id: None,
            start_column_id: None,
            finish_column_id: None,
        };
        self.data.boards.push(board);
        self.log("CREATE", "Board", &format!("Created board '{name}'"));
        self.save()
    }

    pub fn update_board(&mut self, board_id: u64, name: String, icon: String) -> Result<()> {
        if let Some(board) = self.data.boards.iter_mut().find(|b| b.id == board_id) {
            board.name = name.clone();
            board.icon = Some(icon);
            self.log("UPDATE", "Board", &format!("Updated board '{name}'"));
            self.save()?;
        }
        Ok(())
    }

    pub fn delete_board(&mut self, board_id: u64) -> Result<()> {
        if let Some(idx) = self.data.boards.iter().position(|b| b.id == board_id) {
            let name = self.data.boards[idx].name.clone();
            self.data.boards.remove(idx);
            self.log("DELETE", "Board", &format!("Deleted board '{name}'"));
            self.save()?;
        }
        Ok(())
    }

    pub fn create_column(&mut self, name: String, board_id: u64) -> Result<()> {
        if let Some(board) = self.data.boards.iter_mut().find(|b| b.id == board_id) {
            let col = Column {
                id: Self::gen_id(),
                name: name.clone(),
                visible: true,
                tasks: vec![],
            };
            board.columns.push(col);
            self.log("CREATE", "Column", &format!("Created column '{name}'"));
            self.save()?;
        }
        Ok(())
    }

    pub fn update_column(&mut self, board_id: u64, column_id: u64, name: String) -> Result<()> {
        if let Some(board) = self.data.boards.iter_mut().find(|b| b.id == board_id)
            && let Some(col) = board.columns.iter_mut().find(|c| c.id == column_id)
        {
            col.name = name;
            self.save()?;
        }
        Ok(())
    }

    pub fn delete_column(&mut self, board_id: u64, column_id: u64) -> Result<()> {
        if let Some(board) = self.data.boards.iter_mut().find(|b| b.id == board_id)
            && let Some(idx) = board.columns.iter().position(|c| c.id == column_id)
        {
            let col_name = board.columns[idx].name.clone();
            if !board.columns[idx].tasks.is_empty() {
                return Err(anyhow::anyhow!("Cannot delete non-empty column"));
            }
            board.columns.remove(idx);
            self.log("DELETE", "Column", &format!("Deleted column '{col_name}'"));
            self.save()?;
        }
        Ok(())
    }

    pub fn update_column_visibility(
        &mut self,
        board_id: u64,
        column_id: u64,
        visible: bool,
    ) -> Result<()> {
        if let Some(board) = self.data.boards.iter_mut().find(|b| b.id == board_id)
            && let Some(col) = board.columns.iter_mut().find(|c| c.id == column_id)
        {
            col.visible = visible;
            self.save()?;
        }
        Ok(())
    }

    pub fn swap_column_positions(&mut self, board_id: u64, idx1: usize, idx2: usize) -> Result<()> {
        if let Some(board) = self.data.boards.iter_mut().find(|b| b.id == board_id)
            && idx1 < board.columns.len()
            && idx2 < board.columns.len()
        {
            board.columns.swap(idx1, idx2);
            self.save()?;
        }
        Ok(())
    }

    pub fn update_board_settings(
        &mut self,
        board_id: u64,
        start: Option<u64>,
        finish: Option<u64>,
        reset: Option<u64>,
    ) -> Result<()> {
        if let Some(board) = self.data.boards.iter_mut().find(|b| b.id == board_id) {
            board.start_column_id = start;
            board.finish_column_id = finish;
            board.reset_column_id = reset;
            self.save()?;
        }
        Ok(())
    }

    pub fn create_category(&mut self, name: String, color: String) -> Result<()> {
        let cat = Category {
            id: Self::gen_id(),
            name: name.clone(),
            color,
        };
        self.data.categories.push(cat);
        self.log("CREATE", "Category", &format!("Created category '{name}'"));
        self.save()
    }

    pub fn update_category(&mut self, id: u64, name: String, color: String) -> Result<()> {
        if let Some(cat) = self.data.categories.iter_mut().find(|c| c.id == id) {
            cat.name = name;
            cat.color = color;
            self.save()?;
        }
        Ok(())
    }

    pub fn delete_category(&mut self, id: u64) -> Result<()> {
        if let Some(idx) = self.data.categories.iter().position(|c| c.id == id) {
            self.data.categories.remove(idx);
            for board in &mut self.data.boards {
                for col in &mut board.columns {
                    for task in &mut col.tasks {
                        if task.category_id == Some(id) {
                            task.category_id = None;
                        }
                    }
                }
            }
            self.save()?;
        }
        Ok(())
    }

    pub fn create_task(
        &mut self,
        board_id: u64,
        col_id: u64,
        title: String,
        desc: String,
        due: Option<String>,
        cat_id: Option<u64>,
    ) -> Result<()> {
        if let Some(board) = self.data.boards.iter_mut().find(|b| b.id == board_id)
            && let Some(col) = board.columns.iter_mut().find(|c| c.id == col_id)
        {
            let now = Local::now().naive_local();
            let due_date = due
                .and_then(|d| chrono::NaiveDateTime::parse_from_str(&d, "%Y-%m-%d %H:%M:%S").ok());

            let task = Task {
                id: Self::gen_id(),
                title: title.clone(),
                description: Some(desc),
                category_id: cat_id,
                creation_date: now,
                start_date: None,
                finish_date: None,
                due_date,
            };
            col.tasks.push(task);
            self.log("CREATE", "Task", &format!("Created task '{title}'"));
            self.save()?;
        }
        Ok(())
    }

    pub fn update_task(
        &mut self,
        board_id: u64,
        task_id: u64,
        title: String,
        desc: String,
        due: Option<String>,
        cat_id: Option<u64>,
    ) -> Result<()> {
        if let Some(board) = self.data.boards.iter_mut().find(|b| b.id == board_id) {
            for col in &mut board.columns {
                if let Some(task) = col.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.title = title.clone();
                    task.description = Some(desc);
                    task.category_id = cat_id;
                    task.due_date = due.and_then(|d| {
                        chrono::NaiveDateTime::parse_from_str(&d, "%Y-%m-%d %H:%M:%S").ok()
                    });
                    self.log("UPDATE", "Task", &format!("Updated task '{title}'"));
                    self.save()?;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    pub fn delete_task(&mut self, board_id: u64, task_id: u64) -> Result<()> {
        if let Some(board) = self.data.boards.iter_mut().find(|b| b.id == board_id) {
            for col in &mut board.columns {
                if let Some(idx) = col.tasks.iter().position(|t| t.id == task_id) {
                    let title = col.tasks[idx].title.clone();
                    col.tasks.remove(idx);
                    self.log("DELETE", "Task", &format!("Deleted task '{title}'"));
                    self.save()?;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    pub fn move_task(&mut self, board_id: u64, task_id: u64, target_col_id: u64) -> Result<()> {
        let mut task_opt: Option<Task> = None;

        if let Some(board) = self.data.boards.iter_mut().find(|b| b.id == board_id) {
            let start_col = board.start_column_id;
            let finish_col = board.finish_column_id;
            let reset_col = board.reset_column_id;

            for col in &mut board.columns {
                if let Some(idx) = col.tasks.iter().position(|t| t.id == task_id) {
                    task_opt = Some(col.tasks.remove(idx));
                    break;
                }
            }

            if let Some(mut task) = task_opt {
                let now = Local::now().naive_local();

                if Some(target_col_id) == start_col {
                    task.start_date = Some(now);
                    task.finish_date = None;
                } else if Some(target_col_id) == finish_col {
                    if task.start_date.is_none() {
                        task.start_date = Some(task.creation_date);
                    }
                    task.finish_date = Some(now);
                } else if Some(target_col_id) == reset_col {
                    task.start_date = None;
                    task.finish_date = None;
                }

                if let Some(col) = board.columns.iter_mut().find(|c| c.id == target_col_id) {
                    col.tasks.push(task);
                    self.save()?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn setup_db() -> (Database, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let db = Database::new(file.path()).unwrap();
        (db, file)
    }

    #[test]
    fn test_init_creates_default_board() {
        let (db, _file) = setup_db();
        assert_eq!(db.data.boards.len(), 1);
        assert_eq!(db.data.boards[0].name, "Kanban Board");
        assert_eq!(db.data.boards[0].columns.len(), 3);
    }

    #[test]
    fn test_create_and_delete_board() {
        let (mut db, _file) = setup_db();
        db.create_board("New Board".into(), "🚀".into()).unwrap();
        assert_eq!(db.data.boards.len(), 2);

        let id = db.data.boards[1].id;
        db.delete_board(id).unwrap();
        assert_eq!(db.data.boards.len(), 1);
    }

    #[test]
    fn test_create_column() {
        let (mut db, _file) = setup_db();
        let board_id = db.data.boards[0].id;
        db.create_column("Archive".into(), board_id).unwrap();

        let board = &db.data.boards[0];
        assert_eq!(board.columns.len(), 4);
        assert_eq!(board.columns[3].name, "Archive");
    }

    #[test]
    fn test_delete_column_safety() {
        let (mut db, _file) = setup_db();
        let board_id = db.data.boards[0].id;
        let col_id = db.data.boards[0].columns[0].id;

        // Add a task to the column
        db.create_task(board_id, col_id, "T".into(), "".into(), None, None)
            .unwrap();

        // Try to delete non-empty column
        let result = db.delete_column(board_id, col_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_swap_columns() {
        let (mut db, _file) = setup_db();
        let board_id = db.data.boards[0].id;

        // Default: Ready (0), Doing (1), Done (2)
        let id_0 = db.data.boards[0].columns[0].id;
        let id_1 = db.data.boards[0].columns[1].id;

        db.swap_column_positions(board_id, 0, 1).unwrap();

        assert_eq!(db.data.boards[0].columns[0].id, id_1);
        assert_eq!(db.data.boards[0].columns[1].id, id_0);
    }

    #[test]
    fn test_category_crud() {
        let (mut db, _file) = setup_db();
        db.create_category("Work".into(), "#FF0000".into()).unwrap();
        assert_eq!(db.data.categories.len(), 1);

        let id = db.data.categories[0].id;
        db.update_category(id, "Work Updated".into(), "#00FF00".into())
            .unwrap();
        assert_eq!(db.data.categories[0].name, "Work Updated");

        db.delete_category(id).unwrap();
        assert!(db.data.categories.is_empty());
    }

    #[test]
    fn test_delete_category_clears_task_references() {
        let (mut db, _file) = setup_db();
        let board_id = db.data.boards[0].id;
        let col_id = db.data.boards[0].columns[0].id;

        db.create_category("Tag".into(), "#000".into()).unwrap();
        let cat_id = db.data.categories[0].id;

        db.create_task(board_id, col_id, "T".into(), "".into(), None, Some(cat_id))
            .unwrap();

        // Verify assignment
        assert_eq!(
            db.data.boards[0].columns[0].tasks[0].category_id,
            Some(cat_id)
        );

        // Delete category
        db.delete_category(cat_id).unwrap();

        // Verify nullified
        assert_eq!(db.data.boards[0].columns[0].tasks[0].category_id, None);
    }

    #[test]
    fn test_task_crud() {
        let (mut db, _file) = setup_db();
        let board_id = db.data.boards[0].id;
        let col_id = db.data.boards[0].columns[0].id;

        db.create_task(
            board_id,
            col_id,
            "My Task".into(),
            "Desc".into(),
            None,
            None,
        )
        .unwrap();
        let task_id = db.data.boards[0].columns[0].tasks[0].id;

        db.update_task(
            board_id,
            task_id,
            "Updated".into(),
            "Desc".into(),
            None,
            None,
        )
        .unwrap();
        assert_eq!(db.data.boards[0].columns[0].tasks[0].title, "Updated");

        db.delete_task(board_id, task_id).unwrap();
        assert!(db.data.boards[0].columns[0].tasks.is_empty());
    }

    #[test]
    fn test_move_task_basic() {
        let (mut db, _file) = setup_db();
        let board_id = db.data.boards[0].id;
        let col1 = db.data.boards[0].columns[0].id;
        let col2 = db.data.boards[0].columns[1].id;

        db.create_task(board_id, col1, "Move Me".into(), "".into(), None, None)
            .unwrap();
        let task_id = db.data.boards[0].columns[0].tasks[0].id;

        db.move_task(board_id, task_id, col2).unwrap();

        assert!(db.data.boards[0].columns[0].tasks.is_empty());
        assert_eq!(db.data.boards[0].columns[1].tasks.len(), 1);
    }

    #[test]
    fn test_trigger_start_date() {
        let (mut db, _file) = setup_db();
        let board_id = db.data.boards[0].id;
        let col1 = db.data.boards[0].columns[0].id;
        let col2 = db.data.boards[0].columns[1].id;

        // Set col2 as Start Column
        db.update_board_settings(board_id, Some(col2), None, None)
            .unwrap();

        db.create_task(board_id, col1, "T".into(), "".into(), None, None)
            .unwrap();
        let task_id = db.data.boards[0].columns[0].tasks[0].id;

        // Move to start column
        db.move_task(board_id, task_id, col2).unwrap();

        let task = &db.data.boards[0].columns[1].tasks[0];
        assert!(task.start_date.is_some());
        assert!(task.finish_date.is_none());
    }

    #[test]
    fn test_trigger_finish_date() {
        let (mut db, _file) = setup_db();
        let board_id = db.data.boards[0].id;
        let col1 = db.data.boards[0].columns[0].id;
        let col3 = db.data.boards[0].columns[2].id;

        // Set col3 as Finish Column
        db.update_board_settings(board_id, None, Some(col3), None)
            .unwrap();

        db.create_task(board_id, col1, "T".into(), "".into(), None, None)
            .unwrap();
        let task_id = db.data.boards[0].columns[0].tasks[0].id;

        // Move to finish column
        db.move_task(board_id, task_id, col3).unwrap();

        let task = &db.data.boards[0].columns[2].tasks[0];
        assert!(task.finish_date.is_some());
        // Should backfill start date if missing
        assert!(task.start_date.is_some());
    }

    #[test]
    fn test_trigger_reset() {
        let (mut db, _file) = setup_db();
        let board_id = db.data.boards[0].id;
        let col1 = db.data.boards[0].columns[0].id;
        let col2 = db.data.boards[0].columns[1].id;

        // Set col1 as Reset
        db.update_board_settings(board_id, None, None, Some(col1))
            .unwrap();

        // Create task in col2 (simulate it was started)
        db.create_task(board_id, col2, "T".into(), "".into(), None, None)
            .unwrap();
        let task_id = db.data.boards[0].columns[1].tasks[0].id;

        // Move to reset
        db.move_task(board_id, task_id, col1).unwrap();

        let task = &db.data.boards[0].columns[0].tasks[0];
        assert!(task.start_date.is_none());
        assert!(task.finish_date.is_none());
    }

    #[test]
    fn test_audit_logging() {
        let (mut db, _file) = setup_db();

        // Use the log path directly from the database instance
        let log_path = db.log_file_path.clone();

        // Create a board to trigger a log
        db.create_board("Log Test".into(), "".into()).unwrap();

        // Check if log file exists and contains JSON
        assert!(log_path.exists());
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("CREATE"));
        assert!(content.contains("Log Test"));

        // Test reading back via get_recent_logs
        let logs = db.get_recent_logs(50);
        assert!(!logs.is_empty());
        assert_eq!(logs[0].event_type, "CREATE");
        assert_eq!(logs[0].object_type, "Board");

        // Cleanup the created log file
        let _ = std::fs::remove_file(log_path);
    }
}
