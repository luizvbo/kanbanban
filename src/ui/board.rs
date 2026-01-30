use crate::domain::kanban::KanbanData;
use crate::ui::widgets::parse_markdown;
use crate::{app::App, domain::kanban::Card};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

pub fn draw_board(f: &mut Frame, app: &mut App, area: Rect) {
    let project_idx = app.current_project_idx;
    let n_columns = app.data.projects[project_idx].columns.len();

    if n_columns == 0 {
        return;
    }

    // FIX 1: Extract filter query here to avoid borrowing `app` inside the mutable loop
    let filter_query = app.filter_query.to_lowercase();
    let has_filter = !filter_query.is_empty();

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
        {
            let col = &mut app.data.projects[project_idx].columns[i];

            // FIX 2: Inline the filter logic using the local `filter_query` string
            // This avoids the "second borrow of app" error.
            let visible_indices: Vec<usize> = col
                .cards
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    if !has_filter {
                        return true;
                    }
                    c.title.to_lowercase().contains(&filter_query)
                        || c.category
                            .as_ref()
                            .is_some_and(|cat| cat.to_lowercase().contains(&filter_query))
                        || c.tags
                            .iter()
                            .any(|t| t.name.to_lowercase().contains(&filter_query))
                })
                .map(|(idx, _)| idx)
                .collect();

            if is_col_focused && !visible_indices.is_empty() {
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
                    } else if start_draw_idx < col.cards.len() - 1 {
                        start_draw_idx += 1;
                    } else {
                        break;
                    }
                }
            }
        }

        // --- SCOPE 2: IMMUTABLE RENDERING ---
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
            // FIX 3: Use the same local filter logic here
            if has_filter {
                let matches = card.title.to_lowercase().contains(&filter_query)
                    || card
                        .category
                        .as_ref()
                        .is_some_and(|cat| cat.to_lowercase().contains(&filter_query))
                    || card
                        .tags
                        .iter()
                        .any(|t| t.name.to_lowercase().contains(&filter_query));

                if !matches {
                    continue;
                }
            }

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

            draw_card(f, card, card_area, is_selected, &app.data);
            y_offset += card_height;
        }
    }
}

pub fn get_card_height(card: &Card) -> u16 {
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

pub fn draw_card(f: &mut Frame, card: &Card, area: Rect, is_selected: bool, data: &KanbanData) {
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

    let inner = block.inner(area); // FIX: Call inner before moving block
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
        .split(inner); // Use pre-calculated inner

    let mut chunk_idx = 0;

    // 1. Title + Category
    let mut title_spans = Vec::new();
    if let Some(cat) = &card.category {
        title_spans.push(Span::styled(
            format!(" {} ", cat),
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::raw(" "));
    }
    title_spans.push(Span::styled(
        &card.title,
        Style::default().add_modifier(Modifier::BOLD),
    ));

    f.render_widget(Paragraph::new(Line::from(title_spans)), chunks[chunk_idx]);
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
