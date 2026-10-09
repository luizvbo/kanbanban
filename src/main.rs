use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::stdout;
use std::path::PathBuf;

// Import from the library crate
use kanbanban::Args;
use kanbanban::app::App;
use kanbanban::handler;
use kanbanban::ui;

/// Directory for rolling log files. Logs go to the XDG state dir (or
/// ~/.local/state) instead of `./logs`, so running `kbb` doesn't litter
/// the current working directory.
fn log_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_STATE_HOME") {
        return PathBuf::from(dir).join("kanbanban");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/state/kanbanban");
    }
    std::env::temp_dir().join("kanbanban")
}

fn main() -> Result<()> {
    // --- Setup Logging ---
    let log_dir = log_dir();
    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender::rolling::daily(log_dir, "kanbanban.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    // Hook panics to log them
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("Panic occurred: {:?}", info);
        // Restore terminal before crashing so user sees the error
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        default_panic(info);
    }));
    let args = Args::parse();

    // Load the data file *before* touching the terminal so a corrupt or
    // unreadable file exits cleanly with a normal error message.
    let data_path = args.path.unwrap_or_else(|| PathBuf::from("kbb.yaml"));
    let mut app = App::new(data_path)?;

    // Any error inside the TUI loop must still restore the terminal —
    // an early `?` would otherwise leave the shell in raw mode.
    let res = run_tui(&mut app);

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    res
}

fn run_tui(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let res = event_loop(&mut terminal, app);

    terminal.show_cursor()?;
    res
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| {
            ui::draw(f, app);
        })?;

        handler::handle_events(app)?;

        if app.should_redraw {
            terminal.clear()?;
            app.should_redraw = false;
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
