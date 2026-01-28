use crate::types::{Card, KanbanData, Project, Tag};
use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;

#[derive(PartialEq, Clone, Copy)]
pub enum InputMode {
    Normal,
    Editing,
    TagSelection,
    Help,
    ExitingModal,
    DeleteConfirmation,
}

#[derive(PartialEq, Clone, Copy)]
pub enum EditField {
    Title,
    Tags,
    DueDate,
    Description,
}

pub struct TagSelectorState {
    pub available_tags: Vec<Tag>,
    pub selected_indices: Vec<usize>,
    pub current_index: usize,
    pub search_query: String, // NEW: User input
}

pub struct EditState {
    pub title: String,
    pub tags: Vec<Tag>,
    pub due_date: String,
    pub description: String,
    pub focused_field: EditField,
    pub is_new_card: bool,
    pub cursor_position: usize,
    pub scroll_x: u16,
    pub scroll_y: u16,
    pub description_edit_mode: bool,
}

impl EditState {
    fn from_card(card: &Card) -> Self {
        Self {
            title: card.title.clone(),
            tags: card.tags.clone(),
            due_date: card.due_date.clone().unwrap_or_default(),
            description: card.description.clone(),
            focused_field: EditField::Title,
            is_new_card: false,
            cursor_position: card.title.len(),
            scroll_x: 0,
            scroll_y: 0,
            description_edit_mode: false,
        }
    }

    fn new() -> Self {
        Self {
            title: String::new(),
            tags: Vec::new(),
            due_date: String::new(),
            description: String::new(),
            focused_field: EditField::Title,
            is_new_card: true,
            cursor_position: 0,
            scroll_x: 0,
            scroll_y: 0,
            description_edit_mode: false,
        }
    }

    pub fn get_cursor_position_2d(&self, _width: u16) -> (u16, u16) {
        let text = match self.focused_field {
            EditField::Title => &self.title,
            EditField::Tags => return (0, 0),
            EditField::DueDate => &self.due_date,
            EditField::Description => &self.description,
        };

        if self.focused_field != EditField::Description {
            return (self.cursor_position as u16, 0);
        }

        let mut x = 0;
        let mut y = 0;

        for (i, c) in text.char_indices() {
            if i == self.cursor_position {
                return (x, y);
            }
            if c == '\n' {
                x = 0;
                y += 1;
            } else {
                if c == '\t' {
                    x += 4;
                } else {
                    x += 1;
                }
            }
        }
        (x, y)
    }

    pub fn move_cursor_up(&mut self) {
        if self.focused_field != EditField::Description {
            return;
        }

        let text = &self.description;
        let pos = self.cursor_position;

        // 1. Find start of current line
        let current_line_start = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);

        // 2. Calculate current column offset
        let col_offset = pos - current_line_start;

        if current_line_start == 0 {
            // Already on first line, move to start
            self.cursor_position = 0;
            return;
        }

        // 3. Find start of previous line
        let prev_line_end = current_line_start - 1;
        let prev_line_start = text[..prev_line_end]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let prev_line_len = prev_line_end - prev_line_start;

