use crate::app::{App, HitZone};
use crate::ui::markdown::parse_markdown;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    app.hit_zones.clear();
    let mut new_zones = Vec::new();

    {
        let visible_columns: Vec<&crate::models::Column> =
            app.columns.iter().filter(|c| c.visible).collect();

        if !visible_columns.is_empty() {
            let constraints: Vec<Constraint> = visible_columns
                .iter()
                .map(|_| Constraint::Percentage(100 / visible_columns.len() as u16))
                .collect();
            let col_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .split(area);

            for (i, column) in visible_columns.iter().enumerate() {
                let real_idx = app
                    .columns
                    .iter()
                    .position(|c| c.id == column.id)
                    .unwrap_or(0);

                new_zones.push((col_chunks[i], HitZone::Column(real_idx)));

                let is_col_selected = real_idx == app.selected_column_index;
                let tasks = app.get_tasks_in_column(Some(column.id));

                let block_style = if is_col_selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };

                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(column.name.clone())
                    .border_style(block_style);

                f.render_widget(block.clone(), col_chunks[i]);

                let inner_area = block.inner(col_chunks[i]);
                let mut current_y = inner_area.y;

                for (j, task) in tasks.iter().enumerate() {
                    let desc_lines = task
                        .inner
                        .description
                        .as_ref()
                        .map(|d| d.lines().count())
                        .unwrap_or(0);
                    let card_height = (4 + desc_lines.min(5)) as u16;

                    if current_y + card_height > inner_area.y + inner_area.height {
                        break;
                    }

                    let task_area =
                        Rect::new(inner_area.x, current_y, inner_area.width, card_height);
                    new_zones.push((task_area, HitZone::Task(real_idx, j)));

                    let is_task_selected = is_col_selected && j == app.selected_task_index;

                    let mut cat_color = Color::White;
                    if let Some(cid) = task.inner.category_id {
                        if let Some(c) = app.category_map.get(&cid) {
                            cat_color = *c;
                        }
                    }

                    let border_style = if is_task_selected {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };

                    let card_block = Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_style)
                        .style(Style::default().bg(Color::Reset));

                    f.render_widget(card_block.clone(), task_area);
                    let card_inner = card_block.inner(task_area);

                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(1), // Title
                            Constraint::Min(0),    // Markdown Desc
                            Constraint::Length(1), // Footer (Dates)
                        ])
                        .split(card_inner);

                    // 1. Title
                    f.render_widget(
                        Paragraph::new(Span::styled(
                            task.inner.title.clone(),
                            Style::default().fg(cat_color).add_modifier(Modifier::BOLD),
                        )),
                        chunks[0],
                    );

                    // 2. Description
                    if let Some(desc) = &task.inner.description {
                        let md_lines = parse_markdown(desc);
                        f.render_widget(
                            Paragraph::new(md_lines).wrap(Wrap { trim: true }),
                            chunks[1],
                        );
                    }

                    // 3. Footer (Creation Date + Due Date)
                    let days_created = task.inner.days_since_creation();
                    let created_str = if days_created == 0 {
                        "Today".to_string()
                    } else if days_created == 1 {
                        "Yesterday".to_string()
                    } else {
                        format!("{days_created}d ago")
                    };

                    let mut footer_line = Line::from(vec![Span::styled(
                        format!("📅 {created_str}"),
                        Style::default().fg(Color::Gray),
                    )]);

                    if let Some(days) = task.inner.days_left() {
                        let (due_str, color) = if days < 0 {
                            (format!(" Overdue by {} days", days.abs()), Color::Red)
                        } else if days == 0 {
                            (" Due Today".to_string(), Color::Yellow)
                        } else {
                            (format!(" Due in {days} days"), Color::Gray)
                        };

                        footer_line
                            .spans
                            .push(Span::styled(due_str, Style::default().fg(color)));
                    }

                    f.render_widget(Paragraph::new(footer_line), chunks[2]);

                    current_y += card_height;
                }
            }
        }
    }

    app.hit_zones.extend(new_zones);
}

pub fn render_selector(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .boards
        .iter()
        .enumerate()
        .map(|(i, board)| {
            let style = if i == app.board_selector_index {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!(
                "{} {}",
                board.icon.clone().unwrap_or_default(),
                board.name
            ))
            .style(style)
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(" Select Board (n: New, e: Edit, d: Delete) ");

    f.render_widget(List::new(items.clone()).block(list_block.clone()), area);

    let inner_list_area = list_block.inner(area); // This is the area *inside* the block borders
    let item_height = 1; // Assuming each ListItem takes 1 row for its content

    let mut new_selector_zones = Vec::new();
    for (i, board) in app.boards.iter().enumerate() {
        // Calculate the Y position for the start of this item
        let item_y = inner_list_area.y + i as u16 * item_height;

        // Ensure the item_rect is within the visible bounds of the inner_list_area
        if item_y < inner_list_area.y + inner_list_area.height {
            let item_rect = Rect::new(
                inner_list_area.x,
                item_y,
                inner_list_area.width,
                item_height,
            );
            new_selector_zones.push((item_rect, HitZone::BoardSelector(i)));
            tracing::debug!(
                "render_selector: Board '{}' (idx {}) hit zone: Rect({:?}), AppBoardsLen={}",
                board.name,
                i,
                item_rect,
                app.boards.len()
            );
        }
    }
    app.hit_zones.extend(new_selector_zones);
}
