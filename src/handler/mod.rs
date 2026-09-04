pub mod modes;

use crate::app::{App, InputMode};
use crate::handler::modes::{
    handle_column_delete, handle_column_project_rename, handle_delete_confirmation, handle_editing,
    handle_exiting_modal, handle_filter, handle_help, handle_normal, handle_selector,
    handle_view_card,
};
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};

pub fn handle_events(app: &mut App) -> Result<()> {
    if event::poll(std::time::Duration::from_millis(50))? {
        let event = event::read()?;
        process_event(app, event)?;
    }
    Ok(())
}

pub fn process_event(app: &mut App, event: Event) -> Result<()> {
    if let Event::Key(key) = event
        && key.kind == KeyEventKind::Press
    {
        match app.mode {
            InputMode::Normal => handle_normal(app, key),
            InputMode::Filter => handle_filter(app, key),
            InputMode::ViewCard => handle_view_card(app, key),
            InputMode::Help => handle_help(app, key),
            InputMode::Selector => handle_selector(app, key),
            InputMode::ExitingModal => handle_exiting_modal(app, key),
            InputMode::DeleteConfirmation => handle_delete_confirmation(app, key),
            InputMode::Editing => handle_editing(app, key),
            InputMode::ProjectRename | InputMode::ColumnRename | InputMode::ColumnNew => {
                handle_column_project_rename(app, key)
            }
            InputMode::ColumnDelete => handle_column_delete(app, key),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        app::state::EditField,
        domain::kanban::{Card, Column, Project},
    };

    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn create_app() -> App {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("kbb.yaml");
        std::mem::forget(tmp); // keep the temp directory alive for the test
        let mut app = App::new(path).unwrap();

        // Force clean state regardless of file load result
        app.data.projects.clear();
        app.data.projects.push(Project {
            name: "Test Project".into(),
            columns: vec![Column {
                title: "Col 1".into(),
                cards: vec![],
                scroll_offset: 0,
            }],
        });

        // Add a card
        app.data.projects[0].columns[0].cards.push(Card {
            title: "Test".into(),
            description: "".into(),
            category: None,
            tags: vec![],
            due_date: None,
        });

        // Reset indices
        app.current_project_idx = 0;
        app.current_col_idx = 0;
        app.current_card_idx = 0;

        app
    }

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn key_event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_all_normal_mode_keys() {
        let mut app = create_app();

        // Navigation
        process_event(&mut app, key(KeyCode::Char('l'))).unwrap(); // Next col
        process_event(&mut app, key(KeyCode::Char('h'))).unwrap(); // Prev col
        process_event(&mut app, key(KeyCode::Char('j'))).unwrap(); // Next card
        process_event(&mut app, key(KeyCode::Char('k'))).unwrap(); // Prev card

        // Movements
        process_event(&mut app, key(KeyCode::Char('J'))).unwrap(); // Move down
        process_event(&mut app, key(KeyCode::Char('K'))).unwrap(); // Move up
        process_event(&mut app, key(KeyCode::Char('H'))).unwrap(); // Move left
        process_event(&mut app, key(KeyCode::Char('L'))).unwrap(); // Move right

        // Column Ops
        process_event(&mut app, key(KeyCode::Char('>'))).unwrap(); // Move col right
        process_event(&mut app, key(KeyCode::Char('<'))).unwrap(); // Move col left

        // Modals triggers
        process_event(&mut app, key(KeyCode::Char('?'))).unwrap(); // Help
        assert_eq!(app.mode, InputMode::Help);
        process_event(&mut app, key(KeyCode::Esc)).unwrap(); // Close Help

        process_event(&mut app, key(KeyCode::Char('/'))).unwrap(); // Filter
        assert_eq!(app.mode, InputMode::Filter);
        process_event(&mut app, key(KeyCode::Esc)).unwrap(); // Close Filter

        process_event(&mut app, key(KeyCode::Char('v'))).unwrap(); // View
        assert_eq!(app.mode, InputMode::ViewCard);
        process_event(&mut app, key(KeyCode::Esc)).unwrap(); // Close View

        process_event(&mut app, key(KeyCode::Char('R'))).unwrap(); // Rename Project
        assert_eq!(app.mode, InputMode::ProjectRename);
        process_event(&mut app, key(KeyCode::Esc)).unwrap();

        process_event(&mut app, key(KeyCode::Char('r'))).unwrap(); // Rename Col
        assert_eq!(app.mode, InputMode::ColumnRename);
        process_event(&mut app, key(KeyCode::Esc)).unwrap();

        process_event(&mut app, key(KeyCode::Char('c'))).unwrap(); // New Col
        assert_eq!(app.mode, InputMode::ColumnNew);
        process_event(&mut app, key(KeyCode::Esc)).unwrap();

        process_event(&mut app, key(KeyCode::Char('D'))).unwrap(); // Delete Col
        assert_eq!(app.mode, InputMode::ColumnDelete);
        process_event(&mut app, key(KeyCode::Esc)).unwrap();
    }

    #[test]
    fn test_filter_mode_interaction() {
        let mut app = create_app();
        app.mode = InputMode::Filter;

        process_event(&mut app, key(KeyCode::Char('a'))).unwrap();
        assert_eq!(app.filter_query, "a");

        process_event(&mut app, key(KeyCode::Backspace)).unwrap();
        assert_eq!(app.filter_query, "");

        process_event(&mut app, key(KeyCode::Enter)).unwrap();
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn test_tag_selection_interaction() {
        let mut app = create_app();
        app.start_new_card();
        app.open_selector(crate::app::state::SelectorType::Tag); // Sets mode to TagSelection

        // Navigation
        process_event(&mut app, key(KeyCode::Down)).unwrap();
        process_event(&mut app, key(KeyCode::Up)).unwrap();

        // Input
        process_event(&mut app, key(KeyCode::Char('b'))).unwrap();
        assert_eq!(app.selector_state.as_ref().unwrap().search_query, "b");

        process_event(&mut app, key(KeyCode::Backspace)).unwrap();
        assert!(app.selector_state.as_ref().unwrap().search_query.is_empty());

        // Toggle/Confirm
        process_event(&mut app, key(KeyCode::Enter)).unwrap(); // Toggle
        process_event(&mut app, key(KeyCode::Esc)).unwrap(); // Confirm & Close

        assert_eq!(app.mode, InputMode::Editing);
    }

    #[test]

    fn test_editing_mode_interaction() {
        let mut app = create_app();
        app.start_new_card();

        // FIX: Press Enter to switch from Navigation to Insert Mode
        process_event(&mut app, key(KeyCode::Enter)).unwrap();

        // Typing
        process_event(&mut app, key(KeyCode::Char('A'))).unwrap();
        assert_eq!(app.edit_state.as_ref().unwrap().title, "A");

        // Exit Insert Mode
        process_event(&mut app, key(KeyCode::Esc)).unwrap();

        // Navigation
        process_event(&mut app, key(KeyCode::Tab)).unwrap(); // Cycle
        process_event(&mut app, key(KeyCode::BackTab)).unwrap(); // Cycle back

        // Exit Logic
        process_event(&mut app, key(KeyCode::Esc)).unwrap();
        // Since it's new and not empty (has 'A'), it should ask for confirmation
        assert_eq!(app.mode, InputMode::ExitingModal);

        // Confirm Discard
        process_event(&mut app, key(KeyCode::Char('n'))).unwrap();
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn test_column_delete_interaction() {
        let mut app = create_app();
        app.mode = InputMode::ColumnDelete;

        // Cancel
        process_event(&mut app, key(KeyCode::Char('n'))).unwrap();
        assert_eq!(app.mode, InputMode::Normal);

        // Confirm
        app.mode = InputMode::ColumnDelete;
        process_event(&mut app, key(KeyCode::Char('y'))).unwrap();
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn test_view_mode_interaction() {
        let mut app = create_app();
        app.mode = InputMode::ViewCard;

        process_event(&mut app, key(KeyCode::Char('j'))).unwrap(); // Scroll down
        assert_eq!(app.view_scroll_y, 1);

        process_event(&mut app, key(KeyCode::Char('k'))).unwrap(); // Scroll up
        assert_eq!(app.view_scroll_y, 0);

        process_event(&mut app, key(KeyCode::Char('q'))).unwrap(); // Quit view
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]

    fn test_prompt_input_handling() {
        let mut app = create_app();

        // Enter Rename Mode
        process_event(&mut app, key(KeyCode::Char('R'))).unwrap();
        assert_eq!(app.mode, InputMode::ProjectRename);

        // It pre-fills "Test Project" (from create_app)
        let initial_len = app.prompt_input.len();

        // Test Backspace
        process_event(&mut app, key(KeyCode::Backspace)).unwrap();
        assert_eq!(app.prompt_input.len(), initial_len - 1);

        // Test Typing
        process_event(&mut app, key(KeyCode::Char('X'))).unwrap();
        assert!(app.prompt_input.ends_with('X'));

        // Test Esc (Cancel)
        process_event(&mut app, key(KeyCode::Esc)).unwrap();
        assert_eq!(app.mode, InputMode::Normal);

        // FIX: Name should match what create_app sets
        assert_eq!(app.data.projects[0].name, "Test Project");
    }

    #[test]
    fn test_description_esc_behavior() {
        let mut app = create_app();
        app.start_new_card();

        // Navigate to Description
        app.edit_state.as_mut().unwrap().focused_field = EditField::Description;

        // 1. Enter Edit Mode
        process_event(&mut app, key(KeyCode::Enter)).unwrap();
        assert!(app.edit_state.as_ref().unwrap().description_edit_mode);

        // 2. Press ESC - Should ONLY exit edit mode, NOT close modal
        process_event(&mut app, key(KeyCode::Esc)).unwrap();
        assert!(!app.edit_state.as_ref().unwrap().description_edit_mode); // Mode off
        assert_eq!(app.mode, InputMode::Editing); // Still in Editing modal

        // 3. Press ESC again - Should trigger Exit Confirmation (since card is not empty)
        // (Note: start_new_card creates empty, but let's dirty it)
        app.edit_state.as_mut().unwrap().title = "Dirty".into();
        process_event(&mut app, key(KeyCode::Esc)).unwrap();
        assert_eq!(app.mode, InputMode::ExitingModal);
    }

    #[test]

    fn test_vim_edit_mode_transitions() {
        let mut app = create_app();
        app.start_new_card();
        // Initial State: Mode=Editing, Field=Title, is_editing_field=false

        // 1. Try typing 'j' - Should NOT type, should stay in nav
        handle_editing(&mut app, key_event(KeyCode::Char('j')));
        assert_eq!(app.edit_state.as_ref().unwrap().title, "");
        assert!(!app.edit_state.as_ref().unwrap().is_editing_field);

        // 2. Press Enter - Enter Insert Mode
        // Explicitly ensure we are on a text field
        app.edit_state.as_mut().unwrap().focused_field = EditField::Title;
        handle_editing(&mut app, key_event(KeyCode::Enter));

        // ASSERTION: Check the flag directly
        assert!(
            app.edit_state.as_ref().unwrap().is_editing_field,
            "Enter should enable insert mode"
        );

        // 3. Type 'j' - Should type now
        handle_editing(&mut app, key_event(KeyCode::Char('j')));
        assert_eq!(app.edit_state.as_ref().unwrap().title, "j");

        // 4. Press Esc - Exit Insert Mode
        handle_editing(&mut app, key_event(KeyCode::Esc));
        assert!(
            !app.edit_state.as_ref().unwrap().is_editing_field,
            "Esc should disable insert mode"
        );
    }
}
