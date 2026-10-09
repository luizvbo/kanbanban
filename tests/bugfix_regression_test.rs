//! Regression tests for the comprehensive audit.
//! Each test maps to a documented BUG-ID from the audit report and encodes
//! the expected (fixed) behavior.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use kanbanban::{
    app::state::{EditField, SelectorType},
    app::{App, InputMode},
    domain::kanban::{Card, Column, KanbanData, Project, Tag},
    handler::process_event,
    ui::board::get_card_height,
    ui::widgets::parse_markdown,
};
use tempfile::tempdir;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn key_mod(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn card(title: &str) -> Card {
    Card {
        title: title.to_string(),
        description: String::new(),
        category: None,
        tags: vec![],
        due_date: None,
    }
}

fn create_app() -> (App, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("kbb.yaml");
    let mut app = App::new(path).unwrap();
    app.data.projects.clear();
    app.data.projects.push(Project {
        name: "Test".into(),
        columns: vec![
            Column {
                title: "Col 1".into(),
                cards: vec![],
                scroll_offset: 0,
            },
            Column {
                title: "Col 2".into(),
                cards: vec![],
                scroll_offset: 0,
            },
        ],
    });
    app.current_project_idx = 0;
    app.current_col_idx = 0;
    app.current_card_idx = 0;
    (app, dir)
}

// ---------- BUG-01: empty `projects` in YAML must not panic ----------
#[test]
fn test_load_empty_projects_is_backfilled() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("kbb.yaml");
    std::fs::write(&path, "projects: []\n").unwrap();

    let app = App::new(path).expect("load must succeed");
    assert!(!app.data.projects.is_empty(), "projects must be backfilled");
}

// ---------- BUG-02: no columns -> navigation/actions must not panic ----------
#[test]
fn test_no_columns_operations_do_not_panic() {
    let (mut app, _dir) = create_app();
    app.data.projects[0].columns.clear();
    app.current_col_idx = 0;
    app.current_card_idx = 0;

    // None of these may panic or open a dead-end modal
    app.next_card();
    app.prev_card();
    app.next_column();
    app.prev_column();
    app.trigger_delete();
    app.confirm_delete();
    app.open_detail_view();
    app.start_edit_card();
    app.save_edit();
    app.move_card_up();
    app.move_card_down();
    app.move_card_left();
    app.move_card_right();
    app.trigger_delete_column();
    app.confirm_delete_column();
    assert_eq!(app.mode, InputMode::Normal);
}

// ---------- BUG-03: multi-byte chars + cursor up/down must keep char boundary ----------
#[test]
fn test_cursor_up_over_multibyte_stays_on_boundary() {
    let (mut app, _dir) = create_app();
    app.start_new_card();
    {
        let s = app.edit_state.as_mut().unwrap();
        s.focused_field = EditField::Description;
        s.description_edit_mode = true;
        s.is_editing_field = true;
        // '€' is 3 bytes -> description = [€][\n][a][b], len 6
        s.description = "€\nab".into();
        s.cursor_position = s.description.len();
    }
    app.move_cursor_up();
    let pos = app.edit_state.as_ref().unwrap().cursor_position;
    assert!(
        app.edit_state
            .as_ref()
            .unwrap()
            .description
            .is_char_boundary(pos),
        "cursor_position {} must be a char boundary",
        pos
    );
    // The next typed character must not panic (insert at non-boundary)
    app.edit_input_char('x');
}

#[test]
fn test_cursor_down_over_multibyte_stays_on_boundary() {
    let (mut app, _dir) = create_app();
    app.start_new_card();
    {
        let s = app.edit_state.as_mut().unwrap();
        s.focused_field = EditField::Description;
        s.description_edit_mode = true;
        s.description = "ab\n€x".into(); // '€x' line: € is 3 bytes
        s.cursor_position = 1; // column 1 on first line
    }
    app.move_cursor_down();
    let pos = app.edit_state.as_ref().unwrap().cursor_position;
    assert!(
        app.edit_state
            .as_ref()
            .unwrap()
            .description
            .is_char_boundary(pos),
        "cursor_position {} must be a char boundary",
        pos
    );
    app.edit_input_char('y'); // must not panic
}

