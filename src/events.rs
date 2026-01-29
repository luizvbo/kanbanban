use crate::app::{App, EditField, InputMode};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

pub fn handle_events(app: &mut App) -> Result<()> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
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
                        _ => {}
                    },
                    InputMode::Help => match key.code {
                        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.toggle_help(),
                        _ => {}
                    },
                    InputMode::TagSelection => match key.code {
                        // FIX: Esc now Saves and Closes (Vim-style)
                        KeyCode::Esc => app.tag_selector_confirm(),

                        // Enter toggles selection or creates new tag
                        KeyCode::Enter => app.tag_selector_toggle_or_create(),

                        // Tab can also confirm, or just do nothing.
                        // Let's keep it as confirm for flexibility.
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
                    // NEW: Delete Confirmation Handler
                    InputMode::DeleteConfirmation => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                            app.confirm_delete()
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            app.cancel_delete()
                        }
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Esc => {
                            // FIX: Smart Exit Logic
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
                                    s.focused_field == EditField::Description
                                        && s.description_edit_mode
                                })
                                .unwrap_or(false);

                            if is_desc_editing {
                                app.toggle_description_edit();
                            } else if is_new && is_empty {
                                // UX FIX: If it's new and empty, just discard it immediately.
                                app.cancel_edit();
                            } else {
                                // Otherwise, ask for confirmation
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
                                // UX FIX: Prevent saving empty titles
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
                                    s.focused_field == EditField::Description
                                        && s.description_edit_mode
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
                                    s.focused_field == EditField::Description
                                        && s.description_edit_mode
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
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            app.cancel_prompt()
                        }
                        _ => {}
                    },
                }
            }
        }
    }
    Ok(())
}
