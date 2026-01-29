use crate::app::{App, EditField, InputMode};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

pub fn handle_events(app: &mut App) -> Result<()> {
    if event::poll(std::time::Duration::from_millis(50))? {
        let event = event::read()?;
        process_event(app, event)?;
    }
    Ok(())
}

pub fn process_event(app: &mut App, event: Event) -> Result<()> {
    if let Event::Key(key) = event {
        if key.kind == KeyEventKind::Press {
            match app.mode {
                InputMode::Normal => match key.code {
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
                    KeyCode::Char('i') | KeyCode::Char('e') | KeyCode::Enter => {
                        app.start_edit_card()
                    }
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
                },
                InputMode::Filter => match key.code {
                    KeyCode::Enter => app.stop_filter(),
                    KeyCode::Esc => app.clear_filter(),
                    KeyCode::Backspace => app.filter_input_backspace(),
                    KeyCode::Char(c) => app.filter_input_char(c),
                    _ => {}
                },
                InputMode::ViewCard => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('v') => {
                        app.close_detail_view()
                    }
                    KeyCode::Char('j') | KeyCode::Down => app.view_scroll_down(),
                    KeyCode::Char('k') | KeyCode::Up => app.view_scroll_up(),
                    KeyCode::Char('o') => app.open_external_editor(),
                    _ => {}
                },
                InputMode::Help => match key.code {
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.toggle_help(),
                    _ => {}
                },
                InputMode::TagSelection => match key.code {
                    KeyCode::Esc => app.tag_selector_confirm(),
                    KeyCode::Enter => app.tag_selector_toggle_or_create(),
                    KeyCode::Tab => app.tag_selector_confirm(),
                    KeyCode::Down => app.tag_selector_next(),
                    KeyCode::Up => app.tag_selector_prev(),
                    KeyCode::Backspace => app.tag_selector_backspace(),
                    KeyCode::Char(c) => app.tag_selector_input_char(c),
                    _ => {}
                },
                InputMode::ExitingModal => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        app.confirm_exit_save()
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => app.confirm_exit_discard(),
                    KeyCode::Esc => app.cancel_exit_modal(),
                    _ => {}
                },
                InputMode::DeleteConfirmation => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        app.confirm_delete()
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_delete(),
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    KeyCode::Esc => {
                        let is_new = app
                            .edit_state
                            .as_ref()
                            .map(|s| s.is_new_card)
                            .unwrap_or(false);
                        let is_empty = app.is_edit_empty();

                        let is_desc_editing = app
                            .edit_state
                            .as_ref()
                            .map(|s| {
                                s.focused_field == EditField::Description && s.description_edit_mode
                            })
                            .unwrap_or(false);

                        if is_desc_editing {
                            app.toggle_description_edit();
                        } else if is_new && is_empty {
                            app.cancel_edit();
                        } else {
                            app.trigger_exit_modal();
                        }
                    }
                    KeyCode::Enter => {
                        let focused_field = app.get_focused_field();
                        if let Some(EditField::Description) = focused_field {
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
                        } else if let Some(EditField::Tags) = focused_field {
                            app.open_tag_selector();
                        } else {
                            if !app.is_edit_empty() {
                                app.save_edit();
                            }
                        }
                    }
                    KeyCode::Tab => app.edit_input_tab(),
                    KeyCode::BackTab => app.cycle_edit_field(true),
                    KeyCode::Up => {
                        let is_desc_editing = app
                            .edit_state
                            .as_ref()
                            .map(|s| {
                                s.focused_field == EditField::Description && s.description_edit_mode
                            })
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
                            .map(|s| {
                                s.focused_field == EditField::Description && s.description_edit_mode
                            })
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
                },
                InputMode::ProjectRename | InputMode::ColumnRename | InputMode::ColumnNew => {
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
                InputMode::ColumnDelete => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        app.confirm_delete_column()
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_prompt(),
                    _ => {}
                },
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};
    use std::path::PathBuf;

    fn create_app() -> App {
        // Use a dummy path that doesn't exist to avoid loading real data
        let mut app = App::new(PathBuf::from("non_existent_test_file.yaml")).unwrap();
        // Ensure we have data to work with
        app.data.projects[0].columns[0]
            .cards
            .push(crate::types::Card {
                title: "Test".into(),
                description: "".into(),
                category: None,
                tags: vec![],
                due_date: None,
            });
        app
    }

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn test_all_normal_mode_keys() {
        let mut app = create_app();

        // Navigation
        process_event(&mut app, key(KeyCode::Char('l'))).unwrap(); // Next col
        process_event(&mut app, key(KeyCode::Char('h'))).unwrap(); // Prev col
        process_event(&mut app, key(KeyCode::Char('j'))).unwrap(); // Next card
        process_event(&mut app, key(KeyCode::Char('k'))).unwrap(); // Prev card

        // Movements
        process_event(&mut app, key(KeyCode::Char('J'))).unwrap(); // Move down
        process_event(&mut app, key(KeyCode::Char('K'))).unwrap(); // Move up
        process_event(&mut app, key(KeyCode::Char('H'))).unwrap(); // Move left
        process_event(&mut app, key(KeyCode::Char('L'))).unwrap(); // Move right

        // Column Ops
        process_event(&mut app, key(KeyCode::Char('>'))).unwrap(); // Move col right
        process_event(&mut app, key(KeyCode::Char('<'))).unwrap(); // Move col left

        // Modals triggers
        process_event(&mut app, key(KeyCode::Char('?'))).unwrap(); // Help
        assert_eq!(app.mode, InputMode::Help);
        process_event(&mut app, key(KeyCode::Esc)).unwrap(); // Close Help

        process_event(&mut app, key(KeyCode::Char('/'))).unwrap(); // Filter
        assert_eq!(app.mode, InputMode::Filter);
        process_event(&mut app, key(KeyCode::Esc)).unwrap(); // Close Filter

        process_event(&mut app, key(KeyCode::Char('v'))).unwrap(); // View
        assert_eq!(app.mode, InputMode::ViewCard);
        process_event(&mut app, key(KeyCode::Esc)).unwrap(); // Close View

        process_event(&mut app, key(KeyCode::Char('R'))).unwrap(); // Rename Project
        assert_eq!(app.mode, InputMode::ProjectRename);
        process_event(&mut app, key(KeyCode::Esc)).unwrap();

        process_event(&mut app, key(KeyCode::Char('r'))).unwrap(); // Rename Col
        assert_eq!(app.mode, InputMode::ColumnRename);
        process_event(&mut app, key(KeyCode::Esc)).unwrap();

        process_event(&mut app, key(KeyCode::Char('c'))).unwrap(); // New Col
        assert_eq!(app.mode, InputMode::ColumnNew);
        process_event(&mut app, key(KeyCode::Esc)).unwrap();

        process_event(&mut app, key(KeyCode::Char('D'))).unwrap(); // Delete Col
        assert_eq!(app.mode, InputMode::ColumnDelete);
        process_event(&mut app, key(KeyCode::Esc)).unwrap();
    }

    #[test]
    fn test_filter_mode_interaction() {
        let mut app = create_app();
        app.mode = InputMode::Filter;

        process_event(&mut app, key(KeyCode::Char('a'))).unwrap();
        assert_eq!(app.filter_query, "a");

        process_event(&mut app, key(KeyCode::Backspace)).unwrap();
        assert_eq!(app.filter_query, "");

        process_event(&mut app, key(KeyCode::Enter)).unwrap();
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn test_tag_selection_interaction() {
        let mut app = create_app();
        app.start_new_card();
        app.open_tag_selector(); // Sets mode to TagSelection

        // Navigation
        process_event(&mut app, key(KeyCode::Down)).unwrap();
        process_event(&mut app, key(KeyCode::Up)).unwrap();

        // Input
        process_event(&mut app, key(KeyCode::Char('b'))).unwrap();
        assert_eq!(app.tag_selector_state.as_ref().unwrap().search_query, "b");

        process_event(&mut app, key(KeyCode::Backspace)).unwrap();
        assert!(
            app.tag_selector_state
                .as_ref()
                .unwrap()
                .search_query
                .is_empty()
        );

        // Toggle/Confirm
        process_event(&mut app, key(KeyCode::Enter)).unwrap(); // Toggle
        process_event(&mut app, key(KeyCode::Esc)).unwrap(); // Confirm & Close

        assert_eq!(app.mode, InputMode::Editing);
    }

    #[test]
    fn test_editing_mode_interaction() {
        let mut app = create_app();
        app.start_new_card();

        // Typing
        process_event(&mut app, key(KeyCode::Char('A'))).unwrap();
        assert_eq!(app.edit_state.as_ref().unwrap().title, "A");

        // Navigation
        process_event(&mut app, key(KeyCode::Tab)).unwrap(); // Cycle
        process_event(&mut app, key(KeyCode::BackTab)).unwrap(); // Cycle back

        // Exit Logic
        process_event(&mut app, key(KeyCode::Esc)).unwrap();
        // Since it's new and not empty (has 'A'), it should ask for confirmation
        assert_eq!(app.mode, InputMode::ExitingModal);

        // Confirm Discard
        process_event(&mut app, key(KeyCode::Char('n'))).unwrap();
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn test_column_delete_interaction() {
        let mut app = create_app();
        app.mode = InputMode::ColumnDelete;

        // Cancel
        process_event(&mut app, key(KeyCode::Char('n'))).unwrap();
        assert_eq!(app.mode, InputMode::Normal);

        // Confirm
        app.mode = InputMode::ColumnDelete;
        process_event(&mut app, key(KeyCode::Char('y'))).unwrap();
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn test_view_mode_interaction() {
        let mut app = create_app();
        app.mode = InputMode::ViewCard;

        process_event(&mut app, key(KeyCode::Char('j'))).unwrap(); // Scroll down
        assert_eq!(app.view_scroll_y, 1);

        process_event(&mut app, key(KeyCode::Char('k'))).unwrap(); // Scroll up
        assert_eq!(app.view_scroll_y, 0);

        process_event(&mut app, key(KeyCode::Char('q'))).unwrap(); // Quit view
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn test_prompt_input_handling() {
        let mut app = create_app();

        // Enter Rename Mode
        process_event(&mut app, key(KeyCode::Char('R'))).unwrap();
        assert_eq!(app.mode, InputMode::ProjectRename);

        // It pre-fills "Default Board"
        let initial_len = app.prompt_input.len();

        // Test Backspace
        process_event(&mut app, key(KeyCode::Backspace)).unwrap();
        assert_eq!(app.prompt_input.len(), initial_len - 1);

        // Test Typing
        process_event(&mut app, key(KeyCode::Char('X'))).unwrap();
        assert!(app.prompt_input.ends_with('X'));

        // Test Esc (Cancel)
        process_event(&mut app, key(KeyCode::Esc)).unwrap();
        assert_eq!(app.mode, InputMode::Normal);
        // Name should NOT have changed
        assert_eq!(app.data.projects[0].name, "Default Board");
    }
}
