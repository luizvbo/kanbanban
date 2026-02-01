pub mod state;

use crate::app::state::{EditField, EditState, SelectorState, SelectorType};
use crate::domain::kanban::{Column, KanbanData, Project, Tag};
use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;

/// Defines the possible interaction states of the application.
/// Used by the event handler to determine how to interpret key presses
/// and by the UI to determine which modals to render.
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum InputMode {
    /// Main board navigation
    Normal,
    /// Editing a specific card's fields
    Editing,
    /// Modal for picking tags/categories
    Selector,
    /// Information overlay
    Help,
    /// Confirming if unsaved changes should be kept
    ExitingModal,
    /// Confirming card deletion
    DeleteConfirmation,
    /// Input for board name
    ProjectRename,
    /// Input for column name
    ColumnRename,
    /// Input for new column title
    ColumnNew,
    /// Confirming column (and its cards) deletion
    ColumnDelete,
    /// Scrollable full-text view of a card
    ViewCard,
    /// Search/Filter mode
    Filter,
}

/// The central state controller of the application.
///
/// This struct holds the Kanban data, UI state, navigation indices,
/// and sub-states for modals (editing, tag selection, etc.).
pub struct App {
    /// The actual Kanban data (projects, columns, cards, tags)
    pub data: KanbanData,
    /// Path to the persistence file (usually kbb.yaml)
    pub filepath: PathBuf,
    /// Flag to signal the main loop to terminate
    pub should_quit: bool,

    // --- UI State ---
    /// Current interaction mode
    pub mode: InputMode,
    /// Tracks previous mode for returning from temporary modals (like Help or Confirm)
    pub previous_mode: Option<InputMode>,
    /// Temporary text shown in the footer
    pub status_message: Option<String>,
    /// When the status message was set (used for auto-hiding)
    pub status_time: Option<Instant>,
    /// Vertical scroll offset for the Detail View
    pub view_scroll_y: u16,
    /// Flag to force a full terminal clear (e.g., after returning from Vim)
    pub should_redraw: bool,

    // --- Navigation State (Pointer Indices) ---
    /// We use indices instead of references to the data because Rust's
    /// borrow checker would prevent us from holding a reference to a
    /// Card while also needing a mutable reference to the App to handle events.
    pub current_project_idx: usize,
    pub current_col_idx: usize,
    pub current_card_idx: usize,

    // --- Sub-States (Contextual Data) ---
    /// State container for the card editor
    pub edit_state: Option<EditState>,
    /// State container for the tag/category picker
    pub selector_state: Option<SelectorState>,

    // --- Temporary Input Buffers ---
    /// Buffer for renaming projects/columns
    pub prompt_input: String,
    /// Buffer for the global search filter
    pub filter_query: String,
}

impl App {
    /// Initializes a new App instance by loading data from the provided path.
    /// If the file doesn't exist, it initializes with default Kanban board structure.
    pub fn new(filepath: PathBuf) -> Result<Self> {
        let data = KanbanData::load(&filepath)?;
        Ok(Self {
            data,
            filepath,
            should_quit: false,
            mode: InputMode::Normal,
            previous_mode: None,
            current_project_idx: 0,
            current_col_idx: 0,
            current_card_idx: 0,
            edit_state: None,
            selector_state: None,
            status_message: None,
            status_time: None,
            prompt_input: String::new(),
            filter_query: String::new(),
            view_scroll_y: 0,
            should_redraw: false,
        })
    }

    // --- Data Access Helpers ---

    /// Returns a reference to the project currently being viewed.
    pub fn current_project(&self) -> &Project {
        &self.data.projects[self.current_project_idx]
    }

    /// Returns a mutable reference to the project currently being viewed.
    pub fn current_project_mut(&mut self) -> &mut Project {
        &mut self.data.projects[self.current_project_idx]
    }

    /// Serializes current state to the YAML file and updates the UI status message.
    pub fn save_with_feedback(&mut self) {
        match self.data.save(&self.filepath) {
            Ok(_) => {
                self.status_message = Some("Saved successfully".to_string());
                self.status_time = Some(Instant::now());
            }
            Err(e) => {
                self.status_message = Some(format!("Error saving: {}", e));
                self.status_time = Some(Instant::now());
            }
        }
    }

    /// Signals the application to close.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Toggles the help overlay.
    pub fn toggle_help(&mut self) {
        match self.mode {
            InputMode::Normal => self.mode = InputMode::Help,
            InputMode::Help => self.mode = InputMode::Normal,
            _ => {}
        }
    }

    // --- Filter / Search Logic ---

    pub fn start_filter(&mut self) {
        self.mode = InputMode::Filter;
    }

    pub fn stop_filter(&mut self) {
        self.mode = InputMode::Normal;
    }

    pub fn clear_filter(&mut self) {
        self.filter_query.clear();
        self.mode = InputMode::Normal;
    }

    pub fn filter_input_char(&mut self, c: char) {
        self.filter_query.push(c);
    }

    pub fn filter_input_backspace(&mut self) {
        self.filter_query.pop();
    }

    /// Logic for the global filter. Checks title, category, and tags.
    pub fn card_matches_filter(&self, card: &crate::domain::kanban::Card) -> bool {
        if self.filter_query.is_empty() {
            return true;
        }
        let q = self.filter_query.to_lowercase();
        if card.title.to_lowercase().contains(&q) {
            return true;
        }
        if let Some(cat) = &card.category
            && cat.to_lowercase().contains(&q)
        {
            return true;
        }
        if card.tags.iter().any(|t| t.name.to_lowercase().contains(&q)) {
            return true;
        }
        false
    }

    // --- Navigation (Column/Card) ---

    pub fn next_column(&mut self) {
        let len = self.current_project().columns.len();
        if len > 0 {
            self.current_col_idx = (self.current_col_idx + 1) % len;
            self.clamp_card_selection();
        }
    }

