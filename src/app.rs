use crate::types::{KanbanData, Project};
use anyhow::Result;
use std::path::PathBuf;

pub enum InputMode {
    Normal,
    Editing,
    Help,
}

pub struct App {
    pub data: KanbanData,
    pub filepath: PathBuf,
    pub should_quit: bool,
    pub mode: InputMode,

    // Navigation State
    pub current_project_idx: usize,
    pub current_col_idx: usize,
    pub current_card_idx: usize,
}

impl App {
    pub fn new(filepath: PathBuf) -> Result<Self> {
        let data = KanbanData::load(&filepath)?;
        Ok(Self {
            data,
            filepath,
            should_quit: false,
            mode: InputMode::Normal,
            current_project_idx: 0,
            current_col_idx: 0,
            current_card_idx: 0,
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
        // FIX: Capture indices before mutable borrow
        let col_idx = self.current_col_idx;
        let card_idx = self.current_card_idx;

        let project = self.current_project_mut();
        let col = &mut project.columns[col_idx];

        if !col.cards.is_empty() {
            col.cards.remove(card_idx);
            self.clamp_card_selection();
            let _ = self.save();
        }
    }

    pub fn toggle_edit(&mut self) {
        match self.mode {
            InputMode::Normal => self.mode = InputMode::Editing,
            InputMode::Editing => self.mode = InputMode::Normal,
            _ => {}
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

    pub fn save(&self) -> Result<()> {
        self.data.save(&self.filepath)
    }
}