// ---------- BUG-04: view_scroll_y overflow ----------
#[test]
fn test_view_scroll_does_not_overflow() {
    let (mut app, _dir) = create_app();
    app.data.projects[0].columns[0].cards.push(card("c"));
    app.open_detail_view();
    assert_eq!(app.mode, InputMode::ViewCard);

    process_event(&mut app, key(KeyCode::Char('G'))).unwrap(); // -> u16::MAX
    process_event(&mut app, key(KeyCode::Char('j'))).unwrap(); // must not panic
    assert!(app.view_scroll_y == u16::MAX);
}

// ---------- BUG-05: filter vs selection desync ----------
#[test]
fn test_filter_selection_stays_on_visible_card() {
    let (mut app, _dir) = create_app();
    let col = &mut app.data.projects[0].columns[0];
    col.cards.push(card("Alpha"));
    col.cards.push(card("Beta"));

    app.filter_query = "beta".into();
    app.current_card_idx = 0; // currently on hidden "Alpha"

    app.stop_filter(); // Enter: apply filter
    assert_eq!(
        app.current_card_idx, 1,
        "selection must snap to first visible card"
    );

    app.prev_card(); // Alpha is filtered out -> cannot be selected
    assert_eq!(app.current_card_idx, 1);
    app.next_card(); // nothing after Beta
    assert_eq!(app.current_card_idx, 1);

    // Deleting must remove the visible card, not a hidden one
    app.trigger_delete();
    assert_eq!(app.mode, InputMode::DeleteConfirmation);
    app.confirm_delete();
    let remaining: Vec<&str> = app.data.projects[0].columns[0]
        .cards
        .iter()
        .map(|c| c.title.as_str())
        .collect();
    assert_eq!(remaining, vec!["Alpha"]);
}

#[test]
fn test_filter_no_match_blocks_card_ops() {
    let (mut app, _dir) = create_app();
    app.data.projects[0].columns[0].cards.push(card("Alpha"));

    app.filter_query = "zzz".into(); // matches nothing
    app.stop_filter();

    app.trigger_delete(); // must not offer to delete an invisible card
    assert_eq!(app.mode, InputMode::Normal);

    app.open_detail_view(); // must not open an invisible card
    assert_eq!(app.mode, InputMode::Normal);
}

// ---------- BUG-08: save must not leave temp artifacts ----------
#[test]
fn test_save_is_atomic_and_leaves_no_tmp() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("kbb.yaml");

    let mut data = KanbanData::default();
    data.projects[0].name = "Atomic".into();
    data.save(&path).unwrap();

    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries.len(), 1, "no stray temp files: {:?}", entries);

    let loaded = KanbanData::load(&path).unwrap();
    assert_eq!(loaded.projects[0].name, "Atomic");
}

// ---------- BUG-10: card height must match layout reservations ----------
#[test]
fn test_card_height_matches_rendered_layout() {
    // Layout always reserves: 2 (borders) + 1 (title) + meta? + due? + 2 (desc)
    let plain = card("p"); // 2+1+0+0+2 = 5
    assert_eq!(get_card_height(&plain), 5);

    let mut cat_only = card("c");
    cat_only.category = Some("Work".into()); // meta row: 2+1+1+0+2 = 6
    assert_eq!(get_card_height(&cat_only), 6);

    let mut tagged = card("t");
    tagged.tags.push(Tag {
        name: "X".into(),
        color: None,
    }); // 2+1+1+0+2 = 6
    assert_eq!(get_card_height(&tagged), 6);

    let mut full = card("f");
    full.category = Some("Work".into());
    full.due_date = Some("2026-01-01".into());
    full.description = "text".into(); // 2+1+1+1+2 = 7
    assert_eq!(get_card_height(&full), 7);
}

