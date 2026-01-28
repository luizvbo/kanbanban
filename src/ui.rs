use crate::app::{App, EditField, InputMode};
use crate::types::Card;
use chrono::{Local, NaiveDate};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &mut App) {
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

    match app.mode {
        InputMode::Help => draw_help_popup(f),
        InputMode::Editing => draw_edit_popup(f, app),
        InputMode::TagSelection => {
            draw_edit_popup(f, app);
            draw_tag_selector(f, app);
        }
        InputMode::ExitingModal => {
            draw_edit_popup(f, app);
            draw_exit_confirmation(f);
        }
        InputMode::DeleteConfirmation => draw_delete_confirmation(f),
        _ => {}
    }
}

fn draw_delete_confirmation(f: &mut Frame) {
    let area = centered_rect(40, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red))
        .title(" Delete Confirmation ");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Are you sure you want to delete this card?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y] Yes (Delete)", Style::default().fg(Color::Red)),
            Span::raw("   "),
            Span::styled("[n] No (Cancel)", Style::default().fg(Color::Green)),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
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
    // FIX: Improved Highlighting
    // Use Thick borders and Yellow color for high visibility
    let (border_style, border_type) = if is_selected {
        (
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            BorderType::Thick,
        )
    } else {
        (Style::default().fg(Color::DarkGray), BorderType::Rounded)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Dynamic Layout: Title -> Tags -> Date -> Description
    let mut constraints = vec![Constraint::Length(1)]; // 0: Title

    let has_tags = !card.tags.is_empty();
    if has_tags {
        constraints.push(Constraint::Length(1)); // 1: Tags (New Line)
    }

    let has_date = card.due_date.is_some();
    if has_date {
        constraints.push(Constraint::Length(1)); // 2: Date (New Line)
    }

    constraints.push(Constraint::Min(1)); // Last: Description

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut chunk_idx = 0;

    // 1. Title
    f.render_widget(
        Paragraph::new(Span::styled(
            &card.title,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        chunks[chunk_idx],
    );
    chunk_idx += 1;

    // 2. Tags (Rendered on their own line)
    if has_tags {
        let mut tag_spans = Vec::new();
        for tag in &card.tags {
            tag_spans.push(Span::styled(
                format!(" #{} ", tag.name),
                Style::default().bg(tag.get_color()).fg(Color::Black),
            ));
            tag_spans.push(Span::raw(" "));
        }
        f.render_widget(Line::from(tag_spans), chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // 3. Due Date
    if let Some(date_str) = &card.due_date {
        let date_display = if let Ok(parsed_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        {
            let today = Local::now().date_naive();
            let days_diff = (parsed_date - today).num_days();
            let color = if days_diff < 0 {
                Color::Red
            } else if days_diff <= 2 {
                Color::Yellow
            } else {
                Color::Green
            };
            let diff_text = if days_diff == 0 {
                "Today".to_string()
            } else if days_diff > 0 {
                format!("in {}d", days_diff)
            } else {
                format!("{}d ago", days_diff.abs())
            };
            Span::styled(
                format!("🕒 {} ({}) ", date_str, diff_text),
                Style::default().fg(color),
            )
        } else {
            Span::styled(
                format!("🕒 {} ", date_str),
                Style::default().fg(Color::DarkGray),
            )
        };
        f.render_widget(Paragraph::new(date_display), chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // 4. Description
    let markdown_text = parse_markdown(&card.description);
    f.render_widget(
        Paragraph::new(markdown_text).wrap(Wrap { trim: true }),
        chunks[chunk_idx],
    );
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = match app.mode {
        InputMode::Normal => " NORMAL ",
        InputMode::Editing => " EDITING ",
        InputMode::TagSelection => " TAGS ",
        InputMode::Help => " HELP ",
        InputMode::ExitingModal => " CONFIRM ",
        // FIX: Handle the new variant
        InputMode::DeleteConfirmation => " DELETE ",
    };
    let mode_color = match app.mode {
        InputMode::Normal => Color::Blue,
        InputMode::Editing => Color::Yellow,
        InputMode::TagSelection => Color::Magenta,
        InputMode::Help => Color::Green,
        InputMode::ExitingModal => Color::Red,
        // FIX: Handle the new variant
        InputMode::DeleteConfirmation => Color::Red,
    };
    let mut status_text =
        " [q] Quit | [?] Help | [h/j/k/l] Navigate | [J/K] Move Up/Down | [a] Add | [i] Edit "
            .to_string();
    if let Some(msg) = &app.status_message {
        if let Some(time) = app.status_time {
            if time.elapsed().as_secs() < 3 {
                status_text = format!(" {} ", msg);
            }
        }
    }
    let text = Line::from(vec![
        Span::styled(
            mode_str,
            Style::default()
                .bg(mode_color)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(status_text),
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
            Span::styled("H / L", Style::default().fg(Color::Cyan)),
            Span::raw(": Move card Left/Right"),
        ]),
        Line::from(vec![
            Span::styled("d", Style::default().fg(Color::Red)),
            Span::raw(": Delete selected card"),
        ]),
        Line::from(vec![
            Span::styled("a", Style::default().fg(Color::Green)),
            Span::raw(": Add new card"),
        ]),
        Line::from(vec![
            Span::styled("i", Style::default().fg(Color::Green)),
            Span::raw(": Edit card"),
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
                // Ensure heading starts on new line
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
                style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
            }
            // FIX: Ensure lists start on a new line
            Event::Start(Tag::List(_)) => {
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
            }
            // FIX: Ensure list items start on a new line
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
            // Handle explicit list end to ensure spacing if needed
            Event::End(TagEnd::List(_)) => {
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.clone()));
                    current_line_spans.clear();
                }
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

fn create_input_block<'a>(title: &'a str, content: &'a str, is_focused: bool) -> Paragraph<'a> {
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

fn draw_edit_popup(f: &mut Frame, app: &mut App) {
    if let Some(state) = &mut app.edit_state {
        let area = centered_rect(70, 80, f.area());
        f.render_widget(Clear, area);

        let block = Block::default()
            .title(if state.is_new_card {
                " New Card "
            } else {
                " Edit Card "
            })
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .style(Style::default().bg(Color::DarkGray));

        f.render_widget(block.clone(), area);

        let inner = block.inner(area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Tags
                Constraint::Length(3), // Due Date
                Constraint::Min(5),    // Description
                Constraint::Length(1), // Help
            ])
            .split(inner);

        // 1. Title
        let title_widget = create_input_block(
            "Title",
            &state.title,
            state.focused_field == EditField::Title,
        );
        let title_scroll = if state.focused_field == EditField::Title {
            if state.cursor_position as u16 >= chunks[0].width.saturating_sub(2) {
                state.cursor_position as u16 - (chunks[0].width.saturating_sub(3))
            } else {
                0
            }
        } else {
            0
        };
        f.render_widget(title_widget.scroll((0, title_scroll)), chunks[0]);

        // 2. Tags
        let tags_str = state
            .tags
            .iter()
            .map(|t| format!("#{}", t.name))
            .collect::<Vec<_>>()
            .join(" ");
        let tags_title = if state.focused_field == EditField::Tags {
            "Tags (Press Enter to Select)"
        } else {
            "Tags"
        };
        f.render_widget(
            create_input_block(
                tags_title,
                &tags_str,
                state.focused_field == EditField::Tags,
            ),
            chunks[1],
        );

        // 3. Due Date
        let date_widget = create_input_block(
            "Due Date (YYYY-MM-DD)",
            &state.due_date,
            state.focused_field == EditField::DueDate,
        );
        f.render_widget(date_widget, chunks[2]);

        // 4. Description
        let (cursor_col, cursor_row) =
            state.get_cursor_position_2d(chunks[3].width.saturating_sub(2));

        // Handle Vertical Scrolling
        let desc_height = chunks[3].height.saturating_sub(2);
        if cursor_row >= state.scroll_y + desc_height {
            state.scroll_y = cursor_row - desc_height + 1;
        } else if cursor_row < state.scroll_y {
            state.scroll_y = cursor_row;
        }

        // Handle Horizontal Scrolling
        let desc_width = chunks[3].width.saturating_sub(2);
        if cursor_col >= state.scroll_x + desc_width {
            state.scroll_x = cursor_col - desc_width + 1;
        } else if cursor_col < state.scroll_x {
            state.scroll_x = cursor_col;
        }

        // FIX: Use Text::from() instead of Span::raw() to correctly render newlines
        let desc_content = if state.description.is_empty() && !state.description_edit_mode {
            Text::from(Span::styled(
                "No description provided...",
                Style::default().fg(Color::Gray),
            ))
        } else {
            Text::from(state.description.as_str())
        };

        let (desc_border_style, desc_title) = if state.focused_field == EditField::Description {
            if state.description_edit_mode {
                (
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                    " Description (EDITING - Esc to Stop) ",
                )
            } else {
                (
                    Style::default().fg(Color::Yellow),
                    " Description (SELECTED - Enter to Edit) ",
                )
            }
        } else {
            (Style::default(), " Description (Markdown) ")
        };

        let desc_widget = Paragraph::new(desc_content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(desc_title)
                    .border_style(desc_border_style),
            )
            .scroll((state.scroll_y, state.scroll_x));

        f.render_widget(desc_widget, chunks[3]);

        // 5. Footer
        let footer_text =
            if state.focused_field == EditField::Description && state.description_edit_mode {
                "ESC: Stop Editing | TAB: Indent | ENTER: Newline"
            } else {
                "TAB/UP/DOWN: Navigate | ENTER: Edit/Save | ESC: Save/Discard"
            };
        f.render_widget(Paragraph::new(footer_text), chunks[4]);

        // Render Cursor
        if app.mode == InputMode::Editing {
            let show_cursor = if state.focused_field == EditField::Description {
                state.description_edit_mode
            } else {
                state.focused_field != EditField::Tags
            };

            if show_cursor {
                let (x, y) = match state.focused_field {
                    EditField::Title => (
                        chunks[0].x
                            + 1
                            + (state.cursor_position as u16).saturating_sub(title_scroll),
                        chunks[0].y + 1,
                    ),
                    EditField::Tags => (0, 0),
                    EditField::DueDate => (
                        chunks[2].x + 1 + state.cursor_position as u16,
                        chunks[2].y + 1,
                    ),
                    EditField::Description => (
                        chunks[3].x + 1 + (cursor_col.saturating_sub(state.scroll_x)),
                        chunks[3].y + 1 + (cursor_row.saturating_sub(state.scroll_y)),
                    ),
                };
                f.set_cursor_position((x, y));
            }
        }
    }
}

fn draw_exit_confirmation(f: &mut Frame) {
    let area = centered_rect(40, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red))
        .title(" Unsaved Changes ");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Save changes before closing?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y] Yes (Save)", Style::default().fg(Color::Green)),
            Span::raw("   "),
            Span::styled("[n] No (Discard)", Style::default().fg(Color::Red)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "[Esc] Cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn draw_tag_selector(f: &mut Frame, app: &App) {
    if let Some(selector) = &app.tag_selector_state {
        let area = centered_rect(40, 50, f.area());
        f.render_widget(Clear, area);

        // FIX: Updated Title to reflect that Esc saves
        let block = Block::default()
            .title(" Select Tags (Enter: Toggle | Esc: Save & Close) ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        // ... rest of the function remains exactly the same
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search Bar
                Constraint::Min(1),    // List
            ])
            .split(inner_area);

        // 1. Search Bar
        let search_text = if selector.search_query.is_empty() {
            Span::styled(
                "Type to filter/create...",
                Style::default().fg(Color::DarkGray),
            )
        } else {
            Span::raw(&selector.search_query)
        };

        let search_block = Paragraph::new(search_text)
            .block(Block::default().borders(Borders::BOTTOM).title(" Search "));
        f.render_widget(search_block, chunks[0]);

        // 2. Filtered List
        let filtered = app.get_filtered_tags();
        let mut items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(view_idx, (real_idx, tag))| {
                let is_selected = selector.selected_indices.contains(real_idx);
                let checkbox = if is_selected { "[x] " } else { "[ ] " };
                let content = format!("{}{}", checkbox, tag.name);

                let style = if view_idx == selector.current_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(content, style),
                    Span::raw(" "),
                    Span::styled("●", Style::default().fg(tag.get_color())),
                ]))
            })
            .collect();

        if !selector.search_query.is_empty() {
            let is_highlighted = selector.current_index == filtered.len();
            let style = if is_highlighted {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!("+ Create tag \"{}\"", selector.search_query),
                style,
            )])));
        }

        let list = List::new(items);
        f.render_widget(list, chunks[1]);
    }
}