        // 4. Move to same column in previous line, clamping to length
        let new_col = std::cmp::min(col_offset, prev_line_len);
        self.cursor_position = prev_line_start + new_col;
    }

    // NEW: Move cursor visually down
    pub fn move_cursor_down(&mut self) {
        if self.focused_field != EditField::Description {
            return;
        }

        let text = &self.description;
        let pos = self.cursor_position;

        // 1. Find start of current line
        let current_line_start = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col_offset = pos - current_line_start;

        // 2. Find end of current line (start of next)
        let next_newline = text[pos..].find('\n').map(|i| pos + i);

        if let Some(newline_idx) = next_newline {
            let next_line_start = newline_idx + 1;
            if next_line_start >= text.len() {
                self.cursor_position = text.len();
                return;
            }

            // 3. Find length of next line
            let next_line_end = text[next_line_start..]
                .find('\n')
                .map(|i| next_line_start + i)
                .unwrap_or(text.len());
            let next_line_len = next_line_end - next_line_start;

            // 4. Move to same column
            let new_col = std::cmp::min(col_offset, next_line_len);
            self.cursor_position = next_line_start + new_col;
        } else {
            // On last line, move to end
            self.cursor_position = text.len();
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
    pub tag_selector_state: Option<TagSelectorState>,
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
            tag_selector_state: None,
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

    // --- Card Movement (Vertical) ---
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

    // --- Card Movement (Horizontal) ---
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

    // --- Actions ---
    pub fn trigger_delete(&mut self) {
        let col = &self.current_project().columns[self.current_col_idx];
        if !col.cards.is_empty() {
            self.mode = InputMode::DeleteConfirmation;
        }
    }

    // NEW: Confirm Delete
    pub fn confirm_delete(&mut self) {
        self.delete_current_card();
        self.mode = InputMode::Normal;
    }

    // NEW: Cancel Delete
    pub fn cancel_delete(&mut self) {
        self.mode = InputMode::Normal;
    }

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
            let new_card = Card {
                title: if state.title.is_empty() {
                    "Untitled".into()
                } else {
                    state.title.clone()
                },
                description: state.description.clone(),
                tags: state.tags,
                due_date: if state.due_date.is_empty() {
                    None
                } else {
                    Some(state.due_date.clone())
                },
            };

            let col_idx = self.current_col_idx;
            let card_idx = self.current_card_idx;
            let is_new = state.is_new_card;

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

    // --- Cursor & Text Editing ---
    pub fn toggle_description_edit(&mut self) {
        if let Some(state) = &mut self.edit_state {
            if state.focused_field == EditField::Description {
                state.description_edit_mode = !state.description_edit_mode;
            }
        }
    }

    pub fn edit_input_char(&mut self, c: char) {
        if let Some(state) = &mut self.edit_state {
            if state.focused_field == EditField::Description && !state.description_edit_mode {
                return;
            }

            let text = match state.focused_field {
                EditField::Title => &mut state.title,
                EditField::Tags => return,
                EditField::DueDate => &mut state.due_date,
                EditField::Description => &mut state.description,
            };

            // FIX: Normalize Carriage Return (\r) to Newline (\n)
            // This prevents visual desync where cursor moves but text doesn't wrap.
            let char_to_insert = if c == '\r' { '\n' } else { c };

            if state.cursor_position >= text.len() {
                text.push(char_to_insert);
            } else {
                text.insert(state.cursor_position, char_to_insert);
            }

            // Increment by the actual length of the inserted char
            state.cursor_position += char_to_insert.len_utf8();
        }
    }

    // NEW: Handle Tab key specifically
    pub fn edit_input_tab(&mut self) {
        if let Some(state) = &mut self.edit_state {
            if state.focused_field == EditField::Description {
                // Insert 2 spaces for Markdown indentation
                let text = &mut state.description;
                if state.cursor_position >= text.len() {
                    text.push_str("  ");
                } else {
                    text.insert_str(state.cursor_position, "  ");
                }
                state.cursor_position += 2;
            } else {
                // For other fields, Tab cycles to next field
                self.cycle_edit_field(false);
            }
        }
    }

    pub fn edit_input_backspace(&mut self) {
        if let Some(state) = &mut self.edit_state {
            // FIX: Prevent backspace in Description if not in edit mode
            if state.focused_field == EditField::Description && !state.description_edit_mode {
                return;
            }

            let text = match state.focused_field {
                EditField::Title => &mut state.title,
                EditField::Tags => return,
                EditField::DueDate => &mut state.due_date,
                EditField::Description => &mut state.description,
            };

            if state.cursor_position > 0 && !text.is_empty() {
                let mut char_indices = text.char_indices();
                let mut prev_idx = 0;
                while let Some((idx, _)) = char_indices.next() {
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
        if let Some(state) = &mut self.edit_state {
            if state.cursor_position > 0 {
                let text = match state.focused_field {
                    EditField::Title => &state.title,
                    EditField::Tags => "",
                    EditField::DueDate => &state.due_date,
                    EditField::Description => &state.description,
                };

                // Move back by one char boundary
                let mut char_indices = text.char_indices();
                let mut prev_idx = 0;
                while let Some((idx, _)) = char_indices.next() {
                    if idx >= state.cursor_position {
                        break;
                    }
                    prev_idx = idx;
                }
                state.cursor_position = prev_idx;
            }
        }
    }

    pub fn move_cursor_right(&mut self) {
        if let Some(state) = &mut self.edit_state {
            let text = match state.focused_field {
                EditField::Title => &state.title,
                EditField::Tags => "",
                EditField::DueDate => &state.due_date,
                EditField::Description => &state.description,
            };

            // Move forward by one char boundary
            if let Some((idx, _)) = text
                .char_indices()
                .find(|(i, _)| *i == state.cursor_position)
            {
                // Find next char
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
                // Fallback
                state.cursor_position += 1;
            }
        }
    }

    pub fn move_cursor_home(&mut self) {
        if let Some(state) = &mut self.edit_state {
            // If in description, move to start of LINE, not start of text
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
                EditField::Tags => "",
                EditField::DueDate => &state.due_date,
                EditField::Description => &state.description,
            };

            if state.focused_field == EditField::Description {
                // Move to end of LINE
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

    pub fn cycle_edit_field(&mut self, reverse: bool) {
        if let Some(state) = &mut self.edit_state {
            if state.focused_field == EditField::Description && state.description_edit_mode {
                return;
            }

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

            let text_len = match state.focused_field {
                EditField::Title => state.title.len(),
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

    // --- Tag Selector Logic ---
    pub fn open_tag_selector(&mut self) {
        if let Some(state) = &self.edit_state {
            // 1. Load tags from global data
            let available_tags = self.data.known_tags.clone();

            // 2. Determine which are currently selected
            let mut selected_indices = Vec::new();
            for (i, avail) in available_tags.iter().enumerate() {
                if state.tags.iter().any(|t| t.name == avail.name) {
                    selected_indices.push(i);
                }
            }

            self.tag_selector_state = Some(TagSelectorState {
                available_tags,
                selected_indices,
                current_index: 0,
                search_query: String::new(),
            });
            self.mode = InputMode::TagSelection;
        }
    }

    pub fn tag_selector_input_char(&mut self, c: char) {
        if let Some(selector) = &mut self.tag_selector_state {
            selector.search_query.push(c);
            // Reset selection when searching to avoid out of bounds
            selector.current_index = 0;
        }
    }

    pub fn tag_selector_backspace(&mut self) {
        if let Some(selector) = &mut self.tag_selector_state {
            selector.search_query.pop();
            selector.current_index = 0;
        }
    }

    pub fn get_filtered_tags(&self) -> Vec<(usize, Tag)> {
        if let Some(selector) = &self.tag_selector_state {
            let query = selector.search_query.to_lowercase();
            selector
                .available_tags
                .iter()
                .enumerate()
                .filter(|(_, tag)| tag.name.to_lowercase().contains(&query))
                .map(|(i, tag)| (i, tag.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn close_tag_selector(&mut self) {
        self.tag_selector_state = None;
        self.mode = InputMode::Editing;
    }

    pub fn tag_selector_next(&mut self) {
        let filtered_len = self.get_filtered_tags().len();
        if let Some(selector) = &mut self.tag_selector_state {
            // If we are at the end of the list, and there is a search query,
            // allow going one step further to the "Create New" button (represented by len)
            let limit = if !selector.search_query.is_empty() {
                filtered_len
            } else {
                filtered_len.saturating_sub(1)
            };

            if selector.current_index < limit {
                selector.current_index += 1;
            }
        }
    }

    pub fn tag_selector_prev(&mut self) {
        if let Some(selector) = &mut self.tag_selector_state {
            if selector.current_index > 0 {
                selector.current_index -= 1;
            }
        }
    }

    pub fn tag_selector_toggle_or_create(&mut self) {
        let filtered = self.get_filtered_tags();

        // Check if we are selecting the "Create New" option
        if let Some(selector) = &mut self.tag_selector_state {
            if !selector.search_query.is_empty() && selector.current_index == filtered.len() {
                // CREATE NEW TAG
                let new_name = selector.search_query.clone();
                // Simple color cycling logic for new tags
                let colors = ["Red", "Green", "Blue", "Yellow", "Magenta", "Cyan"];
                let color_choice = colors[self.data.known_tags.len() % colors.len()];

                let new_tag = Tag {
                    name: new_name,
                    color: color_choice.to_string(),
                };

                // Add to global data
                self.data.known_tags.push(new_tag.clone());

                // Add to selection
                let new_idx = self.data.known_tags.len() - 1;
                selector.selected_indices.push(new_idx);

                // Reset search
                selector.search_query.clear();
                selector.available_tags = self.data.known_tags.clone();
                selector.current_index = 0;
                return;
            }
        }

        // Normal Toggle Logic
        if let Some((real_idx, _)) =
            filtered.get(self.tag_selector_state.as_ref().unwrap().current_index)
        {
            if let Some(selector) = &mut self.tag_selector_state {
                if let Some(pos) = selector
                    .selected_indices
                    .iter()
                    .position(|&x| x == *real_idx)
                {
                    selector.selected_indices.remove(pos);
                } else {
                    selector.selected_indices.push(*real_idx);
                }
            }
        }
    }

    pub fn tag_selector_toggle(&mut self) {
        if let Some(selector) = &mut self.tag_selector_state {
            let idx = selector.current_index;
            if let Some(pos) = selector.selected_indices.iter().position(|&x| x == idx) {
                selector.selected_indices.remove(pos);
            } else {
                selector.selected_indices.push(idx);
            }
        }
    }

    pub fn tag_selector_confirm(&mut self) {
        if let Some(selector) = &self.tag_selector_state {
            if let Some(edit_state) = &mut self.edit_state {
                edit_state.tags.clear();
                for &idx in &selector.selected_indices {
                    if let Some(tag) = selector.available_tags.get(idx) {
                        edit_state.tags.push(tag.clone());
                    }
                }
            }
        }
        self.close_tag_selector();
    }
}
