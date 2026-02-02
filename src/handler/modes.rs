use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::state::{EditField, SelectorType};
use crate::app::{App, InputMode};

/// Reacts to keyboard input when the app is in 'Normal' mode (navigating the board).
pub fn handle_normal(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.quit(),
        // Shift+Arrows for movement
        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => app.move_card_down(),
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => app.move_card_up(),
        KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => app.move_card_left(),
        KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => app.move_card_right(),
        // Navigation (Vim + Arrows)
        KeyCode::Char('h') | KeyCode::Left => app.prev_column(),
        KeyCode::Char('l') | KeyCode::Right => app.next_column(),
        KeyCode::Char('j') | KeyCode::Down => app.next_card(),
        KeyCode::Char('k') | KeyCode::Up => app.prev_card(),
        // Movement (Vim + Arrows)
        KeyCode::Char('J') => app.move_card_down(),
        KeyCode::Char('K') => app.move_card_up(),
        KeyCode::Char('H') => app.move_card_left(),
        KeyCode::Char('L') => app.move_card_right(),

        KeyCode::Char('<') | KeyCode::Char(',') => app.move_column_left(),
        KeyCode::Char('>') | KeyCode::Char('.') => app.move_column_right(),
        KeyCode::Char('d') => app.trigger_delete(),
        KeyCode::Char('a') | KeyCode::Char('n') => app.start_new_card(),
        KeyCode::Char('i') | KeyCode::Char('e') | KeyCode::Enter => app.start_edit_card(),
        KeyCode::Char('?') => app.toggle_help(),
        KeyCode::Char('R') => app.start_rename_project(),
        KeyCode::Char('r') => app.start_rename_column(),
        KeyCode::Char('c') => app.start_new_column(),
        KeyCode::Char('D') => app.trigger_delete_column(),

        KeyCode::Char('/') => app.start_filter(),
        KeyCode::Esc => app.clear_filter(),
        KeyCode::Char('v') => app.open_detail_view(),
        KeyCode::Char('o') => app.open_external_editor(),
        _ => {}
    }
}

pub fn handle_filter(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => app.stop_filter(),
        KeyCode::Esc => app.clear_filter(),
        KeyCode::Backspace => app.filter_input_backspace(),
        KeyCode::Char(c) => app.filter_input_char(c),
        _ => {}
    }
}

pub fn handle_view_card(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('v') => app.close_detail_view(),

        // Basic Scroll
        KeyCode::Char('j') | KeyCode::Down => app.view_scroll_down(),
        KeyCode::Char('k') | KeyCode::Up => app.view_scroll_up(),

        // Vim Movements
        KeyCode::Char('g') => app.view_scroll_y = 0, // Top
        KeyCode::Char('G') => app.view_scroll_y = u16::MAX, // Bottom (Ratatui handles clamping usually, or we need logic)
        KeyCode::Home => app.view_scroll_y = 0,
        KeyCode::End => app.view_scroll_y = u16::MAX,

        // Page Up/Down (Ctrl+u / Ctrl+d)
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.view_scroll_y = app.view_scroll_y.saturating_sub(10);
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.view_scroll_y = app.view_scroll_y.saturating_add(10);
        }
        KeyCode::PageUp => app.view_scroll_y = app.view_scroll_y.saturating_sub(10),
        KeyCode::PageDown => app.view_scroll_y = app.view_scroll_y.saturating_add(10),

        KeyCode::Char('o') => app.open_external_editor(),
        _ => {}
    }
}

pub fn handle_help(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.toggle_help(),
        _ => {}
    }
}

pub fn handle_selector(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.selector_confirm(), // Save & Close
        KeyCode::Enter => app.selector_toggle_or_create(),
        KeyCode::Delete | KeyCode::Char('x') => app.selector_delete_item(), // Added Delete support
        KeyCode::Down => app.selector_next(),
        KeyCode::Up => app.selector_prev(),
        KeyCode::Backspace => app.selector_backspace(),
        KeyCode::Char(c) => app.selector_input_char(c),
        _ => {}
    }
}

