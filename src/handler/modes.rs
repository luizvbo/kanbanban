use crossterm::event::{KeyCode, KeyEvent};

use crate::app::state::EditField;
use crate::app::{App, InputMode};

pub fn handle_normal(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('h') => app.prev_column(),
        KeyCode::Char('l') => app.next_column(),
        KeyCode::Char('j') => app.next_card(),
        KeyCode::Char('k') => app.prev_card(),
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
        KeyCode::Char('j') | KeyCode::Down => app.view_scroll_down(),
        KeyCode::Char('k') | KeyCode::Up => app.view_scroll_up(),
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

pub fn handle_tag_selection(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.tag_selector_confirm(),
        KeyCode::Enter => app.tag_selector_toggle_or_create(),
        KeyCode::Tab => app.tag_selector_confirm(),
        KeyCode::Down => app.tag_selector_next(),
        KeyCode::Up => app.tag_selector_prev(),
        KeyCode::Backspace => app.tag_selector_backspace(),
        KeyCode::Char(c) => app.tag_selector_input_char(c),
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

pub fn handle_editing(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            let is_desc_editing = app
                .edit_state
                .as_ref()
                .map(|s| s.focused_field == EditField::Description && s.description_edit_mode)
                .unwrap_or(false);

            if is_desc_editing {
                // Just leave text editing mode, stay in the modal
                app.toggle_description_edit();
            } else {
                // Standard exit logic
                if !app.is_edit_empty() {
                    app.trigger_exit_modal();
                } else {
                    app.cancel_edit();
                }
            }
        }
        KeyCode::Enter => {
            let focused_field = app.get_focused_field();
            match focused_field {
                Some(EditField::Description) => {
                    let is_editing = app
                        .edit_state
                        .as_ref()
                        .map(|s| s.description_edit_mode)
                        .unwrap_or(false);
                    if is_editing {
                        app.edit_input_char('\n');
                    } else {
                        app.toggle_description_edit();
                    }
                }
                Some(EditField::Tags) => app.open_tag_selector(),
                Some(EditField::Category) => app.open_category_selector(),
                _ => {
                    app.cycle_edit_field(false);
                }
            }
        }
        // FIX FOR ISSUE #4: 'o' only opens editor if Description is focused
        KeyCode::Char('o') if key.modifiers == crossterm::event::KeyModifiers::NONE => {
            if let Some(EditField::Description) = app.get_focused_field() {
                app.open_external_editor();
            } else {
                app.edit_input_char('o');
            }
        }
        KeyCode::Tab => app.edit_input_tab(),
        KeyCode::BackTab => app.cycle_edit_field(true),
        KeyCode::Up => {
            let is_desc_editing = app
                .edit_state
                .as_ref()
                .map(|s| s.focused_field == EditField::Description && s.description_edit_mode)
                .unwrap_or(false);
            if is_desc_editing {
                app.move_cursor_up();
            } else {
                app.cycle_edit_field(true);
            }
        }
        KeyCode::Down => {
            let is_desc_editing = app
                .edit_state
                .as_ref()
                .map(|s| s.focused_field == EditField::Description && s.description_edit_mode)
                .unwrap_or(false);
            if is_desc_editing {
                app.move_cursor_down();
            } else {
                app.cycle_edit_field(false);
            }
        }
        KeyCode::Left => app.move_cursor_left(),
        KeyCode::Right => app.move_cursor_right(),
        KeyCode::Home => app.move_cursor_home(),
        KeyCode::End => app.move_cursor_end(),
        KeyCode::Backspace => app.edit_input_backspace(),
        KeyCode::Char(c) => app.edit_input_char(c),
        _ => {}
    }
}

pub fn handle_category_selection(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Tab => app.category_selector_confirm(),
        KeyCode::Down => app.category_selector_next(),
        KeyCode::Up => app.category_selector_prev(),
        KeyCode::Backspace => app.category_selector_backspace(),
        KeyCode::Char(c) => app.category_selector_input_char(c),
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
