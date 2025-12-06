use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    // Left Side: Instructions
    let info = Paragraph::new(
        "Settings\n\n\
        j/k: Select Column\n\
        Space: Toggle Visibility\n\
        r: Rename Column\n\
        K: Move Up\n\
        J: Move Down\n\
        1: Set as Start Column\n\
        2: Set as Finish Column\n\
        3: Set as Reset Column",
    )
    .block(Block::default().borders(Borders::ALL).title(" Controls "));

    f.render_widget(info, chunks[0]);

    // Right Side: Column List
    let board = &app.boards[app.active_board_index];

    let items: Vec<ListItem> = app
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let is_selected = i == app.settings_state.selected_column_idx;

            let mut tags = vec![];
            if !col.visible {
                tags.push("[Hidden]");
            }
            if Some(col.id) == board.start_column_id {
                tags.push("[START]");
            }
            if Some(col.id) == board.finish_column_id {
                tags.push("[FINISH]");
            }
            if Some(col.id) == board.reset_column_id {
                tags.push("[RESET]");
            }

            let tag_str = tags.join(" ");
            let content = format!("{} {}", col.name, tag_str);

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Column Management "),
    );

    f.render_widget(list, chunks[1]);
}
