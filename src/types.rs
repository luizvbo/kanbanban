use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TagColor {
    Red,
    Green,
    Blue,
    Yellow,
    Magenta,
    Cyan,
    White,
    Gray,
    LightRed,
    LightGreen,
    LightBlue,
    Orange,
}

impl TagColor {
    // Helper for the selector UI
    pub fn iterator() -> impl Iterator<Item = TagColor> {
        [
            TagColor::Red,
            TagColor::Green,
            TagColor::Blue,
            TagColor::Yellow,
            TagColor::Magenta,
            TagColor::Cyan,
            TagColor::White,
            TagColor::Gray,
            TagColor::LightRed,
            TagColor::LightGreen,
            TagColor::LightBlue,
            TagColor::Orange,
        ]
        .iter()
        .cloned()
    }
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
            TagColor::Gray => Color::DarkGray,
            TagColor::LightRed => Color::LightRed,
            TagColor::LightGreen => Color::LightGreen,
            TagColor::LightBlue => Color::LightBlue,
            TagColor::Orange => Color::Rgb(255, 165, 0),
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
    pub description: String,
    pub tags: Vec<Tag>,
    pub due_date: Option<String>,
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
                            description: "Press `?` for help.".into(),
                            tags: vec![Tag {
                                name: "Info".into(),
                                color: TagColor::Blue,
                            }],
                            due_date: None,
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
        if content.trim().is_empty() {
            return Ok(Self::default());
        }
        match serde_yaml::from_str(&content) {
            Ok(data) => Ok(data),
            Err(_) => Ok(Self::default()),
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_yaml::to_string(self).context("Failed to serialize data")?;
        fs::write(path, content).context("Failed to write data file")?;
        Ok(())
    }
}