pub fn handle_exiting_modal(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.confirm_exit_save(),
        KeyCode::Char('n') | KeyCode::Char('N') => app.confirm_exit_discard(),
        KeyCode::Esc => app.cancel_exit_modal(),
        _ => {}
    }
}

pub fn handle_delete_confirmation(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.confirm_delete(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_delete(),
        _ => {}
    }
}

/// Handles the complex logic of the card editor.
/// Note: This differentiates between moving the cursor/cycling fields
/// and actually typing text into the description.
pub fn handle_editing(app: &mut App, key: KeyEvent) {
    let is_field_active = app
        .edit_state
        .as_ref()
        .map(|s| s.is_editing_field)
        .unwrap_or(false);

    let focused = app.get_focused_field();

    match key.code {
        // --- 1. Exit / Cancel ---
        KeyCode::Esc => {
            if is_field_active {
                // Exit Insert Mode -> Return to Navigation
                if let Some(state) = &mut app.edit_state {
                    state.is_editing_field = false;
                    state.description_edit_mode = false;
                }
            } else if !app.is_edit_empty() {
                app.trigger_exit_modal();
            } else {
                app.cancel_edit();
            }
        }

        // --- 2. Enter (Toggle Mode OR Insert Newline) ---
        KeyCode::Enter => {
            // Special Case: If already editing Description, Enter = Newline
            if is_field_active && focused == Some(EditField::Description) {
                app.edit_input_char('\n');
                return;
            }

            // Otherwise: Toggle Mode or Open Selector
            match focused {
                Some(EditField::Tags) => app.open_selector(SelectorType::Tag),
                Some(EditField::Category) => app.open_selector(SelectorType::Category),
                _ => {
                    // Toggle Insert Mode for Title, Date, Description
                    if let Some(state) = &mut app.edit_state {
                        state.is_editing_field = !state.is_editing_field;

                        // Sync description specific flag
                        if state.focused_field == EditField::Description {
                            state.description_edit_mode = state.is_editing_field;
                        }
                    }
                }
            }
        }

        // --- 3. Navigation (Only when NOT editing a field) ---
        KeyCode::Char('j') | KeyCode::Down if !is_field_active => app.cycle_edit_field(false),
        KeyCode::Char('k') | KeyCode::Up if !is_field_active => app.cycle_edit_field(true),
        KeyCode::Tab => app.edit_input_tab(),
        KeyCode::BackTab => app.cycle_edit_field(true),

        // --- 4. Shortcuts (Navigation Mode Only) ---
        KeyCode::Char('o') if !is_field_active && focused == Some(EditField::Description) => {
            app.open_external_editor();
        }

        // --- 5. Insert Mode Actions (Typing & Cursor) ---

        // Up/Down: Move Cursor if in Description, otherwise ignore (or cycle if we wanted)
        KeyCode::Up if is_field_active => {
            if focused == Some(EditField::Description) {
                app.move_cursor_up();
            }
        }
        KeyCode::Down if is_field_active => {
            if focused == Some(EditField::Description) {
                app.move_cursor_down();
            }
        }

        // Left/Right/Home/End: Standard Cursor Movement
        KeyCode::Left if is_field_active => app.move_cursor_left(),
        KeyCode::Right if is_field_active => app.move_cursor_right(),
        KeyCode::Home if is_field_active => app.move_cursor_home(),
        KeyCode::End if is_field_active => app.move_cursor_end(),

        // Deletion
        KeyCode::Backspace if is_field_active => app.edit_input_backspace(),

        // Typing Characters
        KeyCode::Char(c) if is_field_active => {
            app.edit_input_char(c);
        }

        _ => {}
    }
}

pub fn handle_column_project_rename(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => match app.mode {
            InputMode::ProjectRename => app.confirm_rename_project(),
            InputMode::ColumnRename => app.confirm_rename_column(),
            InputMode::ColumnNew => app.confirm_new_column(),
            _ => {}
        },
        KeyCode::Esc => app.cancel_prompt(),
        KeyCode::Backspace => app.prompt_input_backspace(),
        KeyCode::Char(c) => app.prompt_input_char(c),
        _ => {}
    }
}

pub fn handle_column_delete(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.confirm_delete_column(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_prompt(),
        _ => {}
    }
}
