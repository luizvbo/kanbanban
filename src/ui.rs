use crate::app::{App, EditField, InputMode};
use crate::types::{Card, KanbanData};
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
        InputMode::ProjectRename => draw_input_modal(f, " Rename Board ", &app.prompt_input),
        InputMode::ColumnRename => draw_input_modal(f, " Rename Column ", &app.prompt_input),
        InputMode::ColumnNew => draw_input_modal(f, " New Column Title ", &app.prompt_input),
        InputMode::ColumnDelete => draw_column_delete_modal(f, app),
        _ => {}
    }
}

fn draw_input_modal(f: &mut Frame, title: &str, input: &str) {
    let area = centered_rect(50, 15, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let input_widget = Paragraph::new(input)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::NONE)); // No inner border

    // Center the text vertically in the small box
    let vertical_center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    f.render_widget(input_widget, vertical_center[1]);

    // Draw cursor
    f.set_cursor_position((
        vertical_center[1].x + input.len() as u16,
        vertical_center[1].y,
    ));
}

fn draw_column_delete_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 25, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red))
        .title(" ⚠ DELETE COLUMN ⚠ ");

    let col_name = if let Some(col) = app.current_project().columns.get(app.current_col_idx) {
        &col.title
    } else {
        "Unknown"
    };

    let card_count = if let Some(col) = app.current_project().columns.get(app.current_col_idx) {
        col.cards.len()
    } else {
        0
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Delete column \"{}\"?", col_name),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "WARNING: This will delete all {} cards in this column!",
                card_count
            ),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from("This action cannot be undone."),
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

