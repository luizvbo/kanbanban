use crate::domain::kanban::{Card, Tag};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum EditField {
    Title,
    Category,
    Tags,
    DueDate,
    Description,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SelectorType {
    Tag,
    Category,
}

pub struct SelectorState {
    pub mode: SelectorType,
    pub available_items: Vec<Tag>,   // Stores both Name and Color
    pub selected_items: Vec<String>, // Stores names of selected items
    pub current_index: usize,
    pub search_query: String,
}

impl SelectorState {
    pub fn new(
        mode: SelectorType,
        available_items: Vec<Tag>,
        current_selection: Vec<String>,
    ) -> Self {
        Self {
            mode,
            available_items,
            selected_items: current_selection,
            current_index: 0,
            search_query: String::new(),
        }
    }

    pub fn get_filtered_items(&self) -> Vec<(usize, Tag)> {
        let query = self.search_query.to_lowercase();
        self.available_items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.name.to_lowercase().contains(&query))
            .map(|(i, item)| (i, item.clone()))
            .collect()
    }
}

/// Temporary state used when creating or editing a card.
/// We don't edit the Card struct directly to allow "Cancel without saving."
pub struct EditState {
    pub title: String,
    pub category: String,
    pub tags: Vec<Tag>,
    pub due_date: String,
    pub description: String,
    pub focused_field: EditField,
    pub is_new_card: bool,
    pub cursor_position: usize,
    pub scroll_x: u16,
    pub scroll_y: u16,
    /// When true, Enter key inserts a newline. When false, Enter cycles fields.
    pub description_edit_mode: bool,
}

impl Default for EditState {
    fn default() -> Self {
        Self::new()
    }
}

impl EditState {
    pub fn from_card(card: &Card) -> Self {
        Self {
            title: card.title.clone(),
            category: card.category.clone().unwrap_or_default(),
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

    pub fn new() -> Self {
        Self {
            title: String::new(),
            category: String::new(),
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

    /// Converts the current "draft" back into a Card struct for persistence.
    pub fn into_card(self) -> Card {
        Card {
            title: if self.title.is_empty() {
                "Untitled".into()
            } else {
                self.title
            },
            category: if self.category.is_empty() {
                None
            } else {
                Some(self.category)
            },
            description: self.description,
            tags: self.tags,
            due_date: if self.due_date.is_empty() {
                None
            } else {
                Some(self.due_date)
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.title.trim().is_empty() && self.description.trim().is_empty() && self.tags.is_empty()
    }

    /// Calculates (x, y) coordinates for the terminal cursor.
    /// Necessary for multi-line description editing with proper wrapping.
    pub fn get_cursor_position_2d(&self, _width: u16) -> (u16, u16) {
        let text = match self.focused_field {
            EditField::Title => &self.title,
            EditField::Category => &self.category,
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
            } else if c == '\t' {
                x += 4;
            } else {
                x += 1;
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

        let current_line_start = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col_offset = pos - current_line_start;

        if current_line_start == 0 {
            self.cursor_position = 0;
            return;
        }

        let prev_line_end = current_line_start - 1;
        let prev_line_start = text[..prev_line_end]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let prev_line_len = prev_line_end - prev_line_start;

        let new_col = std::cmp::min(col_offset, prev_line_len);
        self.cursor_position = prev_line_start + new_col;
    }

    pub fn move_cursor_down(&mut self) {
        if self.focused_field != EditField::Description {
            return;
        }

        let text = &self.description;
        let pos = self.cursor_position;

        let current_line_start = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col_offset = pos - current_line_start;

        let next_newline = text[pos..].find('\n').map(|i| pos + i);

        if let Some(newline_idx) = next_newline {
            let next_line_start = newline_idx + 1;
            if next_line_start >= text.len() {
                self.cursor_position = text.len();
                return;
            }

            let next_line_end = text[next_line_start..]
                .find('\n')
                .map(|i| next_line_start + i)
                .unwrap_or(text.len());
            let next_line_len = next_line_end - next_line_start;

            let new_col = std::cmp::min(col_offset, next_line_len);
            self.cursor_position = next_line_start + new_col;
        } else {
            self.cursor_position = text.len();
        }
    }
}