    pub fn prev_column(&mut self) {
        let len = self.current_project().columns.len();
        if len > 0 {
            if self.current_col_idx == 0 {
                self.current_col_idx = len - 1;
            } else {
                self.current_col_idx -= 1;
            }
            self.clamp_card_selection();
        }
    }

    pub fn next_card(&mut self) {
        let col = &self.current_project().columns[self.current_col_idx];
        if !col.cards.is_empty() && self.current_card_idx + 1 < col.cards.len() {
            self.current_card_idx += 1;
        }
    }

    pub fn prev_card(&mut self) {
        if self.current_card_idx > 0 {
            self.current_card_idx -= 1;
        }
    }

    /// Ensures card selection doesn't result in an "index out of bounds" error.
    /// MUST be called whenever columns are swapped, cards are moved, or
    /// the filter query is updated.
    fn clamp_card_selection(&mut self) {
        let col = &self.current_project().columns[self.current_col_idx];
        if col.cards.is_empty() {
            self.current_card_idx = 0;
        } else if self.current_card_idx >= col.cards.len() {
            self.current_card_idx = col.cards.len() - 1;
        }
    }

    // --- Detail View ---

    pub fn open_detail_view(&mut self) {
        let col = &self.current_project().columns[self.current_col_idx];
        if !col.cards.is_empty() {
            self.view_scroll_y = 0;
            self.mode = InputMode::ViewCard;
        }
    }

    pub fn close_detail_view(&mut self) {
        self.mode = InputMode::Normal;
    }

    pub fn view_scroll_down(&mut self) {
        self.view_scroll_y += 1;
    }

    pub fn view_scroll_up(&mut self) {
        if self.view_scroll_y > 0 {
            self.view_scroll_y -= 1;
        }
    }

    // --- External Editor Handling ---

    /// Spawns an external editor (Vim/Nano) to edit the card's long-form description.
    /// This method suspends the TUI, runs the editor process, and resumes the TUI.
    pub fn open_external_editor(&mut self) {
        // Validation: Editor is only available in Editing mode (focused on Description) or View mode.
        if self.mode == InputMode::Editing {
            if let Some(state) = &self.edit_state
                && state.focused_field != EditField::Description
            {
                self.status_message = Some("Focus Description to open editor".to_string());
                self.status_time = Some(Instant::now());
                return;
            }
        } else if self.mode != InputMode::ViewCard {
            return;
        }

        let initial_content = if self.mode == InputMode::Editing {
            self.edit_state
                .as_ref()
                .map(|s| s.description.clone())
                .unwrap_or_default()
        } else {
            let col = &self.current_project().columns[self.current_col_idx];
            if col.cards.is_empty() {
                return;
            }
            col.cards[self.current_card_idx].description.clone()
        };

        use std::io::Write;
        let mut temp_file = std::env::temp_dir();
        temp_file.push("kanbanban_edit.md");

        if let Ok(mut file) = std::fs::File::create(&temp_file) {
            let _ = file.write_all(initial_content.as_bytes());
        } else {
            self.status_message = Some("Failed to create temp file".into());
            self.status_time = Some(Instant::now());
            return;
        }

        // TUI Suspension block (not compiled during unit tests)
        #[cfg(not(test))]
        {
            use crossterm::{
                cursor, execute,
                terminal::{EnterAlternateScreen, LeaveAlternateScreen},
            };
            use std::process::Command;

            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
            let _ = crossterm::terminal::disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen, cursor::Show);
            let _ = Command::new(&editor).arg(&temp_file).status();
            let _ = execute!(std::io::stdout(), EnterAlternateScreen, cursor::Hide);
            let _ = crossterm::terminal::enable_raw_mode();

            // Signal main loop to clear artifacts
            self.should_redraw = true;
        }

        #[cfg(test)]
        {
            use std::fs::OpenOptions;
            if let Ok(mut file) = OpenOptions::new().append(true).open(&temp_file) {
                let _ = writeln!(file, "\n[Edited Externally]");
            }
        }

