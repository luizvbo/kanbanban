use std::collections::HashMap;

use anyhow::Result;
use chrono::NaiveDate;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::TableState;
use tui_textarea::TextArea;

use crate::{
    db::Database,
    models::{Board, Category, Column, LogEvent, Task},
};

#[derive(Clone)]
pub struct TaskWithContext {
    pub inner: Task,
    pub column_id: u64,
}

pub enum InputMode {
    Normal,
    Editing,
}

pub enum ModalType {
    TaskNew,
    TaskEdit(u64),
    BoardNew,
    BoardEdit(u64),
    ColumnNew,
    ColumnEdit(u64),
    CategoryNew,
    CategoryEdit(u64),
}

#[derive(PartialEq, Debug)]
pub enum TaskFocus {
    Title,
    Description,
    Date,
    Category,
}

#[derive(PartialEq, Debug)]
pub enum CategoryFocus {
    Name,
    Color,
}

#[derive(PartialEq, Debug)]
pub enum BoardFocus {
    Name,
    Icon,
}

pub enum ActiveModal<'a> {
    Task {
        title: Box<TextArea<'a>>,
        desc: Box<TextArea<'a>>,
        date: Box<TextArea<'a>>,
        category_idx: usize,
        focus: TaskFocus,
        mode: ModalType,
    },
    Column {
        name: Box<TextArea<'a>>,
        mode: ModalType,
    },
    Category {
        name: Box<TextArea<'a>>,
        color: Box<TextArea<'a>>,
        focus: CategoryFocus,
        mode: ModalType,
    },
    Board {
        name: Box<TextArea<'a>>,
        icon: Box<TextArea<'a>>,
        focus: BoardFocus,
        mode: ModalType,
    },
}

#[derive(Debug, PartialEq)]
pub enum ViewMode {
    Board,
    Overview,
    Settings,
    BoardSelector,
    CategoryManager,
}

pub struct SettingsState {
    pub selected_column_idx: usize,
}

pub enum OverviewTab {
    Charts,
    Logs,
}

pub struct OverviewState {
    pub tab: OverviewTab,
    pub logs: Vec<LogEvent>,
    pub log_state: TableState,
}

// --- New Enums/Structs ---

#[derive(Clone, Copy, Debug)]
pub enum HitZone {
    Column(usize),        // Column Index
    Task(usize, usize),   // Column Index, Task Index
    BoardSelector(usize), // Board Index
    #[allow(dead_code)] // Suppress warning if None isn't explicitly constructed yet
    None,
}

#[derive(Clone, Debug)]
pub struct DragState {
    pub task_id: u64,
    pub source_col_idx: usize,
    #[allow(dead_code)] // Suppress warning until intra-column reordering is implemented
    pub source_task_idx: usize,
    pub is_dragging: bool,
}

pub struct DatePickerState {
    pub current_date: NaiveDate,
    // Removed 'focused' field as it was unused
}

pub struct App<'a> {
    pub db: Database,
    pub should_quit: bool,
    pub input_mode: InputMode,
    pub view_mode: ViewMode,

    // Data Snapshots
    pub boards: Vec<Board>,
    pub active_board_index: usize,
    pub columns: Vec<Column>,
    pub tasks: Vec<TaskWithContext>,
    pub categories: Vec<Category>,
    pub category_map: HashMap<u64, Color>,

    // UI State
    pub active_modal: Option<ActiveModal<'a>>,
    pub selected_column_index: usize,
    pub selected_task_index: usize,
    pub board_selector_index: usize,
    pub category_selector_index: usize,

    pub settings_state: SettingsState,
    pub overview_state: OverviewState,

    // Chart Data
    pub chart_data: Vec<(String, u64)>,
    pub chart_max_y: u64,

    // Mouse & Interaction State
    pub hit_zones: Vec<(Rect, HitZone)>,
    pub drag_state: Option<DragState>,

    // Date Picker State
    pub date_picker: Option<DatePickerState>,
}

