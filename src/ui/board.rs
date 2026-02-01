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

// Helper to safely truncate string by visual width (char count)
fn safe_truncate(s: &str, max_width: usize) -> (String, String) {
    let mut split_idx = 0;

    for (width, (i, c)) in s.char_indices().enumerate() {
        // for (i, c) in s.char_indices() {
        if width >= max_width {
            break;
        }
        split_idx = i + c.len_utf8();
    }

    let keep = s[..split_idx].to_string();
    let rest = if split_idx < s.len() {
        s[split_idx..].to_string()
    } else {
        String::new()
    };

    (keep, rest)
}

/// The main drawing loop for the Kanban columns.
/// It splits the available area into equal chunks based on column count.
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

/// Calculates the height of a card based on which fields are populated.
/// This must return a consistent value to ensure the board's vertical
/// scrolling doesn't jitter.
pub fn get_card_height(card: &Card) -> u16 {
    // Current design uses a fixed 2-line max for descriptions
    // to keep the board compact.
    let mut h = 2; // Top and Bottom Borders

    // 1. Title row (always 1)
    h += 1;

    // 2. Tags row
    if !card.tags.is_empty() {
        h += 1;
    }

    // 3. Due Date row
    if card.due_date.is_some() {
        h += 1;
    }

    // 4. Description Preview
    // We now fix this to a maximum of 2 lines for the board view.
    // If empty, we still give it 1 line for "No description" or empty space.
    if card.description.trim().is_empty() {
        h += 1;
    } else {
        h += 2;
    }

    h
}

/// Renders an individual card widget.
/// Handles text truncation and the "... [v] Details" hint.
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

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Layout Constraints
    let mut constraints = vec![Constraint::Length(1)]; // Title (Row 1)

    // Category + Tags (Row 2) - Always reserve space if either exists
    if card.category.is_some() || !card.tags.is_empty() {
        constraints.push(Constraint::Length(1));
    }

    if card.due_date.is_some() {
        constraints.push(Constraint::Length(1));
    }

    // Fixed length for description area (2 lines)
    constraints.push(Constraint::Length(2));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut chunk_idx = 0;

    // --- 1. Title Only (Yellow/Bold) ---
    let title_span = Span::styled(&card.title, Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(
        Paragraph::new(title_span).wrap(Wrap { trim: true }),
        chunks[chunk_idx],
    );
    chunk_idx += 1;

    // --- 2. Category + Tags (Combined Row) ---
    if card.category.is_some() || !card.tags.is_empty() {
        let mut meta_spans = Vec::new();

        // Category
        if let Some(cat) = &card.category {
            let cat_color = data.get_category_color(cat);
            meta_spans.push(Span::styled("(", Style::default().fg(cat_color)));
            meta_spans.push(Span::styled(
                cat.to_uppercase(),
                Style::default().fg(cat_color).add_modifier(Modifier::BOLD),
            ));
            meta_spans.push(Span::styled(") ", Style::default().fg(cat_color)));
        }

        // Tags
        for tag in &card.tags {
            let color = data.get_tag_color(tag);
            meta_spans.push(Span::styled(
                format!("#{} ", tag.name),
                Style::default().bg(color).fg(Color::Black),
            ));
            meta_spans.push(Span::raw(" "));
        }

        f.render_widget(Line::from(meta_spans), chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // --- 3. Date ---
    if let Some(date_str) = &card.due_date {
        let date_display = Span::styled(
            format!("🕒 {}", date_str),
            Style::default().fg(Color::DarkGray),
        );
        f.render_widget(Paragraph::new(date_display), chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // --- 4. Description ---
    let max_width = inner.width as usize;
    let full_markdown = parse_markdown(&card.description);

    let all_spans: Vec<Span> = full_markdown
        .lines
        .into_iter()
        .flat_map(|line| {
            let mut spans = line.spans;
            spans.push(Span::raw(" "));
            spans
        })
        .collect();

    let suffix = "... [v] to view";
    let suffix_len = suffix.len();

    let mut lines_to_draw = Vec::new();
    let mut current_line_spans = Vec::new();
    let mut current_width = 0;
    let mut line_count = 0;

    for span in all_spans {
        if line_count >= 2 {
            break;
        }

        let content = span.content.as_ref();
        let span_len = content.chars().count();

        let available = if line_count == 1 {
            max_width.saturating_sub(suffix_len)
        } else {
            max_width
        };

        if current_width + span_len <= available {
            current_line_spans.push(span);
            current_width += span_len;
        } else {
            let remaining = available.saturating_sub(current_width);

            // FIX: Use safe_truncate instead of split_at
            if remaining > 0 {
                let (keep, _) = safe_truncate(content, remaining);
                current_line_spans.push(Span::styled(keep, span.style));
            }

            lines_to_draw.push(Line::from(current_line_spans.clone()));
            current_line_spans.clear();
            line_count += 1;

            if line_count >= 2 {
                break;
            }

            let (_, rest) = safe_truncate(content, remaining);
            let next_avail = if line_count == 1 {
                max_width.saturating_sub(suffix_len)
            } else {
                max_width
            };

            // Safe truncate for the next line part as well
            let (keep_next, _) = safe_truncate(&rest, next_avail);
            current_width = keep_next.chars().count();
            current_line_spans.push(Span::styled(keep_next, span.style));
        }
    }

    if !current_line_spans.is_empty() && line_count < 2 {
        lines_to_draw.push(Line::from(current_line_spans));
    }

    // Append suffix logic
    let raw_desc_len: usize = card.description.len();
    let rendered_len: usize = lines_to_draw.iter().map(|l| l.width()).sum();

    if lines_to_draw.len() == 2 || raw_desc_len > rendered_len + 5 {
        if let Some(last_line) = lines_to_draw.last_mut() {
            last_line.spans.push(Span::styled(
                suffix,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ));
        } else {
            lines_to_draw.push(Line::from(Span::styled(
                suffix,
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    f.render_widget(Paragraph::new(lines_to_draw), chunks[chunk_idx]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_truncate() {
        // Test standard ASCII
        let (k, r) = safe_truncate("Hello", 2);
        assert_eq!(k, "He");
        assert_eq!(r, "llo");

        // Test Multi-byte (Bullet point is 3 bytes)
        let (k, r) = safe_truncate("• List", 1);
        assert_eq!(k, "•"); // Should not panic
        assert_eq!(r, " List");

        // Test Boundary
        let (k, r) = safe_truncate("ABC", 3);
        assert_eq!(k, "ABC");
        assert_eq!(r, "");
    }
}