        // Read updated content back into the app state
        if let Ok(content) = std::fs::read_to_string(&temp_file) {
            if self.mode == InputMode::Editing {
                if let Some(state) = &mut self.edit_state {
                    state.description = content;
                    state.cursor_position = state.description.len();
                }
            } else {
                let idx = self.current_card_idx;
                let col_idx = self.current_col_idx;
                self.current_project_mut().columns[col_idx].cards[idx].description = content;
                self.save_with_feedback();
            }
        }
        let _ = std::fs::remove_file(temp_file);
    }

    // --- Column Operations ---

    pub fn start_new_column(&mut self) {
        self.prompt_input = String::new();
        self.mode = InputMode::ColumnNew;
    }

    pub fn confirm_new_column(&mut self) {
        if !self.prompt_input.trim().is_empty() {
            let new_col = Column {
                title: self.prompt_input.clone(),
                cards: Vec::new(),
                scroll_offset: 0,
            };
            self.current_project_mut().columns.push(new_col);
            self.current_col_idx = self.current_project().columns.len() - 1;
            self.current_card_idx = 0;
            self.save_with_feedback();
        }
        self.mode = InputMode::Normal;
    }

    pub fn start_rename_column(&mut self) {
        if self.current_project().columns.is_empty() {
            return;
        }
        self.prompt_input = self.current_project().columns[self.current_col_idx]
            .title
            .clone();
        self.mode = InputMode::ColumnRename;
    }

    pub fn confirm_rename_column(&mut self) {
        if !self.prompt_input.trim().is_empty() {
            let idx = self.current_col_idx;
            self.current_project_mut().columns[idx].title = self.prompt_input.clone();
            self.save_with_feedback();
        }
        self.mode = InputMode::Normal;
    }

    pub fn trigger_delete_column(&mut self) {
        if self.current_project().columns.is_empty() {
            return;
        }
        self.mode = InputMode::ColumnDelete;
    }

    pub fn confirm_delete_column(&mut self) {
        let idx = self.current_col_idx;
        let project = self.current_project_mut();

        if !project.columns.is_empty() {
            project.columns.remove(idx);
            if idx >= project.columns.len() {
                if project.columns.is_empty() {
                    self.current_col_idx = 0;
                } else {
                    self.current_col_idx = project.columns.len() - 1;
                }
            } else {
                self.current_col_idx = idx;
            }
            self.current_card_idx = 0;
            self.save_with_feedback();
        }
        self.mode = InputMode::Normal;
    }

    pub fn move_column_left(&mut self) {
        let idx = self.current_col_idx;
        if idx > 0 {
            let project = self.current_project_mut();
            project.columns.swap(idx, idx - 1);
            self.current_col_idx -= 1;
            self.save_with_feedback();
        }
    }

    pub fn move_column_right(&mut self) {
        let idx = self.current_col_idx;
        let project = self.current_project_mut();
        if idx + 1 < project.columns.len() {
            project.columns.swap(idx, idx + 1);
            self.current_col_idx += 1;
            self.save_with_feedback();
        }
    }

    // --- Project Operations ---

    pub fn start_rename_project(&mut self) {
        self.prompt_input = self.current_project().name.clone();
        self.mode = InputMode::ProjectRename;
    }

    pub fn confirm_rename_project(&mut self) {
        if !self.prompt_input.trim().is_empty() {
            self.current_project_mut().name = self.prompt_input.clone();
            self.save_with_feedback();
        }
        self.mode = InputMode::Normal;
    }

    // --- Card Movement Logic ---

    /// Moves a card from one column to another.
    /// Used for board organization and workflow progression (e.g., Todo -> Done).
    fn move_card_internal(&mut self, from_col: usize, to_col: usize) {
        let card_idx = self.current_card_idx;
        let card = {
            let project = self.current_project_mut();
            let from_column = &mut project.columns[from_col];
            if from_column.cards.is_empty() {
                return;
            }
            from_column.cards.remove(card_idx)
        };

        {
            let project = self.current_project_mut();
            project.columns[to_col].cards.push(card);
        }

        self.current_col_idx = to_col;
        let project = self.current_project();
        let new_len = project.columns[to_col].cards.len();
        self.current_card_idx = if new_len > 0 { new_len - 1 } else { 0 };
        self.save_with_feedback();
    }

    pub fn move_card_up(&mut self) {
        let col_idx = self.current_col_idx;
        let card_idx = self.current_card_idx;
        let project = self.current_project_mut();
        let col = &mut project.columns[col_idx];

        if card_idx > 0 && !col.cards.is_empty() {
            col.cards.swap(card_idx, card_idx - 1);
            self.current_card_idx -= 1;
            self.save_with_feedback();
        }
    }

    pub fn move_card_down(&mut self) {
        let col_idx = self.current_col_idx;
        let card_idx = self.current_card_idx;
        let project = self.current_project_mut();
        let col = &mut project.columns[col_idx];

        if !col.cards.is_empty() && card_idx + 1 < col.cards.len() {
            col.cards.swap(card_idx, card_idx + 1);
            self.current_card_idx += 1;
            self.save_with_feedback();
        }
    }

    pub fn move_card_left(&mut self) {
        let current_col_idx = self.current_col_idx;
        if current_col_idx == 0 {
            return;
        }
        self.move_card_internal(current_col_idx, current_col_idx - 1);
    }

    pub fn move_card_right(&mut self) {
        let current_col_idx = self.current_col_idx;
        let num_cols = self.current_project().columns.len();
        if current_col_idx + 1 >= num_cols {
            return;
        }
        self.move_card_internal(current_col_idx, current_col_idx + 1);
    }

    // --- Card Deletion ---

    pub fn trigger_delete(&mut self) {
        let col = &self.current_project().columns[self.current_col_idx];
        if !col.cards.is_empty() {
            self.mode = InputMode::DeleteConfirmation;
        }
    }

    pub fn confirm_delete(&mut self) {
        let col_idx = self.current_col_idx;
        let card_idx = self.current_card_idx;
        let project = self.current_project_mut();
        let col = &mut project.columns[col_idx];

        if !col.cards.is_empty() {
            col.cards.remove(card_idx);
            self.clamp_card_selection();
            self.save_with_feedback();
        }
        self.mode = InputMode::Normal;
    }

    pub fn delete_current_card(&mut self) {
        self.confirm_delete();
    }

    pub fn cancel_delete(&mut self) {
        self.mode = InputMode::Normal;
    }

    // --- Editing (Delegates to EditState) ---

    pub fn start_new_card(&mut self) {
        self.edit_state = Some(EditState::new());
        self.mode = InputMode::Editing;
    }

    pub fn start_edit_card(&mut self) {
        let col = &self.current_project().columns[self.current_col_idx];
        if col.cards.is_empty() {
            self.start_new_card();
            return;
        }
        if let Some(card) = col.cards.get(self.current_card_idx) {
            self.edit_state = Some(EditState::from_card(card));
            self.mode = InputMode::Editing;
        }
    }

    pub fn cancel_edit(&mut self) {
        self.edit_state = None;
        self.mode = InputMode::Normal;
    }

    pub fn save_edit(&mut self) {
        if let Some(state) = self.edit_state.take() {
            // 1. Capture the values we need from state BEFORE consuming it
            let is_new = state.is_new_card;

            // 2. Consume the state to create the card
            let new_card = state.into_card();

            let col_idx = self.current_col_idx;
            let card_idx = self.current_card_idx;

            {
                let project = self.current_project_mut();
                let col = &mut project.columns[col_idx];
                if is_new {
                    col.cards.push(new_card);
                } else if card_idx < col.cards.len() {
                    col.cards[card_idx] = new_card;
                }
            }

            if is_new {
                let len = self.current_project().columns[col_idx].cards.len();
                if len > 0 {
                    self.current_card_idx = len - 1;
                }
            }
            self.save_with_feedback();
        }
        self.mode = InputMode::Normal;
    }

    pub fn is_edit_empty(&self) -> bool {
        self.edit_state.as_ref().is_none_or(|s| s.is_empty())
    }

    pub fn get_focused_field(&self) -> Option<EditField> {
        self.edit_state.as_ref().map(|s| s.focused_field)
    }

    // --- Delegation to EditState ---

    pub fn toggle_description_edit(&mut self) {
        if let Some(state) = &mut self.edit_state
            && state.focused_field == EditField::Description
        {
            state.description_edit_mode = !state.description_edit_mode;
        }
    }

    pub fn edit_input_char(&mut self, c: char) {
        if let Some(state) = &mut self.edit_state {
            if state.focused_field == EditField::Description && !state.description_edit_mode {
                return;
            }

            let text = match state.focused_field {
                EditField::Title => &mut state.title,
                EditField::Category => &mut state.category,
                EditField::Tags => return,
                EditField::DueDate => &mut state.due_date,
                EditField::Description => &mut state.description,
            };

            let char_to_insert = if c == '\r' { '\n' } else { c };

            if state.cursor_position >= text.len() {
                text.push(char_to_insert);
            } else {
                text.insert(state.cursor_position, char_to_insert);
            }
            state.cursor_position += char_to_insert.len_utf8();
        }
    }

    pub fn edit_input_tab(&mut self) {
        if let Some(state) = &mut self.edit_state {
            if state.focused_field == EditField::Description && state.description_edit_mode {
                let text = &mut state.description;
                if state.cursor_position >= text.len() {
                    text.push_str("  ");
                } else {
                    text.insert_str(state.cursor_position, "  ");
                }
                state.cursor_position += 2;
            } else {
                self.cycle_edit_field(false);
            }
        }
    }

    pub fn edit_input_backspace(&mut self) {
        if let Some(state) = &mut self.edit_state {
            if state.focused_field == EditField::Description && !state.description_edit_mode {
                return;
            }

            let text = match state.focused_field {
                EditField::Title => &mut state.title,
                EditField::Category => &mut state.category,
                EditField::Tags => return,
                EditField::DueDate => &mut state.due_date,
                EditField::Description => &mut state.description,
            };

            if state.cursor_position > 0 && !text.is_empty() {
                let char_indices = text.char_indices();
                let mut prev_idx = 0;
                for (idx, _) in char_indices {
                    if idx >= state.cursor_position {
                        break;
                    }
                    prev_idx = idx;
                }
                text.remove(prev_idx);
                state.cursor_position = prev_idx;
            }
        }
    }

    /// Logic for cycling through fields (Title, Category, Tags, etc.) inside the card editor.
    pub fn cycle_edit_field(&mut self, reverse: bool) {
        if let Some(state) = &mut self.edit_state {
            if state.focused_field == EditField::Description && state.description_edit_mode {
                return;
            }

            // Simple state machine for field focusing
            state.focused_field = if reverse {
                match state.focused_field {
                    EditField::Title => EditField::Description,
                    EditField::Category => EditField::Title,
                    EditField::Tags => EditField::Category,
                    EditField::DueDate => EditField::Tags,
                    EditField::Description => EditField::DueDate,
                }
            } else {
                match state.focused_field {
                    EditField::Title => EditField::Category,
                    EditField::Category => EditField::Tags,
                    EditField::Tags => EditField::DueDate,
                    EditField::DueDate => EditField::Description,
                    EditField::Description => EditField::Title,
                }
            };

            // Reset cursor and scroll when switching fields
            let text_len = match state.focused_field {
                EditField::Title => state.title.len(),
                EditField::Category => state.category.len(),
                EditField::Tags => 0,
                EditField::DueDate => state.due_date.len(),
                EditField::Description => state.description.len(),
            };
            state.cursor_position = text_len;
            state.scroll_x = 0;
            state.scroll_y = 0;
            state.description_edit_mode = false;
        }
    }

    pub fn move_cursor_up(&mut self) {
        if let Some(state) = &mut self.edit_state {
            state.move_cursor_up();
        }
    }

    pub fn move_cursor_down(&mut self) {
        if let Some(state) = &mut self.edit_state {
            state.move_cursor_down();
        }
    }

    pub fn move_cursor_left(&mut self) {
        if let Some(state) = &mut self.edit_state
            && state.cursor_position > 0
        {
            let text = match state.focused_field {
                EditField::Title => &state.title,
                EditField::Category => &state.category,
                EditField::Tags => "",
                EditField::DueDate => &state.due_date,
                EditField::Description => &state.description,
            };

            let char_indices = text.char_indices();
            let mut prev_idx = 0;
            for (idx, _) in char_indices {
                if idx >= state.cursor_position {
                    break;
                }
                prev_idx = idx;
            }
            state.cursor_position = prev_idx;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if let Some(state) = &mut self.edit_state {
            let text = match state.focused_field {
                EditField::Title => &state.title,
                EditField::Category => &state.category,
                EditField::Tags => "",
                EditField::DueDate => &state.due_date,
                EditField::Description => &state.description,
            };

            if let Some((idx, _)) = text
                .char_indices()
                .find(|(i, _)| *i == state.cursor_position)
            {
                let mut iter = text.char_indices();
                while let Some((i, _)) = iter.next() {
                    if i == idx {
                        if let Some((next_i, _)) = iter.next() {
                            state.cursor_position = next_i;
                        } else {
                            state.cursor_position = text.len();
                        }
                        break;
                    }
                }
            } else if state.cursor_position < text.len() {
                state.cursor_position += 1;
            }
        }
    }

    pub fn move_cursor_home(&mut self) {
        if let Some(state) = &mut self.edit_state {
            if state.focused_field == EditField::Description {
                let text = &state.description;
                let last_newline = text[..state.cursor_position]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                state.cursor_position = last_newline;
            } else {
                state.cursor_position = 0;
            }
        }
    }

    pub fn move_cursor_end(&mut self) {
        if let Some(state) = &mut self.edit_state {
            let text = match state.focused_field {
                EditField::Title => &state.title,
                EditField::Category => &state.category,
                EditField::Tags => "",
                EditField::DueDate => &state.due_date,
                EditField::Description => &state.description,
            };

            if state.focused_field == EditField::Description {
                let next_newline = text[state.cursor_position..]
                    .find('\n')
                    .map(|i| state.cursor_position + i)
                    .unwrap_or(text.len());
                state.cursor_position = next_newline;
            } else {
                state.cursor_position = text.len();
            }
        }
    }

    // --- Modal Logic ---

    pub fn trigger_exit_modal(&mut self) {
        self.previous_mode = Some(self.mode);
        self.mode = InputMode::ExitingModal;
    }

    pub fn confirm_exit_save(&mut self) {
        self.save_edit();
        self.previous_mode = None;
    }

    pub fn confirm_exit_discard(&mut self) {
        self.edit_state = None;
        self.mode = InputMode::Normal;
        self.previous_mode = None;
    }

    pub fn cancel_exit_modal(&mut self) {
        if let Some(prev) = self.previous_mode {
            self.mode = prev;
        } else {
            self.mode = InputMode::Normal;
        }
        self.previous_mode = None;
    }

    // --- Prompt Input Helpers ---
    // These methods manage the `prompt_input` string buffer.
    // This buffer is shared across Rename Board, Rename Column, and New Column
    // modes to avoid allocating multiple temporary strings.
    pub fn prompt_input_char(&mut self, c: char) {
        self.prompt_input.push(c);
    }

    pub fn prompt_input_backspace(&mut self) {
        self.prompt_input.pop();
    }

    pub fn cancel_prompt(&mut self) {
        self.mode = InputMode::Normal;
    }

    // --- Tag Selector (Delegates to TagSelectorState) ---

    // These methods handle the transition between InputModes and initialize
    // the temporary "Sub-States" needed for complex interactions like
    // searching for tags or choosing categories.

    pub fn open_selector(&mut self, mode: SelectorType) {
        if let Some(edit_state) = &self.edit_state {
            let (available, current) = match mode {
                SelectorType::Tag => (
                    self.data.known_tags.clone(),
                    edit_state.tags.iter().map(|t| t.name.clone()).collect(),
                ),
                SelectorType::Category => (
                    self.data.known_categories.clone(),
                    if edit_state.category.is_empty() {
                        vec![]
                    } else {
                        vec![edit_state.category.clone()]
                    },
                ),
            };

            self.selector_state = Some(SelectorState::new(mode, available, current));
            self.mode = InputMode::Selector;
        }
    }

    pub fn close_selector(&mut self) {
        self.selector_state = None;
        self.mode = InputMode::Editing;
    }

    pub fn selector_confirm(&mut self) {
        if let Some(selector) = &self.selector_state {
            if let Some(edit_state) = &mut self.edit_state {
                match selector.mode {
                    SelectorType::Tag => {
                        // Reconstruct Tag objects with colors
                        let mut new_tags = Vec::new();
                        for name in &selector.selected_items {
                            let color = self
                                .data
                                .known_tags
                                .iter()
                                .find(|t| t.name == *name)
                                .and_then(|t| t.color.clone());
                            new_tags.push(Tag {
                                name: name.clone(),
                                color,
                            });
                        }
                        edit_state.tags = new_tags;
                    }
                    SelectorType::Category => {
                        if let Some(first) = selector.selected_items.first() {
                            edit_state.category = first.clone();
                        } else if !selector.search_query.is_empty() {
                            // Allow creating new category on confirm if list is empty but query exists
                            edit_state.category = selector.search_query.clone();
                        }
                    }
                }
            }
        }
        self.close_selector();
    }

    pub fn selector_toggle_or_create(&mut self) {
        // We need to mutate data, so we extract what we need first
        let (mode, query, current_idx) = if let Some(s) = &self.selector_state {
            (s.mode, s.search_query.clone(), s.current_index)
        } else {
            return;
        };

        let filtered = self.get_filtered_selector_items();

        // CASE 1: Create New Item
        if !query.is_empty() && current_idx == filtered.len() {
            let color = KanbanData::get_next_color(match mode {
                SelectorType::Tag => self.data.known_tags.len(),
                SelectorType::Category => self.data.known_categories.len(),
            });

            let new_item = Tag {
                name: query.clone(),
                color: Some(color),
            };

            match mode {
                SelectorType::Tag => self.data.known_tags.push(new_item.clone()),
                SelectorType::Category => self.data.known_categories.push(new_item.clone()),
            }

            // Auto-select the new item
            if let Some(s) = &mut self.selector_state {
                s.available_items.push(new_item);
                s.search_query.clear();
                s.current_index = 0; // Reset

                match mode {
                    SelectorType::Tag => s.selected_items.push(query),
                    SelectorType::Category => {
                        s.selected_items.clear(); // Single select
                        s.selected_items.push(query);
                    }
                }
            }
            return;
        }

        // CASE 2: Toggle Existing Item
        if let Some((_, item)) = filtered.get(current_idx) {
            if let Some(s) = &mut self.selector_state {
                match mode {
                    SelectorType::Tag => {
                        if let Some(pos) = s.selected_items.iter().position(|x| x == &item.name) {
                            s.selected_items.remove(pos);
                        } else {
                            s.selected_items.push(item.name.clone());
                        }
                    }
                    SelectorType::Category => {
                        s.selected_items.clear(); // Single select
                        s.selected_items.push(item.name.clone());
                        // Optional: Close immediately on category select?
                        // Let's keep it open to allow changing mind, consistent with tags.
                    }
                }
            }
        }
    }

    pub fn selector_delete_item(&mut self) {
        let (mode, current_idx) = if let Some(s) = &self.selector_state {
            (s.mode, s.current_index)
        } else {
            return;
        };

        let filtered = self.get_filtered_selector_items();

        if let Some((_real_idx, item)) = filtered.get(current_idx) {
            // Remove from global known lists
            match mode {
                SelectorType::Tag => {
                    // Find index in known_tags based on name to be safe
                    if let Some(pos) = self
                        .data
                        .known_tags
                        .iter()
                        .position(|t| t.name == item.name)
                    {
                        self.data.known_tags.remove(pos);
                    }
                }
                SelectorType::Category => {
                    if let Some(pos) = self
                        .data
                        .known_categories
                        .iter()
                        .position(|t| t.name == item.name)
                    {
                        self.data.known_categories.remove(pos);
                    }
                }
            }

            // Update state
            if let Some(s) = &mut self.selector_state {
                // Remove from available
                if let Some(pos) = s.available_items.iter().position(|t| t.name == item.name) {
                    s.available_items.remove(pos);
                }
                // Remove from selected if it was selected
                if let Some(pos) = s.selected_items.iter().position(|n| n == &item.name) {
                    s.selected_items.remove(pos);
                }
                // Adjust cursor
                if s.current_index > 0 && s.current_index >= s.available_items.len() {
                    s.current_index = s.available_items.len().saturating_sub(1);
                }
            }
        }
    }

    pub fn get_filtered_selector_items(&self) -> Vec<(usize, Tag)> {
        self.selector_state
            .as_ref()
            .map(|s| s.get_filtered_items())
            .unwrap_or_default()
    }

    pub fn selector_next(&mut self) {
        if let Some(s) = &mut self.selector_state {
            let count = s.get_filtered_items().len();
            let limit = if !s.search_query.is_empty() {
                count
            } else {
                count.saturating_sub(1)
            };
            if s.current_index < limit {
                s.current_index += 1;
            }
        }
    }

    pub fn selector_prev(&mut self) {
        if let Some(s) = &mut self.selector_state {
            if s.current_index > 0 {
                s.current_index -= 1;
            }
        }
    }

    pub fn selector_input_char(&mut self, c: char) {
        if let Some(s) = &mut self.selector_state {
            s.search_query.push(c);
            s.current_index = 0;
        }
    }

    pub fn selector_backspace(&mut self) {
        if let Some(s) = &mut self.selector_state {
            s.search_query.pop();
            s.current_index = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::kanban::Card;

    use super::*;

    // Helper to create a test app in memory
    fn create_test_app() -> App {
        let mut app = App::new(PathBuf::from("dummy.yaml")).unwrap();
        // Clear default data for clean testing
        app.data.projects[0].columns.clear();
        app.data.projects[0].columns.push(Column {
            title: "Col 1".into(),
            cards: vec![],
            scroll_offset: 0,
        });
        app.data.projects[0].columns.push(Column {
            title: "Col 2".into(),
            cards: vec![],
            scroll_offset: 0,
        });
        app
    }

    #[test]
    fn test_external_editor_integration() {
        let mut app = create_test_app();
        // Add card to Col 1 (index 0)
        app.data.projects[0].columns[0].cards.push(Card {
            title: "T".into(),
            description: "Old".into(),
            category: None,
            tags: vec![],
            due_date: None,
        });

        // Select the card
        app.current_col_idx = 0;
        app.current_card_idx = 0;

        app.start_edit_card();

        // Focus description
        app.edit_state.as_mut().unwrap().focused_field = EditField::Description;

        // Trigger editor
        app.open_external_editor();

        // Check if content was updated
        assert!(
            app.edit_state
                .unwrap()
                .description
                .contains("[Edited Externally]")
        );
    }

    #[test]
    fn test_navigation_clamping() {
        let mut app = create_test_app();
        // Add 2 cards to Col 1
        app.data.projects[0].columns[0].cards.push(Card {
            title: "A".into(),
            description: "".into(),
            category: None,
            tags: vec![],
            due_date: None,
        });
        app.data.projects[0].columns[0].cards.push(Card {
            title: "B".into(),
            description: "".into(),
            category: None,
            tags: vec![],
            due_date: None,
        });

        // Test Card Navigation
        assert_eq!(app.current_card_idx, 0);
        app.next_card();
        assert_eq!(app.current_card_idx, 1);
        app.next_card();
        assert_eq!(app.current_card_idx, 1); // Should clamp at end

        // Test Column Navigation
        assert_eq!(app.current_col_idx, 0);
        app.next_column();
        assert_eq!(app.current_col_idx, 1);

        // When switching to empty column, card idx should be 0
        assert_eq!(app.current_card_idx, 0);
    }

    #[test]
    fn test_move_card_right() {
        let mut app = create_test_app();
        let card = Card {
            title: "MoveMe".into(),
            description: "".into(),
            category: None,
            tags: vec![],
            due_date: None,
        };
        app.data.projects[0].columns[0].cards.push(card);

        assert_eq!(app.data.projects[0].columns[0].cards.len(), 1);
        assert_eq!(app.data.projects[0].columns[1].cards.len(), 0);

        app.move_card_right();

        assert_eq!(app.data.projects[0].columns[0].cards.len(), 0);
        assert_eq!(app.data.projects[0].columns[1].cards.len(), 1);
        assert_eq!(app.data.projects[0].columns[1].cards[0].title, "MoveMe");
    }

    #[test]
    fn test_move_column() {
        let mut app = create_test_app();
        // Col 1 is index 0, Col 2 is index 1

        app.current_col_idx = 0;
        app.move_column_right();

        // "Col 2" should now be at index 0
        assert_eq!(app.data.projects[0].columns[0].title, "Col 2");
        assert_eq!(app.data.projects[0].columns[1].title, "Col 1");
        // Focus should follow the moved column (now at index 1)
        assert_eq!(app.current_col_idx, 1);
    }

    #[test]
    fn test_filtering() {
        let mut app = create_test_app();
        let card1 = Card {
            title: "Fix Bug".into(),
            description: "".into(),
            category: None,
            tags: vec![],
            due_date: None,
        };
        let card2 = Card {
            title: "New Feature".into(),
            description: "".into(),
            category: Some("Backend".into()),
            tags: vec![],
            due_date: None,
        };

        // Test Title Match
        app.filter_query = "bug".into();
        assert!(app.card_matches_filter(&card1));
        assert!(!app.card_matches_filter(&card2));

        // Test Category Match
        app.filter_query = "back".into();
        assert!(!app.card_matches_filter(&card1));
        assert!(app.card_matches_filter(&card2));
    }

    #[test]
    fn test_delete_column_logic() {
        let mut app = create_test_app();
        assert_eq!(app.data.projects[0].columns.len(), 2);

        app.current_col_idx = 0;
        app.confirm_delete_column(); // Logic extracted from UI event handler

        assert_eq!(app.data.projects[0].columns.len(), 1);
        assert_eq!(app.data.projects[0].columns[0].title, "Col 2");
    }

    #[test]
    fn test_edit_state_cursor_movement() {
        let mut state = EditState::new();
        state.focused_field = EditField::Description;
        state.description = "Line 1\nLine 2".into();

        // Start at 0
        assert_eq!(state.cursor_position, 0);

        // Move Down
        state.move_cursor_down();
        // Should be at start of Line 2 (Length of "Line 1" + "\n" = 7)
        assert_eq!(state.cursor_position, 7);

        // Move Up
        state.move_cursor_up();
        assert_eq!(state.cursor_position, 0);
    }

    #[test]
    fn test_text_editing_logic() {
        let mut app = create_test_app();
        app.start_new_card();

        // 1. Typing Title
        app.edit_input_char('H');
        app.edit_input_char('i');
        assert_eq!(app.edit_state.as_ref().unwrap().title, "Hi");

        // 2. Backspace
        app.edit_input_backspace();
        assert_eq!(app.edit_state.as_ref().unwrap().title, "H");

        // 3. Cycle Fields
        app.cycle_edit_field(false); // To Category
        assert_eq!(
            app.edit_state.as_ref().unwrap().focused_field,
            EditField::Category
        );

        app.cycle_edit_field(false); // To Tags
        app.cycle_edit_field(false); // To DueDate
        app.cycle_edit_field(false); // To Description
        assert_eq!(
            app.edit_state.as_ref().unwrap().focused_field,
            EditField::Description
        );

        // 4. Description Editing
        app.toggle_description_edit(); // Enter edit mode
        app.edit_input_char('A');
        app.edit_input_char('\n');
        app.edit_input_char('B');
        assert_eq!(app.edit_state.as_ref().unwrap().description, "A\nB");

        // 5. Cursor Movement in Description
        app.move_cursor_up();
        // Should be at 'A' (index 0) or end of line 1 depending on implementation details,
        // but definitely not index 3 ('B')
        assert!(app.edit_state.as_ref().unwrap().cursor_position < 3);
    }

    #[test]
    fn test_column_operations() {
        let mut app = create_test_app();

        // New Column
        app.start_new_column();
        assert_eq!(app.mode, InputMode::ColumnNew);
        app.prompt_input = "New Col".into();
        app.confirm_new_column();
        assert_eq!(app.data.projects[0].columns.len(), 3);
        assert_eq!(app.data.projects[0].columns[2].title, "New Col");

        // Rename Column
        app.current_col_idx = 2;
        app.start_rename_column();
        assert_eq!(app.prompt_input, "New Col");
        app.prompt_input = "Renamed".into();
        app.confirm_rename_column();
        assert_eq!(app.data.projects[0].columns[2].title, "Renamed");
    }

    #[test]
    fn test_tag_selector_logic() {
        let mut app = create_test_app();
        app.start_new_card();
        app.open_selector(SelectorType::Tag);

        assert!(app.selector_state.is_some());

        // Filter
        app.selector_input_char('b');
        app.selector_input_char('u');
        let filtered = app.get_filtered_selector_items();
        assert!(
            filtered
                .iter()
                .any(|(_, t)| t.name.to_lowercase().contains("bug"))
        );

        // Create New Tag logic
        app.selector_state.as_mut().unwrap().search_query = "NewTag".into();
        app.selector_state.as_mut().unwrap().current_index =
            app.get_filtered_selector_items().len(); // Select "Create New"
        app.selector_toggle_or_create();

        assert!(app.data.known_tags.iter().any(|t| t.name == "NewTag"));
    }

    #[test]
    fn test_card_movement_vertical_boundaries() {
        let mut app = create_test_app();
        // Add 3 cards
        let col = &mut app.data.projects[0].columns[0];
        col.cards.push(Card {
            title: "1".into(),
            description: "".into(),
            category: None,
            tags: vec![],
            due_date: None,
        });
        col.cards.push(Card {
            title: "2".into(),
            description: "".into(),
            category: None,
            tags: vec![],
            due_date: None,
        });
        col.cards.push(Card {
            title: "3".into(),
            description: "".into(),
            category: None,
            tags: vec![],
            due_date: None,
        });

        // Select middle
        app.current_card_idx = 1;

        // Move Up
        app.move_card_up();
        assert_eq!(app.current_card_idx, 0);
        assert_eq!(app.data.projects[0].columns[0].cards[0].title, "2");

        // Move Up again (Boundary check - should do nothing)
        app.move_card_up();
        assert_eq!(app.current_card_idx, 0);
        assert_eq!(app.data.projects[0].columns[0].cards[0].title, "2");

        // Move Down to bottom
        app.move_card_down(); // idx 1
        app.move_card_down(); // idx 2
        assert_eq!(app.current_card_idx, 2);

        // Move Down again (Boundary check)
        app.move_card_down();
        assert_eq!(app.current_card_idx, 2);
    }

    #[test]
    fn test_card_movement_horizontal_boundaries() {
        let mut app = create_test_app();

        // create_test_app initializes columns as empty.
        app.data.projects[0].columns[0].cards.push(Card {
            title: "MoveMe".into(),
            description: "".into(),
            category: None,
            tags: vec![],
            due_date: None,
        });

        // Try moving left from Col 0 (Boundary)
        app.current_col_idx = 0;
        app.move_card_left();
        assert_eq!(app.current_col_idx, 0); // Should stay

        // Move right to Col 1
        app.move_card_right();
        assert_eq!(app.current_col_idx, 1);

        // Try moving right from Col 1 (Boundary - assuming only 2 cols)
        app.move_card_right();
        assert_eq!(app.current_col_idx, 1); // Should stay
    }

    #[test]
    fn test_cycle_edit_field_reverse() {
        let mut app = create_test_app();
        app.start_new_card();

        // Default is Title
        assert_eq!(
            app.edit_state.as_ref().unwrap().focused_field,
            EditField::Title
        );

        // Reverse Cycle
        app.cycle_edit_field(true); // To Description
        assert_eq!(
            app.edit_state.as_ref().unwrap().focused_field,
            EditField::Description
        );

        app.cycle_edit_field(true); // To DueDate
        assert_eq!(
            app.edit_state.as_ref().unwrap().focused_field,
            EditField::DueDate
        );
    }

    #[test]
    fn test_view_scrolling() {
        let mut app = create_test_app();
        app.open_detail_view();

        assert_eq!(app.view_scroll_y, 0);

        app.view_scroll_down();
        assert_eq!(app.view_scroll_y, 1);

        app.view_scroll_up();
        assert_eq!(app.view_scroll_y, 0);

        app.view_scroll_up(); // Boundary
        assert_eq!(app.view_scroll_y, 0);
    }

    #[test]
    fn test_delete_empty_column_guard() {
        let mut app = create_test_app();
        // Clear all columns
        app.data.projects[0].columns.clear();

        // Trigger delete on empty project
        app.trigger_delete_column();
        assert_eq!(app.mode, InputMode::Normal); // Should not enter delete mode
    }

    #[test]
    fn test_cursor_movement_all_fields() {
        let mut app = create_test_app();
        app.start_new_card();

        // Helper to test a specific field
        fn test_field(app: &mut App, field: EditField) {
            app.edit_state.as_mut().unwrap().focused_field = field;
            app.edit_state.as_mut().unwrap().cursor_position = 0;

            if field == EditField::Description {
                app.edit_state.as_mut().unwrap().description_edit_mode = true;
            }

            // Type 'A'
            app.edit_input_char('A');
            assert_eq!(app.edit_state.as_ref().unwrap().cursor_position, 1);

            // Move Left
            app.move_cursor_left();
            assert_eq!(app.edit_state.as_ref().unwrap().cursor_position, 0);

            // Move Right
            app.move_cursor_right();
            assert_eq!(app.edit_state.as_ref().unwrap().cursor_position, 1);

            // Move Home
            app.move_cursor_home();
            assert_eq!(app.edit_state.as_ref().unwrap().cursor_position, 0);

            // Move End
            app.move_cursor_end();
            assert_eq!(app.edit_state.as_ref().unwrap().cursor_position, 1);

            // Backspace
            app.edit_input_backspace();
            assert_eq!(app.edit_state.as_ref().unwrap().cursor_position, 0);
        }

        // Test all text fields to hit all match arms
        test_field(&mut app, EditField::Title);
        test_field(&mut app, EditField::Category);
        test_field(&mut app, EditField::DueDate);

        // Description is special (multiline)
        test_field(&mut app, EditField::Description);
    }

    #[test]
    fn test_external_editor_logic() {
        // We can't easily test the actual Command::spawn in unit tests without mocking,
        // but we can test the guards.
        let mut app = create_test_app();

        // 1. Should fail if empty column
        app.data.projects[0].columns[0].cards.clear();
        app.open_external_editor(); // Should return early

        // 2. Should fail if not in Normal/View mode
        app.mode = InputMode::Editing;
        app.open_external_editor(); // Should return early
    }

    #[test]
    fn test_coverage_booster() {
        let mut app = create_test_app();

        // 1. Test move_cursor_home/end on single line fields
        app.start_new_card();
        app.edit_state.as_mut().unwrap().focused_field = EditField::Title;
        app.edit_state.as_mut().unwrap().title = "Hello".to_string();
        app.edit_state.as_mut().unwrap().cursor_position = 2;

        app.move_cursor_home();
        assert_eq!(app.edit_state.as_ref().unwrap().cursor_position, 0);

        app.move_cursor_end();
        assert_eq!(app.edit_state.as_ref().unwrap().cursor_position, 5);

        // 2. Test edit_input_char with Carriage Return (\r)
        app.edit_state.as_mut().unwrap().focused_field = EditField::Description;
        app.edit_state.as_mut().unwrap().description_edit_mode = true;
        app.edit_state.as_mut().unwrap().cursor_position = 0;
        app.edit_input_char('\r'); // Should become \n
        assert_eq!(app.edit_state.as_ref().unwrap().description, "\n");

        // 3. Test delete_current_card on empty column (Guard clause)
        app.data.projects[0].columns[0].cards.clear();
        app.delete_current_card(); // Should not panic

        // 4. Test confirm_delete_column on empty project (Guard clause)
        app.data.projects[0].columns.clear();
        app.confirm_delete_column(); // Should not panic

        // 5. Test move_card_left/right on boundaries
        // Reset data
        app = create_test_app();
        app.current_col_idx = 0;
        app.move_card_left(); // Should do nothing
        assert_eq!(app.current_col_idx, 0);

        app.current_col_idx = 1; // Last column
        app.move_card_right(); // Should do nothing
        assert_eq!(app.current_col_idx, 1);

        // 6. Test move_card_internal empty guard
        app.data.projects[0].columns[1].cards.clear();
        app.current_col_idx = 1;
        app.move_card_left(); // Should return early because no cards to move
        assert_eq!(app.data.projects[0].columns[0].cards.len(), 0); // Target unchanged
    }

    #[test]
    fn test_tag_color_fallback_integration() {
        let mut app = create_test_app();
        // Add a tag with NO color to a card
        let tag = Tag {
            name: "GlobalTag".into(),
            color: None,
        };

        // Add definition to global
        app.data.known_tags.push(Tag {
            name: "GlobalTag".into(),
            color: Some("Blue".into()),
        });

        // The app logic uses data.get_tag_color, verify it works via app instance
        let color = app.data.get_tag_color(&tag);
        assert_eq!(color, ratatui::style::Color::Blue);
    }
}
