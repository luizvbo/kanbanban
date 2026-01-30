use crate::app::{App, InputMode};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

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

pub fn parse_markdown(content: &str) -> Text<'_> {
    let mut lines = Vec::new();
    let parser = Parser::new(content);

    let mut current_line_spans = Vec::new();
    let mut style = Style::default();

    for event in parser {
        match event {
            Event::Text(t) => current_line_spans.push(Span::styled(t.to_string(), style)),

            Event::Start(Tag::Emphasis) => style = style.add_modifier(Modifier::ITALIC),
            Event::Start(Tag::Strong) => style = style.add_modifier(Modifier::BOLD),
            Event::Start(Tag::Heading { .. }) => {
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
                style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
            }
            Event::Start(Tag::List(_)) => {
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
            }
            Event::Start(Tag::Item) => {
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
                current_line_spans.push(Span::raw("• "));
            }

            Event::End(TagEnd::Emphasis) => style = style.remove_modifier(Modifier::ITALIC),
            Event::End(TagEnd::Strong) => style = style.remove_modifier(Modifier::BOLD),
            Event::End(TagEnd::Heading(_)) => {
                style = Style::default();
                lines.push(Line::from(current_line_spans.clone()));
                current_line_spans.clear();
                lines.push(Line::from(""));
            }
            Event::End(TagEnd::Item) => {
                lines.push(Line::from(current_line_spans.clone()));
                current_line_spans.clear();
            }
            Event::End(TagEnd::List(_)) => {
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
            }
            // FIX: Handle Paragraph End to render double line breaks correctly
            Event::End(TagEnd::Paragraph) => {
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
                lines.push(Line::from("")); // Add spacing between paragraphs
            }

            Event::SoftBreak | Event::HardBreak => {
                lines.push(Line::from(current_line_spans.clone()));
                current_line_spans.clear();
            }

            _ => {}
        }
    }

    if !current_line_spans.is_empty() {
        lines.push(Line::from(current_line_spans));
    }

    Text::from(lines)
}

pub fn create_input_block<'a>(title: &'a str, content: &'a str, is_focused: bool) -> Paragraph<'a> {
    Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(if is_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            }),
    )
}

pub fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = format!(
        " {} ",
        match app.mode {
            InputMode::Normal => "NORMAL",
            InputMode::Editing => "EDIT",
            InputMode::ViewCard => "VIEW",
            InputMode::Filter => "FILTER",
            _ => "BUSY",
        }
    );

    let help_text = match app.mode {
        InputMode::Editing => " [Esc] Back/Save | [Tab] Cycle | [o] Editor | [Enter] Select/Edit ",
        InputMode::Normal => {
            " [q] Quit | [?] Help | [v] View | [a] Add | [h/j/k/l] Nav | [H/J/K/L] Move "
        }
        InputMode::ViewCard => " [Esc/q] Close | [j/k] Scroll | [o] Editor ",
        _ => " [Esc] Return to Normal ",
    };
    let text = Line::from(vec![
        Span::styled(
            mode_str,
            Style::default()
                .bg(Color::Blue)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(help_text, Style::default().fg(Color::Gray)),
    ]);
    f.render_widget(Paragraph::new(text), area);
}

pub fn draw_filter_bar(f: &mut Frame, app: &App) {
    let area = f.area();
    let filter_area = Rect {
        x: area.x,
        y: area.height - 4,
        width: area.width,
        height: 1,
    };

    let prefix = if app.mode == InputMode::Filter {
        "FILTER: "
    } else {
        "FILTER (Active): "
    };
    let style = if app.mode == InputMode::Filter {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let text = Line::from(vec![
        Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
        Span::raw(&app.filter_query),
    ]);

    f.render_widget(ratatui::widgets::Clear, filter_area);
    f.render_widget(
        Paragraph::new(text).style(Style::default().bg(Color::Black)),
        filter_area,
    );
}
