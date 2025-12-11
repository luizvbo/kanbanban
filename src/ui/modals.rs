use super::centered_rect;
use crate::app::{ActiveModal, App, BoardFocus, CategoryFocus, ModalType, TaskFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use tui_textarea::TextArea;

pub fn render(f: &mut Frame, app: &App, modal: &ActiveModal) {
    let area = centered_rect(60, 50, f.area());
    f.render_widget(Clear, area);

    match modal {
        ActiveModal::Task {
            title,
            desc,
            date,
            category_idx,
            focus,
            is_editing,
            show_confirm,
            ..
        } => render_task_modal(
            f,
            app,
            area,
            title,
            desc,
            date,
            *category_idx,
            focus,
            *is_editing,
            *show_confirm,
        ),
        ActiveModal::Column { name, mode } => {
            let title = match mode {
                ModalType::ColumnEdit(_) => " Edit Column ",
                _ => " New Column ",
            };
            render_simple_modal(f, area, title, vec![("Name", name)], None);
        }
        ActiveModal::Category {
            name, color, focus, ..
        } => {
            let active_idx = match focus {
                CategoryFocus::Name => 0,
                CategoryFocus::Color => 1,
            };
            render_simple_modal(
                f,
                area,
                " Category Details ",
                vec![("Name", name), ("Color (Hex e.g. #FF0000)", color)],
                Some(active_idx),
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
            let active_idx = match focus {
                BoardFocus::Name => 0,
                BoardFocus::Icon => 1,
            };
            render_simple_modal(
                f,
                area,
                title,
                vec![("Board Name", name), ("Icon", icon)],
                Some(active_idx),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_task_modal(
    f: &mut Frame,
    app: &App,
    area: Rect,
    title: &TextArea,
    desc: &TextArea,
    date: &TextArea,
    category_idx: usize,
    focus: &TaskFocus,
    is_editing: bool,
    show_confirm: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray))
        .title(" Task Details ");
    f.render_widget(block, area);

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

    let inactive_style = Style::default().fg(Color::White);
    let nav_focus_style = Style::default().fg(Color::Cyan);
    let edit_focus_style = Style::default().fg(Color::Green);

    let get_style = |target_focus: TaskFocus| {
        if *focus == target_focus {
            if is_editing {
                edit_focus_style
            } else {
                nav_focus_style
            }
        } else {
            inactive_style
        }
    };

    // Title Input
    let mut t_widget = title.clone();
    t_widget.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title("Title")
            .border_style(get_style(TaskFocus::Title)),
    );
    if *focus != TaskFocus::Title || !is_editing {
        t_widget.set_cursor_line_style(Style::default());
        t_widget.set_cursor_style(Style::default());
    }

    // Description Input
    let mut d_widget = desc.clone();
    d_widget.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title("Description")
            .border_style(get_style(TaskFocus::Description)),
    );
    if *focus != TaskFocus::Description || !is_editing {
        d_widget.set_cursor_line_style(Style::default());
        d_widget.set_cursor_style(Style::default());
    }

    // Date Input
    let mut date_widget = date.clone();
    date_widget.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title("Due Date (Ctrl+D to Pick)")
            .border_style(get_style(TaskFocus::Date)),
    );
    if *focus != TaskFocus::Date || !is_editing {
        date_widget.set_cursor_line_style(Style::default());
        date_widget.set_cursor_style(Style::default());
    }

    // Category Selector
    let cat_name = if !app.categories.is_empty() && category_idx < app.categories.len() {
        app.categories[category_idx].name.clone()
    } else {
        "No Category".to_string()
    };

    let mut cat_style = Style::default().fg(Color::White);
    if !app.categories.is_empty()
        && category_idx < app.categories.len()
        && let Some(color) = app.category_map.get(&app.categories[category_idx].id)
    {
        cat_style = cat_style.fg(*color);
    }

    let cat_selector = Paragraph::new(format!(" < {cat_name} > "))
        .style(cat_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Category")
                .border_style(get_style(TaskFocus::Category)),
        );

    f.render_widget(&t_widget, layout[0]);
    f.render_widget(&d_widget, layout[1]);
    f.render_widget(&date_widget, layout[2]);
    f.render_widget(cat_selector, layout[3]);

    let help_text = if is_editing {
        "Typing Mode | Enter: Done | Esc: Stop Editing"
    } else {
        "Nav Mode | j/k: Move | Enter: Edit Field | Esc: Close/Save"
    };
    f.render_widget(
        Paragraph::new(help_text).style(Style::default().fg(Color::Gray)),
        layout[4],
    );

    if show_confirm {
        let confirm_area = centered_rect(40, 20, f.area());
        f.render_widget(Clear, confirm_area);
        let confirm_block = Block::default()
            .borders(Borders::ALL)
            .title(" Unsaved Changes ")
            .style(Style::default().bg(Color::Red).fg(Color::White));

        let confirm_text = Paragraph::new(
            "You have unsaved changes.\n\nSave before closing?\n\n(Y)es / (N)o / (Esc) Cancel",
        )
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(confirm_block);

        f.render_widget(confirm_text, confirm_area);
    }
}

fn render_simple_modal(
    f: &mut Frame,
    area: Rect,
    title: &str,
    fields: Vec<(&str, &TextArea)>,
    active_idx: Option<usize>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray))
        .title(title);
    f.render_widget(block, area);

    let mut constraints = Vec::new();
    for _ in 0..fields.len() {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Length(1)); // Help

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(area);

    let active_style = Style::default().fg(Color::Yellow);
    let inactive_style = Style::default().fg(Color::White);

    for (i, (label, text_area)) in fields.iter().enumerate() {
        let mut widget = (*text_area).clone();
        let border_style = if let Some(idx) = active_idx {
            if idx == i {
                active_style
            } else {
                inactive_style
            }
        } else {
            // If no active index (e.g. single field), default to active style or yellow
            Style::default().fg(Color::Yellow)
        };

        widget.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(*label)
                .border_style(border_style),
        );
        f.render_widget(&widget, layout[i]);
    }

    let help_idx = fields.len();
    let help_text = if fields.len() > 1 {
        "TAB: Next Field | Ctrl+S/Enter: Save | ESC: Cancel"
    } else {
        "Ctrl+S/Enter: Save | ESC: Cancel"
    };
    f.render_widget(Paragraph::new(help_text), layout[help_idx]);
}