// ---------- BUG-11: Ctrl/Alt+char must not leak into text inputs ----------
#[test]
fn test_modified_keys_do_not_type() {
    let (mut app, _dir) = create_app();
    app.mode = InputMode::Filter;

    process_event(&mut app, key_mod(KeyCode::Char('a'), KeyModifiers::CONTROL)).unwrap();
    process_event(&mut app, key_mod(KeyCode::Char('b'), KeyModifiers::ALT)).unwrap();
    assert!(app.filter_query.is_empty(), "Ctrl/Alt chars must not type");

    // Shift-modified chars are legit text input
    process_event(&mut app, key_mod(KeyCode::Char('A'), KeyModifiers::SHIFT)).unwrap();
    assert_eq!(app.filter_query, "A");
}

// ---------- BUG-12: 'x' must be typeable in the selector search ----------
#[test]
fn test_selector_x_is_searchable() {
    let (mut app, _dir) = create_app();
    app.start_new_card();
    app.open_selector(SelectorType::Tag);

    process_event(&mut app, key(KeyCode::Char('x'))).unwrap();
    assert_eq!(
        app.selector_state.as_ref().unwrap().search_query,
        "x",
        "'x' must go to the search query, not trigger delete"
    );
}

// ---------- BUG-14: category created via confirm fallback must be registered ----------
#[test]
fn test_category_confirm_registers_new_category() {
    let (mut app, _dir) = create_app();
    app.start_new_card();
    app.open_selector(SelectorType::Category);
    {
        let s = app.selector_state.as_mut().unwrap();
        s.selected_items.clear();
        s.search_query = "Infra".into();
    }
    app.selector_confirm();

    assert_eq!(app.edit_state.as_ref().unwrap().category, "Infra");
    assert!(
        app.data.known_categories.iter().any(|c| c.name == "Infra"),
        "new category must be registered in known_categories"
    );
}

// ---------- BUG-15: prompts store trimmed text ----------
#[test]
fn test_rename_prompt_is_trimmed() {
    let (mut app, _dir) = create_app();
    app.prompt_input = "   My Board   ".into();
    app.confirm_rename_project();
    assert_eq!(app.data.projects[0].name, "My Board");

    app.current_col_idx = 0;
    app.prompt_input = "  New Col ".into();
    app.confirm_rename_column();
    assert_eq!(app.data.projects[0].columns[0].title, "New Col");
}

// ---------- BUG-16: single-line cursor must count chars, not bytes ----------
#[test]
fn test_cursor_x_counts_chars_not_bytes() {
    let mut state = kanbanban::app::state::EditState::new();
    state.focused_field = EditField::Title;
    state.title = "éé".into(); // 4 bytes, 2 chars
    state.cursor_position = state.title.len();
    let (x, _) = state.get_cursor_position_2d(80);
    assert_eq!(x, 2, "cursor column must be char count, not byte count");
}

// ---------- BUG-18: markdown code blocks must render on separate lines ----------
#[test]
fn test_markdown_code_block_renders_lines() {
    let text = parse_markdown("```\nfirst line\nsecond line\n```");
    let rendered: Vec<String> = text
        .lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.to_string()).collect())
        .collect();
    let line1_idx = rendered.iter().position(|l| l.contains("first line"));
    let line2_idx = rendered.iter().position(|l| l.contains("second line"));
    assert!(
        line1_idx.is_some() && line2_idx.is_some() && line1_idx != line2_idx,
        "code block lines must render on separate lines, got: {:?}",
        rendered
    );
}

