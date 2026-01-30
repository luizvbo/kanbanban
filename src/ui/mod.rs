pub mod board;
pub mod modals;
pub mod widgets;

use crate::app::{App, InputMode};
use crate::ui::board::draw_board;
use crate::ui::modals::{
    draw_category_selector, draw_column_delete_modal, draw_delete_confirmation, draw_detail_view,
    draw_edit_popup, draw_exit_confirmation, draw_help_popup, draw_input_modal, draw_tag_selector,
};
use crate::ui::widgets::{draw_filter_bar, draw_footer};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::Clear,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    f.render_widget(Clear, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    let project_name = &app.current_project().name;
    let header = Line::from(vec![
        Span::styled(
            " KANBANBAN ",
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Project: "),
        Span::styled(
            project_name,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(header), chunks[0]);

    draw_board(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    if app.mode == InputMode::Filter || !app.filter_query.is_empty() {
        draw_filter_bar(f, app);
    }

    match app.mode {
        InputMode::Help => draw_help_popup(f, app),
        InputMode::Editing => draw_edit_popup(f, app),
        InputMode::TagSelection => {
            draw_edit_popup(f, app);
            draw_tag_selector(f, app);
        }
        InputMode::CategorySelection => {
            draw_edit_popup(f, app);
            draw_category_selector(f, app);
        }
        InputMode::ExitingModal => {
            draw_edit_popup(f, app);
            draw_exit_confirmation(f);
        }
        InputMode::DeleteConfirmation => draw_delete_confirmation(f),
        InputMode::ProjectRename => draw_input_modal(f, " Rename Board ", &app.prompt_input),
        InputMode::ColumnRename => draw_input_modal(f, " Rename Column ", &app.prompt_input),
        InputMode::ColumnNew => draw_input_modal(f, " New Column Title ", &app.prompt_input),
        InputMode::ColumnDelete => draw_column_delete_modal(f, app),
        InputMode::ViewCard => draw_detail_view(f, app),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};
    use std::path::PathBuf;

    use crate::{
        app::{App, InputMode},
        domain::kanban::Card,
        ui::draw,
    };

    fn create_app() -> App {
        let mut app = App::new(PathBuf::from("dummy")).unwrap();
        // Setup simple state
        app.data.projects[0].name = "Test Project".into();
        app.data.projects[0].columns[0].title = "Col A".into();
        app.data.projects[0].columns[0].cards.clear();

        app.data.projects[0].columns[0].cards.push(Card {
            title: "Card X".into(),
            description: "Desc".into(),
            category: Some("Cat".into()),
            tags: vec![],
            due_date: None,
        });
        app
    }

    #[test]
    fn test_draw_main_board() {
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = create_app();

        terminal.draw(|f| draw(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();

        // Assert Title
        assert!(buffer_contains(buffer, "Kanban TUI"));
        assert!(buffer_contains(buffer, "Project: Test Project"));

        // Assert Column
        assert!(buffer_contains(buffer, "Col A"));

        // Assert Card
        assert!(buffer_contains(buffer, "Card X"));
        assert!(buffer_contains(buffer, "Cat")); // Category
    }

    #[test]
    fn test_draw_help_popup() {
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = create_app();
        app.mode = InputMode::Help;

        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        assert!(buffer_contains(buffer, "Kanban TUI Help"));
        assert!(buffer_contains(buffer, "Navigation"));
    }

    #[test]
    fn test_draw_edit_modal() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = create_app();
        app.start_new_card(); // Enters Editing mode

        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        assert!(buffer_contains(buffer, "Edit"));
        assert!(buffer_contains(buffer, "Title"));
        assert!(buffer_contains(buffer, "Category"));
        assert!(buffer_contains(buffer, "Description"));
    }

    // Helper to search buffer for string
    fn buffer_contains(buffer: &ratatui::buffer::Buffer, s: &str) -> bool {
        for y in 0..buffer.area.height {
            let line_width = buffer.area.width;
            let mut line_string = String::new();
            for x in 0..line_width {
                line_string.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
            }
            if line_string.contains(s) {
                return true;
            }
        }
        false
    }

    #[test]
    fn test_draw_all_modals() {
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = create_app();

        // 1. Delete Confirmation
        app.mode = InputMode::DeleteConfirmation;
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        assert!(buffer_contains(
            terminal.backend().buffer(),
            "Delete Confirmation"
        ));

        // 2. Column Delete
        app.mode = InputMode::ColumnDelete;
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        assert!(buffer_contains(
            terminal.backend().buffer(),
            "DELETE COLUMN"
        ));

        // 3. Rename Project
        app.mode = InputMode::ProjectRename;
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        assert!(buffer_contains(terminal.backend().buffer(), "Rename Board"));

        // 4. Tag Selector
        app.start_new_card();
        app.open_tag_selector(); // Sets mode to TagSelection
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        assert!(buffer_contains(terminal.backend().buffer(), "Select Tags"));

        // 5. Detail View
        app.mode = InputMode::ViewCard;
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        // Should show the card title from create_app ("Card X")
        assert!(buffer_contains(terminal.backend().buffer(), "Card X"));
    }

    #[test]
    fn test_draw_filter_bar() {
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = create_app();

        app.filter_query = "urgent".into();
        terminal.draw(|f| draw(f, &mut app)).unwrap();

        assert!(buffer_contains(
            terminal.backend().buffer(),
            "FILTER (Active)"
        ));
        assert!(buffer_contains(terminal.backend().buffer(), "urgent"));
    }
}
