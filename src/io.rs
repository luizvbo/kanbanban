use crate::types::KanbanData;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Loads the Kanban state from a YAML file.
/// If the file doesn't exist, returns the default state.
pub fn load_kanban_data<P: AsRef<Path>>(path: P) -> Result<KanbanData> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(KanbanData::default());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read data file at {:?}", path))?;

    let data: KanbanData = serde_yaml::from_str(&content)
        .with_context(|| "Failed to parse YAML data. Check file syntax.")?;

    Ok(data)
}

/// Saves the Kanban state to a YAML file.
/// Creates the file if it doesn't exist, overwrites if it does.
pub fn save_kanban_data<P: AsRef<Path>>(data: &KanbanData, path: P) -> Result<()> {
    let path = path.as_ref();
    let content =
        serde_yaml::to_string(data).with_context(|| "Failed to serialize Kanban data to YAML")?;

    fs::write(path, content).with_context(|| format!("Failed to write data file to {:?}", path))?;

    Ok(())
}
