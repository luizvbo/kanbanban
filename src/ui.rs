use crate::app::{App, InputMode};
use crate::types::Card;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &mut App) {
    // FIX: Clear the entire frame area first to prevent artifacts
    f.render_widget(Clear, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    let project_name = &app.current_project().name;
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Kanban TUI ");

    let title_text = Paragraph::new(format!("Project: {}", project_name))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(title_block);

    f.render_widget(title_text, chunks[0]);

    draw_board(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    if let InputMode::Help = app.mode {
        draw_help_popup(f);
    }
}

fn draw_board(f: &mut Frame, app: &App, area: Rect) {
    let columns = &app.current_project().columns;

    let constraints: Vec<Constraint> = (0..columns.len())
        .map(|_| Constraint::Percentage(100 / columns.len() as u16))
        .collect();

    let col_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (i, col) in columns.iter().enumerate() {
        let is_col_focused = app.current_col_idx == i;

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(if is_col_focused {
                BorderType::Thick
            } else {
                BorderType::Plain
            })
            .border_style(if is_col_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title(format!(" {} ({}) ", col.title, col.cards.len()));

        f.render_widget(block.clone(), col_chunks[i]);

        let inner_area = block.inner(col_chunks[i]);
        let mut y_offset = inner_area.y;

        for (j, card) in col.cards.iter().enumerate() {
            let card_height = 4 + card.tags.len() as u16 + 2;

            if y_offset + card_height > inner_area.bottom() {
                break;
            }

            let card_area = Rect {
                x: inner_area.x,
                y: y_offset,
                width: inner_area.width,
                height: card_height,
            };

            let is_selected = is_col_focused && app.current_card_idx == j;
            draw_card(f, card, card_area, is_selected);

            y_offset += card_height;
        }
    }
}

fn draw_card(f: &mut Frame, card: &Card, area: Rect, is_selected: bool) {
    let border_style = if is_selected {
        Style::default()
            .fg(Color::LightBlue)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(Span::styled(
            &card.title,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    let tag_spans: Vec<Span> = card
        .tags
        .iter()
        .map(|t| {
            Span::styled(
                format!(" #{} ", t.name),
                Style::default().bg(Color::from(&t.color)).fg(Color::Black),
            )
        })
        .collect();

    let mut spaced_tags = Vec::new();
    for tag in tag_spans {
        spaced_tags.push(tag);
        spaced_tags.push(Span::raw(" "));
    }

    f.render_widget(Line::from(spaced_tags), chunks[1]);

    let markdown_text = parse_markdown(&card.description);
    f.render_widget(
        Paragraph::new(markdown_text).wrap(Wrap { trim: true }),
        chunks[2],
    );
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = match app.mode {
        InputMode::Normal => " NORMAL ",
        InputMode::Editing => " EDITING ",
        InputMode::Help => " HELP ",
    };

    let mode_color = match app.mode {
        InputMode::Normal => Color::Blue,
        InputMode::Editing => Color::Yellow,
        InputMode::Help => Color::Green,
    };

    let keys = " [q] Quit | [?] Help | [h/j/k/l] Navigate | [d] Delete | [i] Edit ";

    let text = Line::from(vec![
        Span::styled(
            mode_str,
            Style::default()
                .bg(mode_color)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(keys),
    ]);

    f.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::TOP)),
        area,
    );
}

fn draw_help_popup(f: &mut Frame) {
    let area = centered_rect(60, 50, f.area());

    let help_text = vec![
        Line::from(Span::styled(
            "Kanban TUI Help",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("h / l", Style::default().fg(Color::Cyan)),
            Span::raw(": Move between columns"),
        ]),
        Line::from(vec![
            Span::styled("j / k", Style::default().fg(Color::Cyan)),
            Span::raw(": Move between cards"),
        ]),
        Line::from(vec![
            Span::styled("d", Style::default().fg(Color::Red)),
            Span::raw(": Delete selected card"),
        ]),
        Line::from(vec![
            Span::styled("i", Style::default().fg(Color::Green)),
            Span::raw(": Edit card (Mock)"),
        ]),
        Line::from(vec![
            Span::styled("?", Style::default().fg(Color::Magenta)),
            Span::raw(": Toggle this popup"),
        ]),
        Line::from(vec![
            Span::styled("q", Style::default().fg(Color::Red)),
            Span::raw(": Quit"),
        ]),
        Line::from(""),
        Line::from("Data is saved automatically to kanban.yaml"),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));

    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(help_text).block(block), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

// FIX: Added explicit lifetime <'_> to match the return type with input reference
fn parse_markdown(content: &str) -> Text<'_> {
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
                style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
            }
            Event::Start(Tag::Item) => {
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
