use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TagColor {
    Red,
    Green,
    Blue,
    Yellow,
    Magenta,
    Cyan,
    White,
    Black,
    Gray,
}

impl From<&TagColor> for Color {
    fn from(val: &TagColor) -> Self {
        match val {
            TagColor::Red => Color::Red,
            TagColor::Green => Color::Green,
            TagColor::Blue => Color::Blue,
            TagColor::Yellow => Color::Yellow,
            TagColor::Magenta => Color::Magenta,
            TagColor::Cyan => Color::Cyan,
            TagColor::White => Color::White,
            TagColor::Black => Color::Black,
            TagColor::Gray => Color::DarkGray,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub name: String,
    pub color: TagColor,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Card {
    pub title: String,
    pub description: String, // Markdown content
    pub tags: Vec<Tag>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Column {
    pub title: String,
    pub cards: Vec<Card>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub name: String,
    pub columns: Vec<Column>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KanbanData {
    pub projects: Vec<Project>,
}

impl KanbanData {
    pub fn default() -> Self {
        Self {
            projects: vec![Project {
                name: "Default Board".to_string(),
                columns: vec![
                    Column {
                        title: "To Do".into(),
                        cards: vec![Card {
                            title: "Welcome".into(),
                            description:
                                "Press `?` for help.\n\n# Features\n- Vim keys\n- Markdown".into(),
                            tags: vec![Tag {
                                name: "Info".into(),
                                color: TagColor::Blue,
                            }],
                        }],
                    },
                    Column {
                        title: "In Progress".into(),
                        cards: vec![],
                    },
                    Column {
                        title: "Done".into(),
                        cards: vec![],
                    },
                ],
            }],
        }
    }

    pub fn load(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).context("Failed to read data file")?;

        // Fix: Handle empty files or parsing errors gracefully
        if content.trim().is_empty() {
            return Ok(Self::default());
        }

        match serde_yaml::from_str(&content) {
            Ok(data) => Ok(data),
            Err(_) => {
                // If the file is corrupt or doesn't match the schema,
                // fallback to default to prevent the app from crashing.
                // In a real app, you might want to backup the corrupt file here.
                Ok(Self::default())
            }
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_yaml::to_string(self).context("Failed to serialize data")?;
        fs::write(path, content).context("Failed to write data file")?;
        Ok(())
    }
}
