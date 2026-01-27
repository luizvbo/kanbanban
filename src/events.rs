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
                    InputMode::Editing => match key.code {
                        KeyCode::Esc => app.trigger_exit_modal(),

                        KeyCode::Enter => {
                            let focused_field = app.get_focused_field();
                            if let Some(EditField::Description) = focused_field {
                                app.edit_input_char('\n');
                            } else {
                                app.save_edit();
                            }
                        }
                        KeyCode::BackTab => app.cycle_edit_field(true),
                        KeyCode::Tab => app.cycle_edit_field(false),

                        KeyCode::Backspace => app.edit_input_backspace(),
                        KeyCode::Char(c) => app.edit_input_char(c),
                        _ => {}
                    },
                    InputMode::ExitingModal => match key.code {
                        KeyCode::Char('y') | KeyCode::Enter => app.confirm_exit_save(),
                        KeyCode::Char('n') => app.confirm_exit_discard(),
                        KeyCode::Esc => app.cancel_exit_modal(),
                        _ => {}
                    },
                }
            }
        }
    }
    Ok(())
}
