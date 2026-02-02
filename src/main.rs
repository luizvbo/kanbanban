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

fn main() -> Result<()> {
    // --- Setup Logging ---
    let file_appender = tracing_appender::rolling::daily("logs", "kanbanban.log");
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

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let data_path = args.path.unwrap_or_else(|| PathBuf::from("kbb.yaml"));
    let mut app = App::new(data_path)?;

    loop {
        terminal.draw(|f| {
            ui::draw(f, &mut app);
        })?;

        handler::handle_events(&mut app)?;

        if app.should_redraw {
            terminal.clear()?;
            app.should_redraw = false;
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
