use kanbanban::types::{Card, KanbanData, Tag};
use tempfile::NamedTempFile;

#[test]
fn test_save_and_load_cycle() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_path_buf();

    let mut data = KanbanData::default();
    data.projects[0].name = "Integration Test Board".into();

    // FIX: Clear the default columns/cards so we start fresh
    data.projects[0].columns[0].cards.clear();

    data.projects[0].columns[0].cards.push(Card {
        title: "Persisted Card".into(),
        description: "Content".into(),
        category: Some("DevOps".into()),
        tags: vec![Tag {
            name: "Urgent".into(),
            color: Some("Red".into()),
        }],
        due_date: Some("2025-01-01".into()),
    });

    data.save(&path).expect("Failed to save data");

    let loaded_data = KanbanData::load(&path).expect("Failed to load data");

    assert_eq!(loaded_data.projects[0].name, "Integration Test Board");
    // Now this assertion will pass (1 == 1)
    assert_eq!(loaded_data.projects[0].columns[0].cards.len(), 1);

    let card = &loaded_data.projects[0].columns[0].cards[0];
    assert_eq!(card.title, "Persisted Card");
    assert_eq!(card.category, Some("DevOps".into()));
    assert_eq!(card.tags[0].name, "Urgent");
}
