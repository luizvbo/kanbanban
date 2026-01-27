use crate::types::{Card, KanbanData, Project, Tag, TagColor};
use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant; // Removed Duration, Local

#[derive(PartialEq, Clone, Copy)]
pub enum InputMode {
    Normal,
    Editing,
    Help,
    ExitingModal,
}

#[derive(PartialEq, Clone, Copy)]
pub enum EditField {
    Title,
    Tags,
    DueDate,
    Description,
}

pub struct EditState {
    pub title: String,
    pub tags: String,
    pub due_date: String,
    pub description: String,
    pub focused_field: EditField,
    pub is_new_card: bool,
}

impl EditState {
    fn from_card(card: &Card) -> Self {
        let tags_str = card
            .tags
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        Self {
            title: card.title.clone(),
            tags: tags_str,
            due_date: card.due_date.clone().unwrap_or_default(),
            description: card.description.clone(),
            focused_field: EditField::Title,
            is_new_card: false,
        }
    }

    fn new() -> Self {
        Self {
            title: String::new(),
            tags: String::new(),
            due_date: String::new(),
            description: String::new(),
            focused_field: EditField::Title,
            is_new_card: true,
        }
    }
}

pub struct App {
    pub data: KanbanData,
    pub filepath: PathBuf,
    pub should_quit: bool,
    pub mode: InputMode,
    pub previous_mode: Option<InputMode>,
    pub current_project_idx: usize,
    pub current_col_idx: usize,
    pub current_card_idx: usize,
    pub edit_state: Option<EditState>,
    pub status_message: Option<String>,
    pub status_time: Option<Instant>,
}

impl App {
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
            status_message: None,
            status_time: None,
        })
    }

    pub fn current_project(&self) -> &Project {
        &self.data.projects[self.current_project_idx]
    }

    pub fn current_project_mut(&mut self) -> &mut Project {
        &mut self.data.projects[self.current_project_idx]
    }

    // --- Navigation ---
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
        if !col.cards.is_empty() {
            if self.current_card_idx + 1 < col.cards.len() {
                self.current_card_idx += 1;
            }
        }
    }

    pub fn prev_card(&mut self) {
        if self.current_card_idx > 0 {
            self.current_card_idx -= 1;
        }
    }

    fn clamp_card_selection(&mut self) {
        let col = &self.current_project().columns[self.current_col_idx];
        if col.cards.is_empty() {
            self.current_card_idx = 0;
        } else if self.current_card_idx >= col.cards.len() {
            self.current_card_idx = col.cards.len() - 1;
        }
    }

    // --- Actions ---

    pub fn delete_current_card(&mut self) {
        let col_idx = self.current_col_idx;
        let card_idx = self.current_card_idx;
        let project = self.current_project_mut();
        let col = &mut project.columns[col_idx];

        if !col.cards.is_empty() {
            col.cards.remove(card_idx);
            self.clamp_card_selection();
            self.save_with_feedback();
        }
    }

    pub fn toggle_help(&mut self) {
        match self.mode {
            InputMode::Normal => self.mode = InputMode::Help,
            InputMode::Help => self.mode = InputMode::Normal,
            _ => {}
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

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

    // --- Editing Logic ---

    pub fn get_focused_field(&self) -> Option<EditField> {
        self.edit_state.as_ref().map(|s| s.focused_field)
    }

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
            let tags: Vec<Tag> = state
                .tags
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| Tag {
                    name: s.to_string(),
                    color: TagColor::Gray,
                })
                .collect();

            let new_card = Card {
                title: if state.title.is_empty() {
                    "Untitled".into()
                } else {
                    state.title.clone()
                },
                description: state.description.clone(),
                tags,
                due_date: if state.due_date.is_empty() {
                    None
                } else {
                    Some(state.due_date.clone())
                },
            };

            let col_idx = self.current_col_idx;
            let card_idx = self.current_card_idx; // FIX: Capture index before mutable borrow
            let is_new = state.is_new_card;

            {
                let project = self.current_project_mut();
                let col = &mut project.columns[col_idx];

                if is_new {
                    col.cards.push(new_card);
                } else {
                    // Use the captured card_idx here
                    if card_idx < col.cards.len() {
                        col.cards[card_idx] = new_card;
                    }
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

    pub fn edit_input_char(&mut self, c: char) {
        if let Some(state) = &mut self.edit_state {
            match state.focused_field {
                EditField::Title => state.title.push(c),
                EditField::Tags => state.tags.push(c),
                EditField::DueDate => state.due_date.push(c),
                EditField::Description => state.description.push(c),
            }
        }
    }

    pub fn edit_input_backspace(&mut self) {
        if let Some(state) = &mut self.edit_state {
            match state.focused_field {
                EditField::Title => {
                    state.title.pop();
                }
                EditField::Tags => {
                    state.tags.pop();
                }
                EditField::DueDate => {
                    state.due_date.pop();
                }
                EditField::Description => {
                    state.description.pop();
                }
            }
        }
    }

    pub fn cycle_edit_field(&mut self, reverse: bool) {
        if let Some(state) = &mut self.edit_state {
            state.focused_field = if reverse {
                match state.focused_field {
                    EditField::Title => EditField::Description,
                    EditField::Tags => EditField::Title,
                    EditField::DueDate => EditField::Tags,
                    EditField::Description => EditField::DueDate,
                }
            } else {
                match state.focused_field {
                    EditField::Title => EditField::Tags,
                    EditField::Tags => EditField::DueDate,
                    EditField::DueDate => EditField::Description,
                    EditField::Description => EditField::Title,
                }
            };
        }
    }

    // --- Move Card Logic ---

    pub fn move_card_left(&mut self) {
        let current_col_idx = self.current_col_idx;
        if current_col_idx == 0 {
            return;
        }

        let prev_col_idx = current_col_idx - 1;
        self.move_card_internal(current_col_idx, prev_col_idx);
    }

    pub fn move_card_right(&mut self) {
        let current_col_idx = self.current_col_idx;
        let num_cols = self.current_project().columns.len();
        if current_col_idx + 1 >= num_cols {
            return;
        }

        let next_col_idx = current_col_idx + 1;
        self.move_card_internal(current_col_idx, next_col_idx);
    }

    fn move_card_internal(&mut self, from_col: usize, to_col: usize) {
        // 1. Capture index to avoid borrow conflict
        let card_idx = self.current_card_idx;

        // 2. Remove the card (Scope the borrow of self/project)
        let card = {
            let project = self.current_project_mut();
            let from_column = &mut project.columns[from_col];
            if from_column.cards.is_empty() {
                return;
            }
            from_column.cards.remove(card_idx)
        };

        // 3. Add to new column (New borrow scope)
        {
            let project = self.current_project_mut();
            project.columns[to_col].cards.push(card);
        }

        // 4. Update selection logic (No mutable borrows active here)
        self.current_col_idx = to_col;

        // Calculate new index safely
        let project = self.current_project(); // Immutable borrow is fine now
        let new_len = project.columns[to_col].cards.len();
        self.current_card_idx = if new_len > 0 { new_len - 1 } else { 0 };

        self.save_with_feedback();
    }
}
