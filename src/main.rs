use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use directories::ProjectDirs;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::stdout;

mod app;
mod events;
mod io;
mod types;
mod ui; // Ensure io module is linked

use app::App;

fn main() -> Result<()> {
    // 1. Setup Terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // FIX: Explicitly clear the terminal buffer to remove previous shell output
    terminal.clear()?;

    // 2. Determine data path
    let proj_dirs =
        ProjectDirs::from("com", "rust", "kanban-tui").expect("Could not determine config dir");
    let data_dir = proj_dirs.data_dir();
    std::fs::create_dir_all(data_dir)?;
    let data_path = data_dir.join("kanban.yaml");

    // 3. Initialize App
    let mut app = App::new(data_path)?;

    // 4. Main Loop
    loop {
        terminal.draw(|f| {
            ui::draw(f, &mut app);
        })?;

        events::handle_events(&mut app)?;

        if app.should_quit {
            break;
        }
    }

    // 5. Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
