use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::stdout;
use std::path::PathBuf;

mod app;
mod events;
mod types;
mod ui;

use app::App;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the kanban board file (defaults to ./kbb.yaml)
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,
}

fn main() -> Result<()> {
    // 1. Parse CLI Args
    let args = Args::parse();

    // 2. Setup Terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    // 3. Determine data path
    // Default to "kbb.yaml" in the current directory if not provided
    let data_path = args.path.unwrap_or_else(|| PathBuf::from("kbb.yaml"));

    // 4. Initialize App
    let mut app = App::new(data_path)?;

    // 5. Main Loop
    loop {
        terminal.draw(|f| {
            ui::draw(f, &mut app);
        })?;

        events::handle_events(&mut app)?;

        if app.should_quit {
            break;
        }
    }

    // 6. Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