// ---------- BUG-21: help popup must not underflow on tiny terminals ----------
#[test]
fn test_help_popup_tiny_terminal() {
    use ratatui::{Terminal, backend::TestBackend};
    let backend = TestBackend::new(3, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    let (mut app, _dir) = create_app();
    app.mode = InputMode::Help;
    // Must not panic (usize underflow in separator width)
    terminal.draw(|f| kanbanban::ui::draw(f, &mut app)).unwrap();
}

// ---------- BUG-23: cycling into Description while editing must not dead-drop input ----------
#[test]
fn test_cycle_into_description_keeps_typing_alive() {
    let (mut app, _dir) = create_app();
    app.start_new_card();
    {
        let s = app.edit_state.as_mut().unwrap();
        s.is_editing_field = true; // insert mode on Title
        s.focused_field = EditField::Title;
    }
    // Cycle through to Description
    for _ in 0..4 {
        app.cycle_edit_field(false);
    }
    assert_eq!(
        app.edit_state.as_ref().unwrap().focused_field,
        EditField::Description
    );
    app.edit_input_char('z');
    assert_eq!(
        app.edit_state.as_ref().unwrap().description,
        "z",
        "typing after cycling to Description must not be silently dropped"
    );
}

// ---------- BUG: new card requires a column to exist ----------
#[test]
fn test_new_card_without_columns_shows_status() {
    let (mut app, _dir) = create_app();
    app.data.projects[0].columns.clear();

    app.start_new_card();
    assert_eq!(app.mode, InputMode::Normal, "must not open dead-end modal");
    assert!(app.status_message.is_some());
}

// ---------- GAP: filtered-out cards must not be rendered ----------
#[test]
fn test_filtered_board_hides_nonmatching_cards() {
    use ratatui::{Terminal, backend::TestBackend};
    let (mut app, _dir) = create_app();
    let col = &mut app.data.projects[0].columns[0];
    col.cards.push(card("AlphaTask"));
    col.cards.push(card("BetaTask"));

    app.filter_query = "beta".into();
    app.stop_filter();

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| kanbanban::ui::draw(f, &mut app)).unwrap();

    let buffer = terminal.backend().buffer();
    let mut content = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            content.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        content.push('\n');
    }
    assert!(content.contains("BetaTask"), "matching card must render");
    assert!(
        !content.contains("AlphaTask"),
        "filtered-out card must not render"
    );
}

// ---------- GAP: selector delete must clamp the cursor onto the filtered list ----------
#[test]
fn test_selector_delete_clamps_cursor() {
    let (mut app, _dir) = create_app();
    app.data.known_tags = ["a", "b", "c"]
        .iter()
        .map(|n| Tag {
            name: (*n).into(),
            color: None,
        })
        .collect();
    app.start_new_card();
    app.open_selector(SelectorType::Tag);

    // Delete the last item while the cursor sits on it: the cursor must
    // move back onto a real item rather than pointing past the end, where
    // a following Delete would silently no-op.
    app.selector_state.as_mut().unwrap().current_index = 2;
    app.selector_delete_item();
    {
        let s = app.selector_state.as_ref().unwrap();
        assert_eq!(s.get_filtered_items().len(), 2);
        assert!(
            s.current_index < s.get_filtered_items().len(),
            "cursor {} must stay on a real item after deletion",
            s.current_index
        );
    }
    app.selector_delete_item(); // must delete the clamped item, not no-op
    assert_eq!(
        app.selector_state
            .as_ref()
            .unwrap()
            .get_filtered_items()
            .len(),
        1
    );

    // With an active search, deleting the last matching item may leave the
    // cursor on the virtual "+ Create" slot (index == filtered_len) — but
    // never past it.
    {
        let s = app.selector_state.as_mut().unwrap();
        s.search_query = "a".into(); // only "a" remains and matches
        s.current_index = 0;
    }
    app.selector_delete_item(); // deletes "a"
    let s = app.selector_state.as_ref().unwrap();
    assert_eq!(s.get_filtered_items().len(), 0);
    assert_eq!(
        s.current_index, 0,
        "cursor must land on the virtual '+ Create' slot, never past it"
    );
}

// ---------- GAP: soft_wrap must count terminal display width ----------
#[test]
fn test_soft_wrap_counts_display_width() {
    use kanbanban::ui::modals::soft_wrap;
    // Each full-width char occupies 2 columns: 4 chars fit width 4 as two lines.
    assert_eq!(soft_wrap("ＡＢＣＤ", 4), vec!["ＡＢ", "ＣＤ"]);
    // A tab counts as 4 columns, forcing subsequent chars onto the next line.
    assert_eq!(soft_wrap("a\tb", 4), vec!["a", "\t", "b"]);
}
