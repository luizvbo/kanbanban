use crate::app::state::{EditField, SelectorType};
use crate::app::{App, InputMode};
use crate::ui::widgets::{centered_fixed_rect, centered_rect, create_input_block, parse_markdown};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn draw_input_modal(f: &mut Frame, title: &str, input: &str) {
    // Use fixed height of 3 rows (Border + Text + Border)
    let area = centered_fixed_rect(60, 3, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let input_widget = Paragraph::new(format!("> {}", input))
        .style(Style::default().fg(Color::Yellow))
        .block(block);

    f.render_widget(input_widget, area);

    // Position cursor: Area X + Border(1) + "> "(2) + Input Len
    f.set_cursor_position((area.x + 3 + input.len() as u16, area.y + 1));
}

pub fn draw_column_delete_modal(f: &mut Frame, app: &App) {
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

pub fn draw_delete_confirmation(f: &mut Frame) {
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

pub fn draw_exit_confirmation(f: &mut Frame) {
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

pub fn draw_detail_view(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 80, f.area());
    f.render_widget(Clear, area);

    let col = &app.current_project().columns[app.current_col_idx];
    if col.cards.is_empty() {
        return;
    }
    let card = &col.cards[app.current_card_idx];

    let block = Block::default()
        .title(format!(" {} ", card.title))
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .style(Style::default().bg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Meta (Category | Date)
            Constraint::Length(1), // Tags
            Constraint::Length(1), // Separator
            Constraint::Min(1),    // Description
            Constraint::Length(1), // Footer hint
        ])
        .split(inner);

    // 1. Meta Row
    let mut meta_spans = Vec::new();
    if let Some(cat) = &card.category {
        meta_spans.push(Span::styled(
            format!("PROJECT: {} ", cat),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        meta_spans.push(Span::raw(" | "));
    }
    if let Some(date) = &card.due_date {
        meta_spans.push(Span::styled(
            format!("DUE: {} ", date),
            Style::default().fg(Color::Yellow),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(meta_spans)), chunks[0]);

    // 2. Tags
    let mut tag_spans = Vec::new();
    for tag in &card.tags {
        let color = app.data.get_tag_color(tag);
        tag_spans.push(Span::styled(
            format!(" #{} ", tag.name),
            Style::default().bg(color).fg(Color::Black),
        ));
        tag_spans.push(Span::raw(" "));
    }
    f.render_widget(Line::from(tag_spans), chunks[1]);

    // 3. Separator
    f.render_widget(Block::default().borders(Borders::BOTTOM), chunks[2]);

    // 4. Description (Scrollable)
    let markdown = parse_markdown(&card.description);
    f.render_widget(
        Paragraph::new(markdown)
            .wrap(Wrap { trim: false })
            .scroll((app.view_scroll_y, 0)),
        chunks[3],
    );

    // 5. Footer
    f.render_widget(
        Paragraph::new("j/k: Scroll | o: Open External | Esc: Close")
            .style(Style::default().fg(Color::Gray)),
        chunks[4],
    );
}

pub fn draw_selector(f: &mut Frame, app: &App) {
    if let Some(selector) = &app.selector_state {
        let area = centered_rect(40, 50, f.area());
        f.render_widget(Clear, area);

        let title = match selector.mode {
            SelectorType::Tag => " Select Tags ",
            SelectorType::Category => " Select Category ",
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search
                Constraint::Min(1),    // List
                Constraint::Length(1), // Footer
            ])
            .split(inner_area);

        // 1. Search
        let search_text = if selector.search_query.is_empty() {
            Span::styled(
                "Type to filter/create...",
                Style::default().fg(Color::DarkGray),
            )
        } else {
            Span::raw(&selector.search_query)
        };
        f.render_widget(
            Paragraph::new(search_text)
                .block(Block::default().borders(Borders::BOTTOM).title(" Search ")),
            chunks[0],
        );

        // 2. List
        let filtered = selector.get_filtered_items();
        let mut items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(view_idx, (_, tag))| {
                let is_selected = selector.selected_items.contains(&tag.name);

                // UX: Distinguish between Multi-select (Tag) and Single-select (Category)
                let indicator = match selector.mode {
                    SelectorType::Tag => {
                        if is_selected {
                            "[x] "
                        } else {
                            "[ ] "
                        }
                    }
                    SelectorType::Category => {
                        if is_selected {
                            "(●) "
                        } else {
                            "( ) "
                        }
                    }
                };

                let content = format!("{}{}", indicator, tag.name);

                let style = if view_idx == selector.current_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                // Parse the color stored in the tag
                let color = if let Some(c) = &tag.color {
                    crate::domain::kanban::Tag::parse_color_string(c)
                } else {
                    Color::White
                };

                ListItem::new(Line::from(vec![
                    Span::styled(content, style),
                    Span::raw(" "),
                    Span::styled("■", Style::default().fg(color)), // Show color swatch
                ]))
            })
            .collect();

        // "Create New" Option
        if !selector.search_query.is_empty() {
            let is_highlighted = selector.current_index == filtered.len();
            let style = if is_highlighted {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            items.push(ListItem::new(Span::styled(
                format!("+ Create \"{}\"", selector.search_query),
                style,
            )));
        }

        f.render_widget(List::new(items), chunks[1]);

        // 3. Footer Instructions
        let footer = "Enter: Toggle | Del: Delete | Esc: Done";
        f.render_widget(
            Paragraph::new(footer).style(Style::default().fg(Color::Cyan)),
            chunks[2],
        );
    }
}

pub fn draw_edit_popup(f: &mut Frame, app: &mut App) {
    if let Some(state) = &mut app.edit_state {
        let area = centered_rect(70, 80, f.area());
        f.render_widget(Clear, area);
        let block = Block::default().borders(Borders::ALL).title(" Edit Card ");
        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Category
                Constraint::Length(3), // Tags
                Constraint::Length(3), // Due Date
                Constraint::Min(5),    // Description
                Constraint::Length(1), // Footer
            ])
            .split(inner);

        // 1. Title
        let title_widget = create_input_block(
            "Title",
            &state.title,
            state.focused_field == EditField::Title,
        );
        f.render_widget(title_widget, chunks[0]);

        // 2. Category (Updated visual to indicate it's a selector)
        let cat_style = if state.focused_field == EditField::Category {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let cat_text = if state.category.is_empty() {
            Span::styled(
                "Select Category (Enter)...",
                Style::default().fg(Color::DarkGray),
            )
        } else {
            Span::raw(&state.category)
        };
        f.render_widget(
            Paragraph::new(cat_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Category (Enter to Select)")
                    .border_style(cat_style),
            ),
            chunks[1],
        );

        // 3. Tags
        let tags_str = state
            .tags
            .iter()
            .map(|t| format!("#{}", t.name))
            .collect::<Vec<_>>()
            .join(" ");
        let tags_widget = create_input_block(
            "Tags (Enter to Select)",
            &tags_str,
            state.focused_field == EditField::Tags,
        );
        f.render_widget(tags_widget, chunks[2]);

        // 4. Due Date
        let date_widget = create_input_block(
            "Due Date",
            &state.due_date,
            state.focused_field == EditField::DueDate,
        );
        f.render_widget(date_widget, chunks[3]);

        // 5. Description - FIX FOR ISSUE #1: Ensure Multi-line Text Area behavior
        let (_cursor_col, cursor_row) =
            state.get_cursor_position_2d(chunks[4].width.saturating_sub(2));

        // Scroll logic
        let desc_height = chunks[4].height.saturating_sub(2);
        if cursor_row >= state.scroll_y + desc_height {
            state.scroll_y = cursor_row - desc_height + 1;
        } else if cursor_row < state.scroll_y {
            state.scroll_y = cursor_row;
        }

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
            (Style::default(), " Description ")
        };

        let desc_widget = Paragraph::new(desc_content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(desc_title)
                    .border_style(desc_border_style),
            )
            .wrap(Wrap { trim: false })
            .scroll((state.scroll_y, 0));

        f.render_widget(desc_widget, chunks[4]);

        // 6. Footer (Updated instructions)
        let footer_text =
            if state.focused_field == EditField::Description && state.description_edit_mode {
                "ESC: Stop Editing | TAB: Indent | ENTER: Newline | o: External Editor"
            } else {
                "TAB/UP/DOWN: Navigate | ENTER: Edit Field | ESC: Save & Close"
            };
        f.render_widget(
            Paragraph::new(footer_text).style(Style::default().fg(Color::Cyan)),
            chunks[5],
        );

        // Render Cursor
        if app.mode == InputMode::Editing {
            let show_cursor = state.focused_field != EditField::Tags
                && state.focused_field != EditField::Category;
            if show_cursor {
                let (x, y) = match state.focused_field {
                    EditField::Title => (
                        chunks[0].x + 1 + state.cursor_position as u16,
                        chunks[0].y + 1,
                    ),
                    EditField::Category => (0, 0), // Handled by modal
                    EditField::Tags => (0, 0),     // Handled by modal
                    EditField::DueDate => (
                        chunks[3].x + 1 + state.cursor_position as u16,
                        chunks[3].y + 1,
                    ),
                    EditField::Description => {
                        let (col, row) =
                            state.get_cursor_position_2d(chunks[4].width.saturating_sub(2));
                        (
                            chunks[4].x + 1 + col,
                            chunks[4].y + 1 + row - state.scroll_y,
                        )
                    }
                };
                f.set_cursor_position((x, y));
            }
        }
    }
}

pub fn draw_help_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 80, f.area());
    f.render_widget(Clear, area);

    let app_name = env!("CARGO_PKG_NAME").to_uppercase();
    let app_version = env!("CARGO_PKG_VERSION");

    // Explicitly define the Vec as holding 'static lifetimes
    let mut help_text: Vec<Line<'static>> = Vec::new();

    // --- Header ---
    help_text.push(Line::from(vec![
        Span::styled(
            format!(" {} ", app_name),
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" v{} ", app_version),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    help_text.push(Line::from(""));

    // --- Closure with owned String conversion ---
    // We use .to_string() on title and desc to move ownership into the Line
    let add_section = |lines: &mut Vec<Line<'static>>, title: &str, bindings: Vec<(&str, &str)>| {
        lines.push(Line::from(Span::styled(
            title.to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in bindings {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:>10} ", key), Style::default().fg(Color::Cyan)),
                Span::raw("→ "),
                Span::styled(desc.to_string(), Style::default().fg(Color::White)),
            ]));
        }
        lines.push(Line::from(""));
    };

    // --- Content Sections ---
    add_section(
        &mut help_text,
        "Navigation",
        vec![
            ("h / l", "Switch Columns"),
            ("j / k", "Select Card"),
            ("v", "View Details"),
            ("/", "Filter Board"),
        ],
    );

    add_section(
        &mut help_text,
        "Card Actions",
        vec![
            ("a / n", "Add New Card"),
            ("Enter", "Edit Card"),
            ("d", "Delete Card"),
            ("o", "External Editor (Vim/Nano)"),
        ],
    );

    add_section(
        &mut help_text,
        "Movement",
        vec![
            ("H / L", "Move Card to Left/Right Column"),
            ("J / K", "Move Card Up/Down in Column"),
            ("< / >", "Reorder Columns"),
        ],
    );

    add_section(
        &mut help_text,
        "Board/Column",
        vec![
            ("R", "Rename Board"),
            ("r", "Rename Column"),
            ("c", "Create New Column"),
            ("D", "Delete Current Column"),
        ],
    );

    // --- Footer / Path ---
    // Added a simple separator line
    help_text.push(Line::from(Span::styled(
        "─".repeat(area.width as usize - 4),
        Style::default().fg(Color::DarkGray),
    )));

    help_text.push(Line::from(vec![
        Span::styled(" Storage Path: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            app.filepath.to_string_lossy().to_string(),
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        ),
    ]));

    let block = Block::default()
        .title(" Help Menu ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    f.render_widget(
        Paragraph::new(help_text)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
