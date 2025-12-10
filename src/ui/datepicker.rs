use crate::app::App;
use chrono::{Datelike, NaiveDate};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(f: &mut Frame, app: &App) {
    if let Some(dp) = &app.date_picker {
        let area = crate::ui::centered_rect(40, 40, f.area());
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Select Date ")
            .style(Style::default().bg(Color::DarkGray));
        f.render_widget(block.clone(), area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Month/Year Header
                Constraint::Length(1), // Weekday Header
                Constraint::Min(0),    // Calendar Grid
                Constraint::Length(1), // Help
            ])
            .split(area);

        // 1. Header
        let header_text = dp.current_date.format("%B %Y").to_string();
        f.render_widget(
            Paragraph::new(header_text)
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(ratatui::layout::Alignment::Center),
            layout[0],
        );

        // 2. Weekdays
        let weekdays = "Mon Tue Wed Thu Fri Sat Sun";
        f.render_widget(
            Paragraph::new(weekdays)
                .style(Style::default().fg(Color::Gray))
                .alignment(ratatui::layout::Alignment::Center),
            layout[1],
        );

        // 3. Grid Calculation
        let first_day_of_month =
            NaiveDate::from_ymd_opt(dp.current_date.year(), dp.current_date.month(), 1).unwrap();
        // chrono weekday is Mon=0..Sun=6, we want that alignment
        let start_offset = first_day_of_month.weekday().num_days_from_monday();

        // Generate calendar text
        let mut calendar_lines = Vec::new();
        let mut current_line_spans = Vec::new();

        // Padding for first week
        for _ in 0..start_offset {
            current_line_spans.push(Span::raw("    "));
        }

        let days_in_month = if dp.current_date.month() == 12 {
            NaiveDate::from_ymd_opt(dp.current_date.year() + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(dp.current_date.year(), dp.current_date.month() + 1, 1).unwrap()
        }
        .signed_duration_since(first_day_of_month)
        .num_days();

        for day in 1..=days_in_month {
            let is_selected = (day as u32) == dp.current_date.day();
            let day_str = format!("{day:^3} ");

            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };

            current_line_spans.push(Span::styled(day_str, style));

            if (start_offset + day as u32).is_multiple_of(7) {
                calendar_lines.push(Line::from(current_line_spans.clone()));
                current_line_spans.clear();
            }
        }
        if !current_line_spans.is_empty() {
            calendar_lines.push(Line::from(current_line_spans));
        }

        let calendar_para =
            Paragraph::new(calendar_lines).alignment(ratatui::layout::Alignment::Center);
        f.render_widget(calendar_para, layout[2]);

        // 4. Help
        f.render_widget(
            Paragraph::new("Arrows: Move | Enter: Select | Esc: Cancel")
                .style(Style::default().fg(Color::Gray)),
            layout[3],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, DatePickerState};
    use crate::db::Database;
    use chrono::NaiveDate;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use tempfile::NamedTempFile;

    fn setup_app() -> App<'static> {
        let file = NamedTempFile::new().unwrap();
        let db = Database::new(file.path()).unwrap();
        App::new(db).unwrap()
    }

    #[test]
    fn test_render_datepicker() {
        let mut app = setup_app();

        // Set a fixed date: 2023-12-25 (Monday)
        let fixed_date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        app.date_picker = Some(DatePickerState {
            current_date: fixed_date,
        });

        // FIX: Increased size from 50x20 to 100x40 to accommodate the 28-char wide calendar grid
        // inside the 40% width popup.
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| render(f, &app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();

        // Check Header
        assert!(content.contains("Select Date"));
        assert!(content.contains("December 2023"));

        // Check Days
        // The format for day 1 is " 1  " (centered in 3 + 1 space)
        assert!(content.contains(" 1 "), "Content missing ' 1 ': {content}");
        assert!(content.contains("25"));
        assert!(content.contains("31"));

        // Check Footer
        assert!(content.contains("Arrows"));
    }
}
