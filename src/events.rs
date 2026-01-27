use crate::app::{App, InputMode};
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
                        KeyCode::Char('d') => app.delete_current_card(),
                        KeyCode::Char('i') => app.toggle_edit(),
                        KeyCode::Char('?') => app.toggle_help(),
                        _ => {}
                    },
                    InputMode::Help => match key.code {
                        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.toggle_help(),
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Esc => app.toggle_edit(),
                        // In a real app, you would handle text input here
                        _ => {}
                    },
                }
            }
        }
    }
    Ok(())
}
