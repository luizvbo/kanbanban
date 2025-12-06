use super::centered_rect;
use crate::app::{ActiveModal, App, BoardFocus, CategoryFocus, ModalType, TaskFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, modal: &ActiveModal) {
    let area = centered_rect(60, 50, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));

    match modal {
        ActiveModal::Task {
            title,
            desc,
            date,
            category_idx,
            focus,
            ..
        } => {
            f.render_widget(block.title(" Task Details "), area);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3), // Title
                    Constraint::Min(3),    // Description
                    Constraint::Length(3), // Due Date
                    Constraint::Length(3), // Category Selector
                    Constraint::Length(1), // Help
                ])
                .split(area);

            let active_style = Style::default().fg(Color::Yellow);
            let inactive_style = Style::default().fg(Color::White);

            // Title Input
            let mut t_widget = title.clone();
            t_widget.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Title")
                    .border_style(if *focus == TaskFocus::Title {
                        active_style
                    } else {
                        inactive_style
                    }),
            );

            // Description Input
            let mut d_widget = desc.clone();
            d_widget.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Description")
                    .border_style(if *focus == TaskFocus::Description {
                        active_style
                    } else {
                        inactive_style
                    }),
            );

            // Date Input
            let mut date_widget = date.clone();
            date_widget.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Due Date (Ctrl+D to Pick)") // Updated Title
                    .border_style(if *focus == TaskFocus::Date {
                        active_style
                    } else {
                        inactive_style
                    }),
            );

            // Category Selector (Custom Paragraph)
            let cat_name = if !app.categories.is_empty() && *category_idx < app.categories.len() {
                app.categories[*category_idx].name.clone()
            } else {
                "No Category".to_string()
            };

            // Display category color if available
            let mut cat_style = Style::default().fg(Color::White);
            if !app.categories.is_empty()
                && *category_idx < app.categories.len()
                && let Some(color) = app.category_map.get(&app.categories[*category_idx].id)
            {
                cat_style = cat_style.fg(*color);
            }

            let cat_selector = Paragraph::new(format!(" < {} > ", cat_name))
                .style(cat_style)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Category")
                        .border_style(if *focus == TaskFocus::Category {
                            active_style
                        } else {
                            inactive_style
                        }),
                );

            f.render_widget(t_widget.as_ref(), layout[0]);
            f.render_widget(d_widget.as_ref(), layout[1]);
            f.render_widget(date_widget.as_ref(), layout[2]);
            f.render_widget(cat_selector, layout[3]);

            let help = Paragraph::new(
                "TAB: Next Field | Left/Right: Category | Ctrl+S: Save | ESC: Cancel",
            )
            .style(Style::default().fg(Color::Gray));
            f.render_widget(help, layout[4]);
        }
        ActiveModal::Column { name, mode } => {
            let title = match mode {
                ModalType::ColumnEdit(_) => " Edit Column ",
                _ => " New Column ",
            };
            f.render_widget(block.title(title), area);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Length(1)])
                .split(area);

            let mut n_widget = name.clone();
            n_widget.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Name")
                    .border_style(Style::default().fg(Color::Yellow)),
            );

            f.render_widget(n_widget.as_ref(), layout[0]);
            f.render_widget(Paragraph::new("Ctrl+S: Save | ESC: Cancel"), layout[1]);
        }
        ActiveModal::Category {
            name, color, focus, ..
        } => {
            f.render_widget(block.title(" Category Details "), area);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(area);

            let active_style = Style::default().fg(Color::Yellow);
            let inactive_style = Style::default().fg(Color::White);

            let mut n_widget = name.clone();
            n_widget.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Name")
                    .border_style(if *focus == CategoryFocus::Name {
                        active_style
                    } else {
                        inactive_style
                    }),
            );

            let mut c_widget = color.clone();
            c_widget.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Color (Hex e.g. #FF0000)")
                    .border_style(if *focus == CategoryFocus::Color {
                        active_style
                    } else {
                        inactive_style
                    }),
            );

            f.render_widget(n_widget.as_ref(), layout[0]);
            f.render_widget(c_widget.as_ref(), layout[1]);
            f.render_widget(
                Paragraph::new("TAB: Next Field | Ctrl+S: Save | ESC: Cancel"),
                layout[2],
            );
        }
        ActiveModal::Board {
            name,
            icon,
            focus,
            mode,
        } => {
            let title = match mode {
                ModalType::BoardEdit(_) => " Edit Board ",
                _ => " New Board ",
            };
            f.render_widget(block.title(title), area);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(area);

            let active_style = Style::default().fg(Color::Yellow);
            let inactive_style = Style::default().fg(Color::White);

            let mut n_widget = name.clone();
            n_widget.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Board Name")
                    .border_style(if *focus == BoardFocus::Name {
                        active_style
                    } else {
                        inactive_style
                    }),
            );

            let mut i_widget = icon.clone();
            i_widget.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Icon")
                    .border_style(if *focus == BoardFocus::Icon {
                        active_style
                    } else {
                        inactive_style
                    }),
            );

            f.render_widget(n_widget.as_ref(), layout[0]);
            f.render_widget(i_widget.as_ref(), layout[1]);
            f.render_widget(
                Paragraph::new("TAB: Next Field | Ctrl+S: Save | ESC: Cancel"),
                layout[2],
            );
        }
    }
}