impl<'a> App<'a> {
    pub fn new(db: Database) -> Result<Self> {
        let mut app = Self {
            db,
            should_quit: false,
            input_mode: InputMode::Normal,
            view_mode: ViewMode::Board,
            boards: vec![],
            active_board_index: 0,
            columns: vec![],
            tasks: vec![],
            categories: vec![],
            category_map: HashMap::new(),
            active_modal: None,
            selected_column_index: 0,
            selected_task_index: 0,
            board_selector_index: 0,
            category_selector_index: 0,
            settings_state: SettingsState {
                selected_column_idx: 0,
            },
            overview_state: OverviewState {
                tab: OverviewTab::Logs,
                logs: vec![],
                log_state: TableState::default(),
            },
            chart_data: vec![],
            chart_max_y: 10,
            hit_zones: vec![],
            drag_state: None,
            date_picker: None,
        };
        app.refresh_data()?;
        Ok(app)
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn refresh_data(&mut self) -> Result<()> {
        self.boards = self.db.data.boards.clone();
        self.categories = self.db.data.categories.clone();
        self.overview_state.logs = self.db.data.audit_logs.clone();
        self.overview_state.logs.reverse();

        self.category_map.clear();
        for cat in &self.categories {
            self.category_map
                .insert(cat.id, self.hex_to_color(&cat.color));
        }

        if self.active_board_index >= self.boards.len() {
            self.active_board_index = 0;
        }

        self.columns.clear();
        self.tasks.clear();

        if let Some(board) = self.boards.get(self.active_board_index) {
            self.columns = board.columns.clone();
            for col in &board.columns {
                for task in &col.tasks {
                    self.tasks.push(TaskWithContext {
                        inner: task.clone(),
                        column_id: col.id,
                    });
                }
            }
            self.calculate_chart_data();
        }
        Ok(())
    }

    fn hex_to_color(&self, hex: &str) -> Color {
        if hex.len() == 7 && hex.starts_with('#') {
            let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(255);
            Color::Rgb(r, g, b)
        } else {
            Color::White
        }
    }

    // --- Navigation Logic ---
    pub fn next_board(&mut self) -> Result<()> {
        if self.board_selector_index < self.boards.len().saturating_sub(1) {
            self.board_selector_index += 1;
        }
        Ok(())
    }
    pub fn prev_board(&mut self) -> Result<()> {
        if self.board_selector_index > 0 {
            self.board_selector_index -= 1;
        }
        Ok(())
    }

    pub fn select_highlighted_board(&mut self) -> Result<()> {
        self.active_board_index = self.board_selector_index;
        self.view_mode = ViewMode::Board;
        self.refresh_data()?;
        Ok(())
    }

    pub fn move_down(&mut self) {
        let col_id = self.get_selected_column_id();
        let tasks_in_col = self.get_tasks_in_column(col_id).len();
        if tasks_in_col > 0 && self.selected_task_index < tasks_in_col - 1 {
            self.selected_task_index += 1;
        }
    }
    pub fn move_up(&mut self) {
        if self.selected_task_index > 0 {
            self.selected_task_index -= 1;
        }
    }
    pub fn move_right(&mut self) {
        let visible_cols: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.visible)
            .map(|(i, _)| i)
            .collect();

        if visible_cols.is_empty() {
            return;
        }

        if let Some(pos) = visible_cols
            .iter()
            .position(|&i| i == self.selected_column_index)
        {
            if pos < visible_cols.len() - 1 {
                self.selected_column_index = visible_cols[pos + 1];
                self.selected_task_index = 0;
            }
        } else {
            self.selected_column_index = visible_cols[0];
            self.selected_task_index = 0;
        }
    }
    pub fn move_left(&mut self) {
        let visible_cols: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.visible)
            .map(|(i, _)| i)
            .collect();

        if visible_cols.is_empty() {
            return;
        }

