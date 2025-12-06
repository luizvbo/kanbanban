mod app;
mod config;
mod db;
mod models;
mod ui;

use app::{ActiveModal, App, InputMode, OverviewTab, TaskFocus, ViewMode};
use config::load_config;
use db::Database;

use anyhow::Result;
use ratatui::crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEvent,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, time::Duration};

// Trait to abstract event handling for testing
pub trait EventHandler {
    fn poll(&self, timeout: Duration) -> Result<bool>;
    fn read(&self) -> Result<Event>;
}

struct CrosstermEventHandler;

impl EventHandler for CrosstermEventHandler {
    fn poll(&self, timeout: Duration) -> Result<bool> {
        Ok(event::poll(timeout)?)
    }
    fn read(&self) -> Result<Event> {
        Ok(event::read()?)
    }
}

fn main() -> Result<()> {
    let (_config, db_path) = load_config()?;
    let db = Database::new(db_path)?;
    let mut app = App::new(db)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let event_handler = CrosstermEventHandler;
    let res = run_app(&mut terminal, &mut app, &event_handler);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    events: &impl EventHandler,
) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|f| {
            crate::ui::ui(f, app);
            // Render Date Picker on top if active
            if app.date_picker.is_some() {
                crate::ui::datepicker::render(f, app);
            }
        })?;

        if events.poll(Duration::from_millis(250))? {
            match events.read()? {
                Event::Key(key) => handle_key(app, key)?,
                Event::Mouse(mouse) => handle_mouse(app, mouse)?,
                _ => {}
            }
        }
    }
    Ok(())
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) -> Result<()> {
    // If date picker is open, ignore mouse for now
    if app.date_picker.is_some() {
        return Ok(());
    }

    match mouse.kind {
        MouseEventKind::Down(_) => {
            app.handle_mouse_down(mouse.column, mouse.row);
        }
        MouseEventKind::Drag(_) => {
            app.handle_mouse_drag(mouse.column, mouse.row);
        }
        MouseEventKind::Up(_) => {
            app.handle_mouse_up(mouse.column, mouse.row)?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
    // 1. Handle Date Picker Input Priority
    if app.date_picker.is_some() {
        match key.code {
            KeyCode::Esc => app.date_picker = None,
            KeyCode::Enter => app.select_date(),
            KeyCode::Left => app.date_picker_nav(-1, 0),
            KeyCode::Right => app.date_picker_nav(1, 0),
            KeyCode::Up => app.date_picker_nav(0, -1),
            KeyCode::Down => app.date_picker_nav(0, 1),
            _ => {}
        }
        return Ok(());
    }

    match app.input_mode {
        InputMode::Normal => {
            match key.code {
                KeyCode::Char('q') => app.quit(),

                // View Switching
                KeyCode::Char('v') => app.view_mode = ViewMode::Overview,
                KeyCode::Char('b') => app.view_mode = ViewMode::Board,
                KeyCode::Char('B') => {
                    app.view_mode = ViewMode::BoardSelector;
                    app.board_selector_index = app.active_board_index;
                }
                KeyCode::Char('s') => app.view_mode = ViewMode::Settings,
                KeyCode::Char('C') => app.view_mode = ViewMode::CategoryManager,

                // Context Dependent Keys
                _ => match app.view_mode {
                    ViewMode::Board => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                        KeyCode::Char('h') | KeyCode::Left => app.move_left(),
                        KeyCode::Char('l') | KeyCode::Right => app.move_right(),
                        KeyCode::Char('L') => {
                            let _ = app.move_task_right();
                        }
                        KeyCode::Char('H') => {
                            let _ = app.move_task_left();
                        }
                        KeyCode::Char('n') => app.open_new_task_modal(),
                        KeyCode::Char('e') => app.open_edit_task_modal(),
                        KeyCode::Char('c') => app.open_new_column_modal(),
                        KeyCode::Char('d') => {
                            let _ = app.delete_current_task();
                        }
                        KeyCode::Char('D') => {
                            let _ = app.delete_current_column();
                        }
                        _ => {}
                    },
                    ViewMode::BoardSelector => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            let _ = app.next_board();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            let _ = app.prev_board();
                        }
                        KeyCode::Enter => {
                            app.select_highlighted_board()?;
                        }
                        KeyCode::Char('n') => {
                            app.open_new_board_modal();
                        }
                        KeyCode::Char('e') => {
                            app.open_edit_board_modal();
                        }
                        KeyCode::Char('d') => {
                            let _ = app.delete_highlighted_board();
                        }
                        KeyCode::Esc => app.view_mode = ViewMode::Board,
                        _ => {}
                    },
                    ViewMode::Settings => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            if app.settings_state.selected_column_idx
                                < app.columns.len().saturating_sub(1)
                            {
                                app.settings_state.selected_column_idx += 1;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if app.settings_state.selected_column_idx > 0 {
                                app.settings_state.selected_column_idx -= 1;
                            }
                        }
                        KeyCode::Char(' ') => {
                            app.toggle_column_visibility()?;
                        }

                        KeyCode::Char('r') => {
                            app.open_edit_column_modal();
                        }
                        KeyCode::Char('K') => {
                            app.move_column_up()?;
                        }
                        KeyCode::Char('J') => {
                            app.move_column_down()?;
                        }
                        KeyCode::Char('1') => {
                            app.set_column_as_status("start")?;
                        }
                        KeyCode::Char('2') => {
                            app.set_column_as_status("finish")?;
                        }
                        KeyCode::Char('3') => {
                            app.set_column_as_status("reset")?;
                        }
                        _ => {}
                    },
                    ViewMode::Overview => match key.code {
                        KeyCode::Char('1') => app.overview_state.tab = OverviewTab::Charts,
                        KeyCode::Char('2') => app.overview_state.tab = OverviewTab::Logs,
                        KeyCode::Char('j') | KeyCode::Down => {
                            if let OverviewTab::Logs = app.overview_state.tab {
                                app.scroll_logs_down();
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if let OverviewTab::Logs = app.overview_state.tab {
                                app.scroll_logs_up();
                            }
                        }
                        _ => {}
                    },
                    ViewMode::CategoryManager => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            if app.category_selector_index < app.categories.len().saturating_sub(1)
                            {
                                app.category_selector_index += 1;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if app.category_selector_index > 0 {
                                app.category_selector_index -= 1;
                            }
                        }
                        KeyCode::Char('n') => app.open_new_category_modal(),
                        KeyCode::Char('e') => app.open_edit_category_modal(),
                        KeyCode::Char('d') => {
                            let _ = app.delete_current_category();
                        }
                        KeyCode::Esc => app.view_mode = ViewMode::Board,
                        _ => {}
                    },
                },
            }
        }
        InputMode::Editing => match key.code {
            KeyCode::Esc => app.close_modal(),
            KeyCode::Tab => app.cycle_modal_focus(),
            KeyCode::Char('s') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                app.save_modal_data()?;
            }
            // Special handling for Category Selector in Task Modal
            KeyCode::Left => {
                if let Some(ActiveModal::Task {
                    focus: TaskFocus::Category,
                    category_idx,
                    ..
                }) = &mut app.active_modal
                {
                    if *category_idx > 0 {
                        *category_idx -= 1;
                    }
                } else {
                    app.on_modal_input(key);
                }
            }
            KeyCode::Right => {
                if let Some(ActiveModal::Task {
                    focus: TaskFocus::Category,
                    category_idx,
                    ..
                }) = &mut app.active_modal
                {
                    if *category_idx < app.categories.len().saturating_sub(1) {
                        *category_idx += 1;
                    }
                } else {
                    app.on_modal_input(key);
                }
            }
            // Add Date Picker Trigger
            KeyCode::Char('d') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                let should_open = matches!(
                    &app.active_modal,
                    Some(ActiveModal::Task {
                        focus: TaskFocus::Date,
                        ..
                    })
                );

                if should_open {
                    app.open_date_picker();
                }
            }
            _ => {
                app.on_modal_input(key);
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use tempfile::NamedTempFile;

    struct MockEventHandler {
        events: RefCell<VecDeque<Event>>,
    }

    impl MockEventHandler {
        fn new(events: Vec<Event>) -> Self {
            Self {
                events: RefCell::new(VecDeque::from(events)),
            }
        }
    }

    impl EventHandler for MockEventHandler {
        fn poll(&self, _timeout: Duration) -> Result<bool> {
            Ok(!self.events.borrow().is_empty())
        }
        fn read(&self) -> Result<Event> {
            self.events
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| anyhow::anyhow!("No more events"))
        }
    }

    fn setup_app() -> App<'static> {
        let file = NamedTempFile::new().unwrap();
        let db = Database::new(file.path()).unwrap();
        App::new(db).unwrap()
    }

    #[test]
    fn test_run_app_quit() {
        let mut app = setup_app();
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let events = vec![Event::Key(KeyEvent::from(KeyCode::Char('q')))];
        let handler = MockEventHandler::new(events);

        run_app(&mut terminal, &mut app, &handler).unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn test_run_app_switch_views() {
        let mut app = setup_app();
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let events = vec![
            Event::Key(KeyEvent::from(KeyCode::Char('v'))), // Overview
            Event::Key(KeyEvent::from(KeyCode::Char('q'))),
        ];
        let handler = MockEventHandler::new(events);

        run_app(&mut terminal, &mut app, &handler).unwrap();
        assert_eq!(app.view_mode, ViewMode::Overview);
    }

    #[test]
    fn test_handle_key_board_nav() {
        let mut app = setup_app();
        // Add tasks
        let bid = app.boards[0].id;
        let cid = app.columns[0].id;
        app.db
            .create_task(bid, cid, "T1".into(), "".into(), None, None)
            .unwrap();
        app.db
            .create_task(bid, cid, "T2".into(), "".into(), None, None)
            .unwrap();
        app.refresh_data().unwrap();

        handle_key(&mut app, KeyEvent::from(KeyCode::Char('j'))).unwrap();
        assert_eq!(app.selected_task_index, 1);

        handle_key(&mut app, KeyEvent::from(KeyCode::Char('k'))).unwrap();
        assert_eq!(app.selected_task_index, 0);

        handle_key(&mut app, KeyEvent::from(KeyCode::Char('l'))).unwrap();
        assert_eq!(app.selected_column_index, 1);

        handle_key(&mut app, KeyEvent::from(KeyCode::Char('h'))).unwrap();
        assert_eq!(app.selected_column_index, 0);
    }

    #[test]
    fn test_handle_key_modal_interaction() {
        let mut app = setup_app();

        // Open modal
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('n'))).unwrap();
        assert!(matches!(app.input_mode, InputMode::Editing));

        // Type 'A'
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('A'))).unwrap();

        // Save (Ctrl+S)
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('s'), event::KeyModifiers::CONTROL),
        )
        .unwrap();

        assert!(matches!(app.input_mode, InputMode::Normal));
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].inner.title, "A");
    }

    #[test]
    fn test_handle_key_board_selector() {
        let mut app = setup_app();
        app.db.create_board("B2".into(), "".into()).unwrap();
        app.refresh_data().unwrap();

        // Switch to selector
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('B'))).unwrap();
        assert_eq!(app.view_mode, ViewMode::BoardSelector);

        // Move down
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('j'))).unwrap();
        assert_eq!(app.board_selector_index, 1);

        // Select
        handle_key(&mut app, KeyEvent::from(KeyCode::Enter)).unwrap();
        assert_eq!(app.view_mode, ViewMode::Board);
        assert_eq!(app.active_board_index, 1);
    }

    #[test]
    fn test_handle_key_settings() {
        let mut app = setup_app();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('s'))).unwrap();
        assert_eq!(app.view_mode, ViewMode::Settings);

        // Move down
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('j'))).unwrap();
        assert_eq!(app.settings_state.selected_column_idx, 1);

        // Toggle visibility
        handle_key(&mut app, KeyEvent::from(KeyCode::Char(' '))).unwrap();
        assert!(!app.columns[1].visible);

        // Move up
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('K'))).unwrap();
        assert_eq!(app.columns[0].name, "Doing"); // Swapped
    }

    #[test]
    fn test_handle_key_overview() {
        let mut app = setup_app();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('v'))).unwrap();
        assert_eq!(app.view_mode, ViewMode::Overview);

        // Switch tabs
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('1'))).unwrap();
        assert!(matches!(app.overview_state.tab, OverviewTab::Charts));

        handle_key(&mut app, KeyEvent::from(KeyCode::Char('2'))).unwrap();
        assert!(matches!(app.overview_state.tab, OverviewTab::Logs));

        // Scroll
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('j'))).unwrap();
        assert_eq!(app.overview_state.log_state.selected(), Some(0));
    }

    #[test]
    fn test_handle_key_category_manager() {
        let mut app = setup_app();
        app.db.create_category("C1".into(), "".into()).unwrap();
        app.refresh_data().unwrap();

        handle_key(&mut app, KeyEvent::from(KeyCode::Char('C'))).unwrap();
        assert_eq!(app.view_mode, ViewMode::CategoryManager);

        // New category
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('n'))).unwrap();
        assert!(matches!(app.input_mode, InputMode::Editing));
        handle_key(&mut app, KeyEvent::from(KeyCode::Esc)).unwrap();

        // Delete
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('d'))).unwrap();
        assert_eq!(app.categories.len(), 0);
    }

    #[test]
    fn test_handle_key_task_modal_category_nav() {
        let mut app = setup_app();
        app.db.create_category("C1".into(), "".into()).unwrap();
        app.db.create_category("C2".into(), "".into()).unwrap();
        app.refresh_data().unwrap();

        app.open_new_task_modal();
        // Set focus to category
        if let Some(ActiveModal::Task { focus, .. }) = &mut app.active_modal {
            *focus = TaskFocus::Category;
        }

        handle_key(&mut app, KeyEvent::from(KeyCode::Right)).unwrap();
        if let Some(ActiveModal::Task { category_idx, .. }) = &app.active_modal {
            assert_eq!(*category_idx, 1);
        }

        handle_key(&mut app, KeyEvent::from(KeyCode::Left)).unwrap();
        if let Some(ActiveModal::Task { category_idx, .. }) = &app.active_modal {
            assert_eq!(*category_idx, 0);
        }
    }
}
