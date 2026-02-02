use crate::app::{App, InputMode};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

/// Centers a rect with a fixed height (in rows) and percentage width.
pub fn centered_fixed_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Length((r.height.saturating_sub(height)) / 2),
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

/// Iterates through CommonMark events and maps them to Ratatui Styles.
/// Supports: Bold, Italics, Lists, and Headings.
///
/// Note: This handles the logic for double line-breaks to ensure
/// paragraphs are visually separated.
pub fn parse_markdown(content: &str) -> Text<'_> {
    let mut lines = Vec::new();
    let parser = Parser::new(content);

    let mut current_line_spans = Vec::new();
    let mut style = Style::default();
    let mut in_code_block = false;

    for event in parser {
        match event {
            // Normal Text
            Event::Text(t) => {
                if in_code_block {
                    // Render code block content
                    current_line_spans.push(Span::styled(
                        t.to_string(),
                        Style::default().fg(Color::LightCyan),
                    ));
                } else {
                    current_line_spans.push(Span::styled(t.to_string(), style));
                }
            }

            // Inline Code (e.g., `variable`)
            Event::Code(t) => {
                current_line_spans.push(Span::styled(
                    t.to_string(),
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            // Code Blocks (```rust ...)
            Event::Start(Tag::CodeBlock(_)) => {
                // Flush current line before starting block
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                // Flush the last line of the code block
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
                in_code_block = false;
                lines.push(Line::from("")); // Add spacing after block
            }

            // Styling
            Event::Start(Tag::Emphasis) => style = style.add_modifier(Modifier::ITALIC),
            Event::Start(Tag::Strong) => style = style.add_modifier(Modifier::BOLD),
            Event::Start(Tag::Heading { .. }) => {
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
                style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
            }

            // Lists
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

            // Reset Styles on End
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
            Event::End(TagEnd::Paragraph) => {
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
                // Only add spacing if we aren't inside a code block (which handles its own newlines usually)
                if !in_code_block {
                    lines.push(Line::from(""));
                }
            }

            // Line Breaks
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

    let mode_style = Style::default()
        .bg(Color::Blue)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);

    // 1. Determine Middle Content (Status or Filter)
    let (content, style) = if app.mode == InputMode::Filter || !app.filter_query.is_empty() {
        (
            format!(" FILTER: {} ", app.filter_query),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else if let Some(msg) = &app.status_message {
        if let Some(time) = app.status_time {
            if time.elapsed().as_secs() < 3 {
                (format!(" {} ", msg), Style::default().fg(Color::White))
            } else {
                (String::new(), Style::default())
            }
        } else {
            (String::new(), Style::default())
        }
    } else {
        (String::new(), Style::default())
    };

    // 2. Determine Right-Side Help Text
    let help_text = match app.mode {
        InputMode::Editing => " [Esc] Back/Save | [Tab] Cycle | [o] Editor | [Enter] Select/Edit ",
        InputMode::Normal => " [q] Quit | [?] Help | [v] View | [a] Add | [h/j/k/l] Nav ",
        InputMode::ViewCard => " [Esc/q] Close | [j/k] Scroll | [o] Editor ",
        InputMode::Filter => " [Esc] Clear/Exit | [Enter] Apply ",
        _ => " [Esc] Return ",
    };

    // Layout: [MODE] [Status/Filter ......] [Help]
    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(mode_str.len() as u16),
            Constraint::Min(1), // Middle takes remaining space
            Constraint::Length(help_text.len() as u16),
        ])
        .split(area);

    // Render Mode
    f.render_widget(Paragraph::new(mode_str).style(mode_style), footer_chunks[0]);

    // Render Content (Filter/Status)
    f.render_widget(Paragraph::new(content).style(style), footer_chunks[1]);

    // Render Help
    f.render_widget(
        Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Right),
        footer_chunks[2],
    );
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
