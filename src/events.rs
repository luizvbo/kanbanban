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
                        KeyCode::Char('d') => app.delete_current_card(),
                        KeyCode::Char('a') | KeyCode::Char('n') => app.start_new_card(),
                        KeyCode::Char('i') | KeyCode::Char('e') => app.start_edit_card(),
                        KeyCode::Char('?') => app.toggle_help(),
                        _ => {}
                    },
                    InputMode::Help => match key.code {
                        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.toggle_help(),
                        _ => {}
                    },
                    InputMode::TagSelection => match key.code {
                        KeyCode::Esc => app.close_tag_selector(),
                        KeyCode::Enter => app.tag_selector_confirm(),
                        KeyCode::Char(' ') => app.tag_selector_toggle(),
                        KeyCode::Char('j') | KeyCode::Down => app.tag_selector_next(),
                        KeyCode::Char('k') | KeyCode::Up => app.tag_selector_prev(),
                        _ => {}
                    },
                    InputMode::ExitingModal => match key.code {
                        // FIX: Added uppercase Y/N support
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                            app.confirm_exit_save()
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => app.confirm_exit_discard(),
                        KeyCode::Esc => app.cancel_exit_modal(),
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Esc => {
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
                                app.save_edit();
                            }
                        }

                        KeyCode::Tab => app.edit_input_tab(),
                        KeyCode::BackTab => app.cycle_edit_field(true),

                        KeyCode::Up => app.cycle_edit_field(true),
                        KeyCode::Down => app.cycle_edit_field(false),

                        KeyCode::Left => app.move_cursor_left(),
                        KeyCode::Right => app.move_cursor_right(),
                        KeyCode::Home => app.move_cursor_home(),
                        KeyCode::End => app.move_cursor_end(),

                        KeyCode::Backspace => app.edit_input_backspace(),
                        KeyCode::Char(c) => app.edit_input_char(c),
                        _ => {}
                    },
                }
            }
        }
    }
    Ok(())
}