fn draw_board(f: &mut Frame, app: &mut App, area: Rect) {
    // 1. Get basic info first to avoid holding a borrow of 'app' too long
    let project_idx = app.current_project_idx;
    let n_columns = app.data.projects[project_idx].columns.len();

    if n_columns == 0 {
        return;
    }

    let constraints: Vec<Constraint> = (0..n_columns)
        .map(|_| Constraint::Percentage(100 / n_columns as u16))
        .collect();
    let col_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for i in 0..n_columns {
        let is_col_focused = app.current_col_idx == i;
        let selected_card_idx = app.current_card_idx;

        // --- SCOPE 1: MUTABLE CALCULATION ---
        // We open a block so the mutable borrow 'col' is dropped as soon as the block ends.
        {
            let col = &mut app.data.projects[project_idx].columns[i];

            if is_col_focused && !col.cards.is_empty() {
                // Ensure scroll_offset is not beyond current selection
                if col.scroll_offset > selected_card_idx {
                    col.scroll_offset = selected_card_idx;
                }

                // Calculate heights to check if selected card is out of view
                let block = Block::default().borders(Borders::ALL);
                let inner_area = block.inner(col_chunks[i]);
                let available_height = inner_area.height;

                let mut start_draw_idx = col.scroll_offset;

                loop {
                    let mut current_height = 0;
                    let mut selected_in_view = false;

                    for (j, card) in col.cards.iter().enumerate().skip(start_draw_idx) {
                        let card_h = get_card_height(card);
                        if current_height + card_h > available_height {
                            break;
                        }
                        current_height += card_h;
                        if j == selected_card_idx {
                            selected_in_view = true;
                            break;
                        }
                    }

                    if selected_in_view {
                        col.scroll_offset = start_draw_idx;
                        break;
                    } else {
                        if start_draw_idx < col.cards.len() - 1 {
                            start_draw_idx += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
        } // <--- Mutable borrow of 'app.data' ends here!

        // --- SCOPE 2: IMMUTABLE RENDERING ---
        // Now we can safely borrow app.data immutably.
        let col = &app.data.projects[project_idx].columns[i];

        let border_style = if is_col_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(if is_col_focused {
                BorderType::Thick
            } else {
                BorderType::Plain
            })
            .border_style(border_style)
            .title(format!(" {} ({}) ", col.title, col.cards.len()));

        f.render_widget(block.clone(), col_chunks[i]);
        let inner_area = block.inner(col_chunks[i]);

        if col.cards.is_empty() {
            continue;
        }

        let mut y_offset = inner_area.y;
        for (j, card) in col.cards.iter().enumerate().skip(col.scroll_offset) {
            let card_height = get_card_height(card);

            if y_offset + card_height > inner_area.bottom() {
                break;
            }

            let card_area = Rect {
                x: inner_area.x,
                y: y_offset,
                width: inner_area.width,
                height: card_height,
            };

            let is_selected = is_col_focused && selected_card_idx == j;

            // This now works because 'col' and '&app.data' are both immutable borrows.
            draw_card(f, card, card_area, is_selected, &app.data);
            y_offset += card_height;
        }
    }
}

fn get_card_height(card: &Card) -> u16 {
    let mut h = 2; // Borders
    h += 1; // Title
    if !card.tags.is_empty() {
        h += 1;
    }
    if card.due_date.is_some() {
        h += 1;
    }

    // Estimate description height (simple line count approximation)
    // For perfect scrolling, we'd need to wrap text based on width, but that's expensive here.
    // We'll assume 1 line per newline + 1 buffer.
    let desc_lines = card.description.matches('\n').count() as u16 + 1;
    // Cap description preview at 4 lines for calculation stability if needed,
    // or use actual lines. Let's use actual lines but clamp min.
    h += std::cmp::max(1, desc_lines);

    h
}

fn draw_card(f: &mut Frame, card: &Card, area: Rect, is_selected: bool, data: &KanbanData) {
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

    // Layout
    let mut constraints = vec![Constraint::Length(1)]; // Title
    if !card.tags.is_empty() {
        constraints.push(Constraint::Length(1));
    }
    if card.due_date.is_some() {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Min(1)); // Description

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

    // 2. Tags
    if !card.tags.is_empty() {
        let mut tag_spans = Vec::new();
        for tag in &card.tags {
            // FIX: Use data.get_tag_color
            let color = data.get_tag_color(tag);
            tag_spans.push(Span::styled(
                format!(" #{} ", tag.name),
                Style::default().bg(color).fg(Color::Black),
            ));
            tag_spans.push(Span::raw(" "));
        }
        f.render_widget(Line::from(tag_spans), chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // 3. Date
    if let Some(date_str) = &card.due_date {
        // ... (Date logic same as before)
        let date_display = Span::styled(
            format!("🕒 {}", date_str),
            Style::default().fg(Color::DarkGray),
        ); // Simplified for brevity
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
        InputMode::DeleteConfirmation => " DELETE CARD ",
        InputMode::ProjectRename => " RENAME BOARD ",
        InputMode::ColumnRename => " RENAME COL ",
        InputMode::ColumnNew => " NEW COL ",
        InputMode::ColumnDelete => " DELETE COL ",
    };
    let mode_color = match app.mode {
        InputMode::Normal => Color::Blue,
        InputMode::Editing => Color::Yellow,
        InputMode::TagSelection => Color::Magenta,
        InputMode::Help => Color::Green,
        InputMode::ExitingModal => Color::Red,
        InputMode::DeleteConfirmation => Color::Red,
        InputMode::ProjectRename => Color::Cyan,
        InputMode::ColumnRename => Color::Cyan,
        InputMode::ColumnNew => Color::Green,
        InputMode::ColumnDelete => Color::Red,
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
    let area = centered_rect(60, 60, f.area()); // Slightly taller

    let help_text = vec![
        Line::from(Span::styled(
            "Kanban TUI Help",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled("Navigation", Style::default().fg(Color::Cyan))),
        Line::from("  h / l            : Move between columns"),
        Line::from("  j / k            : Move between cards"),
        Line::from(""),
        Line::from(Span::styled(
            "Card Actions",
            Style::default().fg(Color::Cyan),
        )),
        Line::from("  H / L            : Move card Left/Right"),
        Line::from("  J / K            : Move card Up/Down"),
        Line::from("  a / n            : Add new card"),
        Line::from("  i / e / Enter    : Edit card"),
        Line::from("  d                : Delete card"),
        Line::from(""),
        Line::from(Span::styled(
            "Column Actions",
            Style::default().fg(Color::Cyan),
        )),
        Line::from("  c                : New Column"),
        Line::from("  r                : Rename Column"),
        Line::from("  D                : Delete Column"),
        Line::from("  < / >            : Move Column Left/Right"), // NEW
        Line::from(""),
        Line::from(Span::styled(
            "Board Actions",
            Style::default().fg(Color::Cyan),
        )),
        Line::from("  R                : Rename Board"),
        Line::from(""),
        Line::from(Span::styled("General", Style::default().fg(Color::Cyan))),
        Line::from("  ?                : Toggle this popup"),
        Line::from("  q                : Quit"),
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

                // FIX: Use app.data.get_tag_color
                let color = app.data.get_tag_color(tag);

                ListItem::new(Line::from(vec![
                    Span::styled(content, style),
                    Span::raw(" "),
                    Span::styled("●", Style::default().fg(color)),
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