        if let Some(pos) = visible_cols
            .iter()
            .position(|&i| i == self.selected_column_index)
        {
            if pos > 0 {
                self.selected_column_index = visible_cols[pos - 1];
                self.selected_task_index = 0;
            }
        } else {
            self.selected_column_index = visible_cols[0];
            self.selected_task_index = 0;
        }
    }

    // --- Task Movement ---
    pub fn move_task_right(&mut self) -> Result<()> {
        self.shift_task(1)
    }
    pub fn move_task_left(&mut self) -> Result<()> {
        self.shift_task(-1)
    }

    fn shift_task(&mut self, dir: i32) -> Result<()> {
        let visible_cols: Vec<(usize, &Column)> = self
            .columns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.visible)
            .collect();
        if visible_cols.is_empty() {
            return Ok(());
        }

        let current_vis_idx = visible_cols
            .iter()
            .position(|(i, _)| *i == self.selected_column_index)
            .unwrap_or(0);
        let new_idx = current_vis_idx as i32 + dir;

        if new_idx >= 0 && new_idx < visible_cols.len() as i32 {
            let (target_real_idx, _) = visible_cols[new_idx as usize];

            if self.selected_column_index >= self.columns.len()
                || target_real_idx >= self.columns.len()
            {
                return Ok(());
            }

            let current_col_id = self.columns[self.selected_column_index].id;
            let next_col_id = self.columns[target_real_idx].id;
            let tasks = self.get_tasks_in_column(Some(current_col_id));

            if let Some(task) = tasks.get(self.selected_task_index) {
                let board = &self.boards[self.active_board_index];
                self.db.move_task(board.id, task.inner.id, next_col_id)?;
                self.selected_column_index = target_real_idx;
                self.selected_task_index = 0;
                self.refresh_data()?;
            }
        }
        Ok(())
    }

    // --- Settings Logic ---
    pub fn toggle_column_visibility(&mut self) -> Result<()> {
        let idx = self.settings_state.selected_column_idx;
        if let Some(col) = self.columns.get(idx) {
            let board = &self.boards[self.active_board_index];
            self.db
                .update_column_visibility(board.id, col.id, !col.visible)?;
            self.refresh_data()?;
        }
        Ok(())
    }

    pub fn move_column_up(&mut self) -> Result<()> {
        let idx = self.settings_state.selected_column_idx;
        if idx > 0 {
            let board = &self.boards[self.active_board_index];
            self.db.swap_column_positions(board.id, idx, idx - 1)?;
            self.settings_state.selected_column_idx -= 1;
            self.refresh_data()?;
        }
        Ok(())
    }

    pub fn move_column_down(&mut self) -> Result<()> {
        let idx = self.settings_state.selected_column_idx;
        if idx < self.columns.len() - 1 {
            let board = &self.boards[self.active_board_index];
            self.db.swap_column_positions(board.id, idx, idx + 1)?;
            self.settings_state.selected_column_idx += 1;
            self.refresh_data()?;
        }
        Ok(())
    }

    pub fn set_column_as_status(&mut self, status_type: &str) -> Result<()> {
        let idx = self.settings_state.selected_column_idx;
        if let Some(col) = self.columns.get(idx) {
            let col_id = col.id;
            let mut board = self.boards[self.active_board_index].clone();

            match status_type {
                "start" => board.start_column_id = Some(col_id),
                "finish" => board.finish_column_id = Some(col_id),
                "reset" => board.reset_column_id = Some(col_id),
                _ => {}
            }

            self.db.update_board_settings(
                board.id,
                board.start_column_id,
                board.finish_column_id,
                board.reset_column_id,
            )?;
            self.refresh_data()?;
        }
        Ok(())
    }

    // --- Overview Scrolling ---
    pub fn scroll_logs_down(&mut self) {
        let i = match self.overview_state.log_state.selected() {
            Some(i) => {
                if i >= self.overview_state.logs.len().saturating_sub(1) {
                    self.overview_state.logs.len().saturating_sub(1)
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.overview_state.log_state.select(Some(i));
    }

    pub fn scroll_logs_up(&mut self) {
        let i = match self.overview_state.log_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.overview_state.log_state.select(Some(i));
    }

    // --- Chart Logic ---
    fn calculate_chart_data(&mut self) {
        let mut counts: HashMap<String, u64> = HashMap::new();

        for cat in &self.categories {
            counts.insert(cat.name.clone(), 0);
        }
        counts.insert("No Category".to_string(), 0);

        for task in &self.tasks {
            let key = if let Some(cid) = task.inner.category_id {
                self.categories
                    .iter()
                    .find(|c| c.id == cid)
                    .map(|c| c.name.clone())
                    .unwrap_or("Unknown".to_string())
            } else {
                "No Category".to_string()
            };
            *counts.entry(key).or_default() += 1;
        }

        self.chart_data = counts.into_iter().collect();
        self.chart_data.sort_by(|a, b| a.0.cmp(&b.0));
        self.chart_max_y = self
            .chart_data
            .iter()
            .map(|(_, c)| *c)
            .max()
            .unwrap_or(0)
            .max(5);
    }

    // --- Modal Openers ---
    pub fn open_new_task_modal(&mut self) {
        let mut title = TextArea::default();
        title.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Title"),
        );
        let mut desc = TextArea::default();
        desc.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Description"),
        );
        let mut date = TextArea::default();
        date.set_placeholder_text("YYYY-MM-DD");
        date.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Due Date (Ctrl+D to Pick)"),
        );

        self.active_modal = Some(ActiveModal::Task {
            title: Box::new(title),
            desc: Box::new(desc),
            date: Box::new(date),
            category_idx: 0,
            focus: TaskFocus::Title,
            mode: ModalType::TaskNew,
        });
        self.input_mode = InputMode::Editing;
    }

    pub fn open_edit_task_modal(&mut self) {
        let col_id = self.get_selected_column_id();
        let tasks = self.get_tasks_in_column(col_id);
        if let Some(task) = tasks.get(self.selected_task_index) {
            let mut title = TextArea::from(task.inner.title.lines());
            title.set_block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title("Title"),
            );
            let mut desc =
                TextArea::from(task.inner.description.clone().unwrap_or_default().lines());
            desc.set_block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title("Description"),
            );
            let date_str = task
                .inner
                .due_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            let mut date = TextArea::from(date_str.lines());
            date.set_block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title("Due Date (Ctrl+D to Pick)"),
            );

            let cat_idx = if let Some(cid) = task.inner.category_id {
                self.categories
                    .iter()
                    .position(|c| c.id == cid)
                    .unwrap_or(0)
            } else {
                0
            };

            self.active_modal = Some(ActiveModal::Task {
                title: Box::new(title),
                desc: Box::new(desc),
                date: Box::new(date),
                category_idx: cat_idx,
                focus: TaskFocus::Title,
                mode: ModalType::TaskEdit(task.inner.id),
            });
            self.input_mode = InputMode::Editing;
        }
    }

    pub fn open_edit_board_modal(&mut self) {
        if let Some(board) = self.boards.get(self.board_selector_index) {
            let mut name = TextArea::from(board.name.lines());
            name.set_block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title("Board Name"),
            );
            let mut icon = TextArea::from(board.icon.clone().unwrap_or_default().lines());
            icon.set_block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title("Icon"),
            );

            self.active_modal = Some(ActiveModal::Board {
                name: Box::new(name),
                icon: Box::new(icon),
                focus: BoardFocus::Name,
                mode: ModalType::BoardEdit(board.id),
            });
            self.input_mode = InputMode::Editing;
        }
    }

    pub fn delete_highlighted_board(&mut self) -> Result<()> {
        // Prevent deleting the last board
        if self.boards.len() <= 1 {
            return Ok(());
        }

        if let Some(board) = self.boards.get(self.board_selector_index) {
            self.db.delete_board(board.id)?;

            // Adjust indices
            if self.board_selector_index >= self.boards.len().saturating_sub(1) {
                self.board_selector_index = self.boards.len().saturating_sub(2);
            }

            // If we deleted the active board, reset active index
            if self.active_board_index >= self.boards.len().saturating_sub(1) {
                self.active_board_index = 0;
            }

            self.refresh_data()?;
        }
        Ok(())
    }

    pub fn open_new_column_modal(&mut self) {
        let mut name = TextArea::default();
        name.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Column Name"),
        );
        self.active_modal = Some(ActiveModal::Column {
            name: Box::new(name),
            mode: ModalType::ColumnNew,
        });
        self.input_mode = InputMode::Editing;
    }

    pub fn open_edit_column_modal(&mut self) {
        if let Some(col) = self.columns.get(self.settings_state.selected_column_idx) {
            let mut name = TextArea::from(col.name.lines());
            name.set_block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title("Column Name"),
            );
            self.active_modal = Some(ActiveModal::Column {
                name: Box::new(name),
                mode: ModalType::ColumnEdit(col.id),
            });
            self.input_mode = InputMode::Editing;
        }
    }

    pub fn open_new_category_modal(&mut self) {
        let mut name = TextArea::default();
        name.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Category Name"),
        );
        let mut color = TextArea::default();
        color.set_placeholder_text("#RRGGBB");
        color.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Color (Hex)"),
        );
        self.active_modal = Some(ActiveModal::Category {
            name: Box::new(name),
            color: Box::new(color),
            focus: CategoryFocus::Name,
            mode: ModalType::CategoryNew,
        });
        self.input_mode = InputMode::Editing;
    }

    pub fn open_edit_category_modal(&mut self) {
        if let Some(cat) = self.categories.get(self.category_selector_index) {
            let mut name = TextArea::from(cat.name.lines());
            name.set_block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title("Category Name"),
            );
            let mut color = TextArea::from(cat.color.lines());
            color.set_block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title("Color (Hex)"),
            );
            self.active_modal = Some(ActiveModal::Category {
                name: Box::new(name),
                color: Box::new(color),
                focus: CategoryFocus::Name,
                mode: ModalType::CategoryEdit(cat.id),
            });
            self.input_mode = InputMode::Editing;
        }
    }

    pub fn open_new_board_modal(&mut self) {
        let mut name = TextArea::default();
        name.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Board Name"),
        );
        let mut icon = TextArea::default();
        icon.set_placeholder_text("Icon (e.g. 📝)");
        icon.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Icon"),
        );

        self.active_modal = Some(ActiveModal::Board {
            name: Box::new(name),
            icon: Box::new(icon),
            focus: BoardFocus::Name,
            mode: ModalType::BoardNew,
        });
        self.input_mode = InputMode::Editing;
    }

    pub fn close_modal(&mut self) {
        self.active_modal = None;
        self.input_mode = InputMode::Normal;
    }

    pub fn cycle_modal_focus(&mut self) {
        if let Some(modal) = &mut self.active_modal {
            match modal {
                ActiveModal::Task { focus, .. } => {
                    *focus = match focus {
                        TaskFocus::Title => TaskFocus::Description,
                        TaskFocus::Description => TaskFocus::Date,
                        TaskFocus::Date => TaskFocus::Category,
                        TaskFocus::Category => TaskFocus::Title,
                    };
                }
                ActiveModal::Category { focus, .. } => {
                    *focus = match focus {
                        CategoryFocus::Name => CategoryFocus::Color,
                        CategoryFocus::Color => CategoryFocus::Name,
                    };
                }
                ActiveModal::Board { focus, .. } => {
                    *focus = match focus {
                        BoardFocus::Name => BoardFocus::Icon,
                        BoardFocus::Icon => BoardFocus::Name,
                    };
                }
                ActiveModal::Column { .. } => {}
            }
        }
    }

    pub fn on_modal_input(&mut self, key: ratatui::crossterm::event::KeyEvent) {
        if let Some(modal) = &mut self.active_modal {
            match modal {
                ActiveModal::Task {
                    title,
                    desc,
                    date,
                    focus,
                    ..
                } => match focus {
                    TaskFocus::Title => {
                        title.input(key);
                    }
                    TaskFocus::Description => {
                        desc.input(key);
                    }
                    TaskFocus::Date => {
                        date.input(key);
                    }
                    TaskFocus::Category => {} // Handled in main.rs
                },
                ActiveModal::Column { name, .. } => {
                    name.input(key);
                }
                ActiveModal::Category {
                    name, color, focus, ..
                } => match focus {
                    CategoryFocus::Name => {
                        name.input(key);
                    }
                    CategoryFocus::Color => {
                        color.input(key);
                    }
                },
                ActiveModal::Board {
                    name, icon, focus, ..
                } => match focus {
                    BoardFocus::Name => {
                        name.input(key);
                    }
                    BoardFocus::Icon => {
                        icon.input(key);
                    }
                },
            }
        }
    }

    pub fn save_modal_data(&mut self) -> Result<()> {
        if let Some(modal) = &self.active_modal {
            match modal {
                ActiveModal::Task {
                    title,
                    desc,
                    date,
                    category_idx,
                    mode,
                    ..
                } => {
                    let t = title.lines().join("\n");
                    let d = desc.lines().join("\n");
                    let date_str = date.lines().join("");
                    let due = if date_str.trim().is_empty() {
                        None
                    } else {
                        Some(format!("{} 00:00:00", date_str.trim()))
                    };

                    let cat_id =
                        if !self.categories.is_empty() && *category_idx < self.categories.len() {
                            Some(self.categories[*category_idx].id)
                        } else {
                            None
                        };

                    if !t.trim().is_empty() {
                        match mode {
                            ModalType::TaskNew => {
                                if let Some(col) = self.columns.get(self.selected_column_index) {
                                    let board = &self.boards[self.active_board_index];
                                    self.db.create_task(board.id, col.id, t, d, due, cat_id)?;
                                }
                            }
                            ModalType::TaskEdit(id) => {
                                let board = &self.boards[self.active_board_index];
                                self.db.update_task(board.id, *id, t, d, due, cat_id)?;
                            }
                            _ => {}
                        }
                    }
                }
                ActiveModal::Column { name, mode } => {
                    let n = name.lines().join("");
                    if !n.trim().is_empty()
                        && let Some(board) = self.boards.get(self.active_board_index)
                    {
                        match mode {
                            ModalType::ColumnNew => {
                                // Was implicitly new before
                                self.db.create_column(n, board.id)?;
                            }
                            ModalType::ColumnEdit(id) => {
                                self.db.update_column(board.id, *id, n)?;
                            }
                            _ => {} // Should not happen
                        }
                    }
                }
                ActiveModal::Category {
                    name, color, mode, ..
                } => {
                    let n = name.lines().join("");
                    let c = color.lines().join("");
                    if !n.trim().is_empty() && !c.trim().is_empty() {
                        match mode {
                            ModalType::CategoryNew => {
                                self.db.create_category(n, c)?;
                            }
                            ModalType::CategoryEdit(id) => {
                                self.db.update_category(*id, n, c)?;
                            }
                            _ => {}
                        }
                    }
                }
                ActiveModal::Board {
                    name, icon, mode, ..
                } => {
                    let n = name.lines().join("");
                    let i = icon.lines().join("");
                    if !n.trim().is_empty() {
                        match mode {
                            ModalType::BoardNew => {
                                self.db.create_board(n, i)?;
                            }
                            ModalType::BoardEdit(id) => {
                                self.db.update_board(*id, n, i)?;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        self.close_modal();
        self.refresh_data()?;
        Ok(())
    }

    pub fn delete_current_column(&mut self) -> Result<()> {
        if let Some(col) = self.columns.get(self.selected_column_index)
            && let Some(board) = self.boards.get(self.active_board_index)
        {
            self.db.delete_column(board.id, col.id)?;
            if self.selected_column_index > 0 {
                self.selected_column_index -= 1;
            }
            self.refresh_data()?;
        }
        Ok(())
    }

    pub fn delete_current_task(&mut self) -> Result<()> {
        let col_id = self.get_selected_column_id();
        let tasks = self.get_tasks_in_column(col_id);

        if let Some(task) = tasks.get(self.selected_task_index) {
            let board = &self.boards[self.active_board_index];
            self.db.delete_task(board.id, task.inner.id)?;

            // Adjust selection to prevent out of bounds
            if self.selected_task_index > 0 {
                self.selected_task_index -= 1;
            }
            self.refresh_data()?;
        }
        Ok(())
    }

    pub fn delete_current_category(&mut self) -> Result<()> {
        if let Some(cat) = self.categories.get(self.category_selector_index) {
            self.db.delete_category(cat.id)?;
            if self.category_selector_index > 0 {
                self.category_selector_index -= 1;
            }
            self.refresh_data()?;
        }
        Ok(())
    }

    // Helpers
    fn get_selected_column_id(&self) -> Option<u64> {
        self.columns.get(self.selected_column_index).map(|c| c.id)
    }

    pub fn get_tasks_in_column(&self, col_id: Option<u64>) -> Vec<&TaskWithContext> {
        if let Some(cid) = col_id {
            self.tasks.iter().filter(|t| t.column_id == cid).collect()
        } else {
            vec![]
        }
    }

    // --- Mouse Logic ---

    pub fn handle_mouse_down(&mut self, x: u16, y: u16) {
        // Find what was clicked
        let zone = self
            .hit_zones
            .iter()
            .find(|(rect, _)| {
                x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
            })
            .map(|(_, zone)| zone);

        if let Some(z) = zone {
            match z {
                HitZone::Task(col_idx, task_idx) => {
                    // Select the task immediately
                    self.selected_column_index = *col_idx;
                    self.selected_task_index = *task_idx;

                    // Prepare for potential drag
                    let col_id = self.columns[*col_idx].id;
                    let tasks = self.get_tasks_in_column(Some(col_id));
                    if let Some(task) = tasks.get(*task_idx) {
                        self.drag_state = Some(DragState {
                            task_id: task.inner.id,
                            source_col_idx: *col_idx,
                            source_task_idx: *task_idx,
                            is_dragging: false, // Will become true on Drag event
                        });
                    }
                }
                HitZone::Column(col_idx) => {
                    self.selected_column_index = *col_idx;
                    self.selected_task_index = 0;
                }
                HitZone::BoardSelector(idx) => {
                    self.board_selector_index = *idx;
                    let _ = self.select_highlighted_board();
                }
                _ => {}
            }
        }
    }

    pub fn handle_mouse_drag(&mut self, _x: u16, _y: u16) {
        if let Some(state) = &mut self.drag_state {
            state.is_dragging = true;
        }
    }

    pub fn handle_mouse_up(&mut self, x: u16, y: u16) -> Result<()> {
        // Handle Drop
        if let Some(drag) = self.drag_state.take()
            && drag.is_dragging
        {
            // Find target column
            let target_zone = self.hit_zones.iter().find(|(rect, zone)| {
                x >= rect.x
                    && x < rect.x + rect.width
                    && y >= rect.y
                    && y < rect.y + rect.height
                    && matches!(zone, HitZone::Column(_) | HitZone::Task(_, _))
            });

            if let Some((_, zone)) = target_zone {
                let target_col_idx = match zone {
                    HitZone::Column(i) => *i,
                    HitZone::Task(i, _) => *i,
                    _ => return Ok(()),
                };

                // Only move if column changed
                if target_col_idx != drag.source_col_idx {
                    let board = &self.boards[self.active_board_index];
                    let target_col_id = self.columns[target_col_idx].id;
                    self.db.move_task(board.id, drag.task_id, target_col_id)?;

                    self.selected_column_index = target_col_idx;
                    self.selected_task_index = 0; // Simplified: drop to top
                    self.refresh_data()?;
                }
            }
        }
        Ok(())
    }

    // --- Date Picker Logic ---

    #[allow(dead_code)] // Suppress warning if not used in tests
    pub fn open_date_picker(&mut self) {
        let now = chrono::Local::now().naive_local().date();
        self.date_picker = Some(DatePickerState { current_date: now });
    }

    pub fn date_picker_nav(&mut self, dx: i32, dy: i32) {
        if let Some(dp) = &mut self.date_picker {
            if dy != 0 {
                // Up/Down changes weeks
                dp.current_date += chrono::Duration::days((dy * 7) as i64);
            } else if dx != 0 {
                // Left/Right changes days
                dp.current_date += chrono::Duration::days(dx as i64);
            }
        }
    }

    pub fn select_date(&mut self) {
        if let Some(dp) = &self.date_picker {
            let date_str = dp.current_date.format("%Y-%m-%d").to_string();

            // Inject into the active modal's date field
            if let Some(ActiveModal::Task { date, .. }) = &mut self.active_modal {
                // Create a new TextArea with the selected date
                let mut new_area = TextArea::from(vec![date_str]);

                // Preserve the existing block styling (borders, title)
                if let Some(block) = date.block() {
                    new_area.set_block(block.clone());
                }

                // Replace the old TextArea
                *date = Box::new(new_area);
            }
        }
        self.date_picker = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use tempfile::NamedTempFile;

    fn setup_app() -> (App<'static>, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let db = Database::new(file.path()).unwrap();
        let app = App::new(db).unwrap();
        (app, file)
    }

    #[test]
    fn test_app_initial_state() {
        let (app, _file) = setup_app();
        assert_eq!(app.boards.len(), 1);
        assert_eq!(app.columns.len(), 3);
        assert_eq!(app.view_mode, ViewMode::Board);
    }

    #[test]
    fn test_navigation_vertical() {
        let (mut app, _file) = setup_app();
        let board_id = app.boards[0].id;
        let col_id = app.columns[0].id;

        // Add 3 tasks
        app.db
            .create_task(board_id, col_id, "1".into(), "".into(), None, None)
            .unwrap();
        app.db
            .create_task(board_id, col_id, "2".into(), "".into(), None, None)
            .unwrap();
        app.db
            .create_task(board_id, col_id, "3".into(), "".into(), None, None)
            .unwrap();
        app.refresh_data().unwrap();

        assert_eq!(app.selected_task_index, 0);

        app.move_down();
        assert_eq!(app.selected_task_index, 1);

        app.move_down();
        assert_eq!(app.selected_task_index, 2);

        // Should clamp at bottom
        app.move_down();
        assert_eq!(app.selected_task_index, 2);

        app.move_up();
        assert_eq!(app.selected_task_index, 1);
    }

    #[test]
    fn test_navigation_horizontal() {
        let (mut app, _file) = setup_app();
        // Default cols: 0, 1, 2
        let col0_id = app.columns[0].id;
        let col1_id = app.columns[1].id;

        assert_eq!(app.get_selected_column_id(), Some(col0_id));

        app.move_right();
        assert_eq!(app.get_selected_column_id(), Some(col1_id));

        app.move_left();
        assert_eq!(app.get_selected_column_id(), Some(col0_id));
    }

    #[test]
    fn test_board_switching() {
        let (mut app, _file) = setup_app();
        app.db.create_board("Board 2".into(), "".into()).unwrap();
        app.refresh_data().unwrap();

        assert_eq!(app.active_board_index, 0);

        // Simulate selector UI movement
        app.next_board().unwrap();
        assert_eq!(app.board_selector_index, 1);

        // Select it
        app.select_highlighted_board().unwrap();
        assert_eq!(app.active_board_index, 1);
        assert_eq!(app.boards[app.active_board_index].name, "Board 2");
    }

    #[test]
    fn test_chart_data_calculation() {
        let (mut app, _file) = setup_app();
        let board_id = app.boards[0].id;
        let col_id = app.columns[0].id;

        app.db.create_category("Cat1".into(), "".into()).unwrap();
        let cat_id = app.db.data.categories[0].id;

        // 1 Task with Category, 1 without
        app.db
            .create_task(board_id, col_id, "T1".into(), "".into(), None, Some(cat_id))
            .unwrap();
        app.db
            .create_task(board_id, col_id, "T2".into(), "".into(), None, None)
            .unwrap();

        app.refresh_data().unwrap();

        // Chart data should have "Cat1": 1 and "No Category": 1
        let cat1_count = app
            .chart_data
            .iter()
            .find(|(k, _)| k == "Cat1")
            .map(|(_, v)| *v);
        let none_count = app
            .chart_data
            .iter()
            .find(|(k, _)| k == "No Category")
            .map(|(_, v)| *v);

        assert_eq!(cat1_count, Some(1));
        assert_eq!(none_count, Some(1));
    }

    #[test]
    fn test_settings_column_visibility_toggle() {
        let (mut app, _file) = setup_app();

        // Select first column in settings
        app.settings_state.selected_column_idx = 0;

        // Toggle visibility
        app.toggle_column_visibility().unwrap();

        assert!(!app.columns[0].visible);

        // Toggle back
        app.toggle_column_visibility().unwrap();
        assert!(app.columns[0].visible);
    }

    #[test]
    fn test_task_creation_modal_logic() {
        let (mut app, _file) = setup_app();

        // Open modal
        app.open_new_task_modal();
        assert!(app.active_modal.is_some());

        // Simulate filling data (manually setting state as UI interaction is hard to mock)
        if let Some(ActiveModal::Task { title, .. }) = &mut app.active_modal {
            title.insert_str("New Task");
        }

        // Save
        app.save_modal_data().unwrap();

        // Verify
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].inner.title, "New Task");
    }

    #[test]
    fn test_task_movement_logic() {
        let (mut app, _file) = setup_app();
        let board_id = app.boards[0].id;
        let col0_id = app.columns[0].id;
        let col1_id = app.columns[1].id;

        app.db
            .create_task(board_id, col0_id, "T1".into(), "".into(), None, None)
            .unwrap();
        app.refresh_data().unwrap();

        assert_eq!(app.tasks[0].column_id, col0_id);

        // Move right
        app.move_task_right().unwrap();

        // Verify column change
        assert_eq!(app.tasks[0].column_id, col1_id);
        assert_eq!(app.selected_column_index, 1);
    }

    #[test]
    fn test_modal_focus_cycling() {
        let (mut app, _file) = setup_app();
        app.open_new_task_modal();

        if let Some(ActiveModal::Task { focus, .. }) = &app.active_modal {
            assert_eq!(*focus, TaskFocus::Title);
        }

        app.cycle_modal_focus();
        if let Some(ActiveModal::Task { focus, .. }) = &app.active_modal {
            assert_eq!(*focus, TaskFocus::Description);
        }

        app.cycle_modal_focus();
        if let Some(ActiveModal::Task { focus, .. }) = &app.active_modal {
            assert_eq!(*focus, TaskFocus::Date);
        }

        app.cycle_modal_focus();
        if let Some(ActiveModal::Task { focus, .. }) = &app.active_modal {
            assert_eq!(*focus, TaskFocus::Category);
        }

        app.cycle_modal_focus();
        if let Some(ActiveModal::Task { focus, .. }) = &app.active_modal {
            assert_eq!(*focus, TaskFocus::Title);
        }
    }

    #[test]
    fn test_modal_input_handling() {
        let (mut app, _file) = setup_app();
        app.open_new_column_modal();

        let key = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::empty());
        app.on_modal_input(key);

        if let Some(ActiveModal::Column { name, .. }) = &app.active_modal {
            assert_eq!(name.lines()[0], "A");
        }
    }

    #[test]
    fn test_category_modal_save() {
        let (mut app, _file) = setup_app();
        app.open_new_category_modal();

        if let Some(ActiveModal::Category { name, color, .. }) = &mut app.active_modal {
            name.insert_str("Work");
            color.insert_str("#FF0000");
        }

        app.save_modal_data().unwrap();
        assert_eq!(app.categories.len(), 1);
        assert_eq!(app.categories[0].name, "Work");
    }

    #[test]
    fn test_board_modal_save() {
        let (mut app, _file) = setup_app();
        app.open_new_board_modal();

        if let Some(ActiveModal::Board { name, icon, .. }) = &mut app.active_modal {
            name.insert_str("New Board");
            icon.insert_str("NB");
        }

        app.save_modal_data().unwrap();
        assert_eq!(app.boards.len(), 2);
        assert_eq!(app.boards[1].name, "New Board");
    }

    #[test]
    fn test_delete_column() {
        let (mut app, _file) = setup_app();
        // Default 3 columns
        app.delete_current_column().unwrap();
        assert_eq!(app.columns.len(), 2);
    }

    #[test]
    fn test_delete_category() {
        let (mut app, _file) = setup_app();
        app.db
            .create_category("Test".into(), "#000".into())
            .unwrap();
        app.refresh_data().unwrap();

        app.category_selector_index = 0;
        app.delete_current_category().unwrap();
        assert_eq!(app.categories.len(), 0);
    }

    #[test]
    fn test_settings_move_column() {
        let (mut app, _file) = setup_app();
        // 0: Ready, 1: Doing
        app.settings_state.selected_column_idx = 1;
        app.move_column_up().unwrap();

        // Now 0: Doing, 1: Ready
        assert_eq!(app.columns[0].name, "Doing");
        assert_eq!(app.settings_state.selected_column_idx, 0);

        app.move_column_down().unwrap();
        assert_eq!(app.columns[0].name, "Ready");
    }

    #[test]
    fn test_settings_set_status() {
        let (mut app, _file) = setup_app();
        app.settings_state.selected_column_idx = 0;
        app.set_column_as_status("start").unwrap();

        let board = &app.boards[0];
        assert_eq!(board.start_column_id, Some(app.columns[0].id));
    }

    #[test]
    fn test_overview_scrolling() {
        let (mut app, _file) = setup_app();
        // Add logs
        for i in 0..10 {
            app.db.create_board(format!("B{}", i), "".into()).unwrap();
        }
        app.refresh_data().unwrap();

        app.overview_state.log_state.select(Some(0));
        app.scroll_logs_down();
        assert_eq!(app.overview_state.log_state.selected(), Some(1));

        app.scroll_logs_up();
        assert_eq!(app.overview_state.log_state.selected(), Some(0));
    }

    #[test]
    fn test_delete_task() {
        let (mut app, _file) = setup_app();
        let bid = app.boards[0].id;
        let cid = app.columns[0].id;

        app.db
            .create_task(bid, cid, "T1".into(), "".into(), None, None)
            .unwrap();
        app.refresh_data().unwrap();

        assert_eq!(app.tasks.len(), 1);

        app.delete_current_task().unwrap();
        assert_eq!(app.tasks.len(), 0);
    }
}
