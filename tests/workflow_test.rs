use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use kanbanban::app::{App, InputMode};
use kanbanban::events::process_event;
use tempfile::NamedTempFile;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn test_full_user_workflow() {
    // 1. Setup
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_path_buf();
    let mut app = App::new(path.clone()).unwrap();

    // 2. Create a New Column
    process_event(&mut app, key(KeyCode::Char('c'))).unwrap();
    assert_eq!(app.mode, InputMode::ColumnNew);

    // Type "Work"
    process_event(&mut app, key(KeyCode::Char('W'))).unwrap();
    process_event(&mut app, key(KeyCode::Char('o'))).unwrap();
    process_event(&mut app, key(KeyCode::Char('r'))).unwrap();
    process_event(&mut app, key(KeyCode::Char('k'))).unwrap();
    process_event(&mut app, key(KeyCode::Enter)).unwrap();

    // Verify Column Created
    let last_col_idx = app.data.projects[0].columns.len() - 1;
    assert_eq!(app.data.projects[0].columns[last_col_idx].title, "Work");

    // 3. Add a Card to "Work"
    process_event(&mut app, key(KeyCode::Char('n'))).unwrap(); // New Card
    assert_eq!(app.mode, InputMode::Editing);

    // Type Title "Task 1"
    process_event(&mut app, key(KeyCode::Char('T'))).unwrap();
    process_event(&mut app, key(KeyCode::Char('a'))).unwrap();
    process_event(&mut app, key(KeyCode::Char('s'))).unwrap();
    process_event(&mut app, key(KeyCode::Char('k'))).unwrap();
    process_event(&mut app, key(KeyCode::Char(' '))).unwrap();
    process_event(&mut app, key(KeyCode::Char('1'))).unwrap();

    // Save
    process_event(&mut app, key(KeyCode::Enter)).unwrap();

    assert_eq!(app.mode, InputMode::Normal);
    assert_eq!(app.data.projects[0].columns[last_col_idx].cards.len(), 1);
    assert_eq!(
        app.data.projects[0].columns[last_col_idx].cards[0].title,
        "Task 1"
    );

    // 4. Rename the Board
    process_event(&mut app, key(KeyCode::Char('R'))).unwrap();
    // Prompt is pre-filled with "Default Board". We append "My".
    process_event(&mut app, key(KeyCode::Char('M'))).unwrap();
    process_event(&mut app, key(KeyCode::Char('y'))).unwrap();
    process_event(&mut app, key(KeyCode::Enter)).unwrap();

    // FIX: Expect appended name
    assert_eq!(app.data.projects[0].name, "Default BoardMy");

    // 5. Filter
    process_event(&mut app, key(KeyCode::Char('/'))).unwrap();
    process_event(&mut app, key(KeyCode::Char('T'))).unwrap();
    process_event(&mut app, key(KeyCode::Char('a'))).unwrap();
    assert_eq!(app.filter_query, "Ta");
    process_event(&mut app, key(KeyCode::Esc)).unwrap(); // Clear filter
    assert_eq!(app.filter_query, "");

    // 6. Delete the Card
    process_event(&mut app, key(KeyCode::Char('d'))).unwrap();
    process_event(&mut app, key(KeyCode::Char('y'))).unwrap();
    assert_eq!(app.data.projects[0].columns[last_col_idx].cards.len(), 0);

    // 7. Verify Persistence (Reload from disk)
    let loaded_app = App::new(path).unwrap();
    // FIX: Expect appended name here as well
    assert_eq!(loaded_app.data.projects[0].name, "Default BoardMy");
    assert_eq!(
        loaded_app.data.projects[0].columns.last().unwrap().title,
        "Work"
    );
}
