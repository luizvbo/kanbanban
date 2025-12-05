pub mod board;
pub mod categories;
pub mod datepicker;
pub mod markdown;
pub mod modals;
pub mod overview;
pub mod settings;

use crate::app::{App, ViewMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Tabs},
};

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);

    match app.view_mode {
        ViewMode::Board => board::render(f, app, chunks[1]),
        ViewMode::Overview => overview::render(f, app, chunks[1]),
        ViewMode::Settings => settings::render(f, app, chunks[1]),
        ViewMode::BoardSelector => board::render_selector(f, app, chunks[1]),
        ViewMode::CategoryManager => categories::render(f, app, chunks[1]),
    }

    render_footer(f, app, chunks[2]);

    if let Some(modal) = &app.active_modal {
        modals::render(f, app, modal);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let board_name = app
        .boards
        .get(app.active_board_index)
        .map(|b| b.name.as_str())
        .unwrap_or("No Board");

    let titles = vec![
        "Board (b)",
        "Overview (v)",
        "Settings (s)",
        "Categories (C)",
    ];
    let index = match app.view_mode {
        ViewMode::Board | ViewMode::BoardSelector => 0,
        ViewMode::Overview => 1,
        ViewMode::Settings => 2,
        ViewMode::CategoryManager => 3,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Kanbanban - {} ", board_name)),
        )
        .select(index)
        .highlight_style(Style::default().fg(Color::Yellow));

    f.render_widget(tabs, area);
}

fn render_footer(f: &mut Frame, _app: &App, area: Rect) {
    let help = Paragraph::new("q: Quit | h/j/k/l: Nav | H/L: Move Task | n: New Task | e: Edit | c: New Col | s: Settings")
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, OverviewTab, ViewMode};
    use crate::db::Database;
    use chrono::{Duration, Local};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use tempfile::NamedTempFile;

    fn setup_app() -> App<'static> {
        let file = NamedTempFile::new().unwrap();
        let db = Database::new(file.path()).unwrap();
        App::new(db).unwrap()
    }

    #[test]
    fn test_ui_render_board() {
        let mut app = setup_app();
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        assert!(buffer.content.iter().any(|c| c.symbol() == "K")); // Kanbanban
        assert!(buffer.content.iter().any(|c| c.symbol() == "R")); // Ready
        assert!(buffer.content.iter().any(|c| c.symbol() == "D")); // Doing
    }

    #[test]
    fn test_ui_render_board_tasks() {
        let mut app = setup_app();
        let bid = app.boards[0].id;
        let cid = app.columns[0].id;

        // Overdue task
        let overdue = Local::now().naive_local() - Duration::days(2);
        app.db
            .create_task(
                bid,
                cid,
                "Overdue".into(),
                "".into(),
                Some(overdue.format("%Y-%m-%d %H:%M:%S").to_string()),
                None,
            )
            .unwrap();

        // Due today
        let today = Local::now().naive_local();
        app.db
            .create_task(
                bid,
                cid,
                "Today".into(),
                "".into(),
                Some(today.format("%Y-%m-%d %H:%M:%S").to_string()),
                None,
            )
            .unwrap();

        app.refresh_data().unwrap();

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();

        assert!(content.contains("Overdue"));
        assert!(content.contains("Today"));
        assert!(content.contains("Due Today"));
    }

    #[test]
    fn test_ui_render_settings() {
        let mut app = setup_app();
        app.view_mode = ViewMode::Settings;

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content_str: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("Column"));
        assert!(content_str.contains("Management"));
    }

    #[test]
    fn test_ui_render_modal_task() {
        let mut app = setup_app();
        app.open_new_task_modal();

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content_str: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("Task"));
        assert!(content_str.contains("Details"));
    }

    #[test]
    fn test_ui_render_modal_column() {
        let mut app = setup_app();
        app.open_new_column_modal();

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content_str: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("New Column"));
    }

    #[test]
    fn test_ui_render_modal_category() {
        let mut app = setup_app();
        app.open_new_category_modal();

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content_str: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("Category Details"));
    }

    #[test]
    fn test_ui_render_modal_board() {
        let mut app = setup_app();
        app.open_new_board_modal();

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content_str: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("New Board"));
    }

    #[test]
    fn test_ui_render_overview_logs() {
        let mut app = setup_app();
        app.view_mode = ViewMode::Overview;
        app.overview_state.tab = OverviewTab::Logs;

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content_str: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("Audit Logs"));
    }

    #[test]
    fn test_ui_render_overview_charts() {
        let mut app = setup_app();
        app.view_mode = ViewMode::Overview;
        app.overview_state.tab = OverviewTab::Charts;

        // Add data for chart
        app.db.create_category("C1".into(), "".into()).unwrap();
        let bid = app.boards[0].id;
        let cid = app.columns[0].id;
        let cat_id = app.db.data.categories[0].id;
        app.db
            .create_task(bid, cid, "T".into(), "".into(), None, Some(cat_id))
            .unwrap();
        app.refresh_data().unwrap();

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content_str: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("Tasks by Category"));
    }

    #[test]
    fn test_ui_render_category_manager() {
        let mut app = setup_app();
        app.view_mode = ViewMode::CategoryManager;
        app.db
            .create_category("Work".into(), "#FFF".into())
            .unwrap();
        app.refresh_data().unwrap();

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content_str: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("Category Manager"));
        assert!(content_str.contains("Work"));
    }

    #[test]
    fn test_ui_render_board_selector() {
        let mut app = setup_app();
        app.view_mode = ViewMode::BoardSelector;

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content_str: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("Select Board"));
    }
}
