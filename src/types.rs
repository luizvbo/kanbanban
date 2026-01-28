use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Tag {
    pub name: String,
    pub color: String, // Changed from Enum to String for flexibility
}

impl Tag {
    pub fn get_color(&self) -> Color {
        // 1. Try to parse as Hex (#RRGGBB)
        if self.color.starts_with('#') {
            if let Ok(c) = Color::from_str(&self.color) {
                return c;
            }
        }

        // 2. Match named colors (Case insensitive)
        match self.color.to_lowercase().as_str() {
            "red" => Color::Red,
            "green" => Color::Green,
            "blue" => Color::Blue,
            "yellow" => Color::Yellow,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "white" => Color::White,
            "gray" => Color::Gray,
            "black" => Color::Black,
            "lightred" => Color::LightRed,
            "lightgreen" => Color::LightGreen,
            "lightblue" => Color::LightBlue,
            "orange" => Color::Rgb(255, 165, 0),
            _ => Color::White, // Default fallback
        }
    }
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
    // NEW: Store all known tags here so they persist in YAML
    #[serde(default)]
    pub known_tags: Vec<Tag>,
}

impl KanbanData {
    pub fn default() -> Self {
        Self {
            known_tags: vec![
                Tag {
                    name: "Bug".into(),
                    color: "Red".into(),
                },
                Tag {
                    name: "Feature".into(),
                    color: "Green".into(),
                },
                Tag {
                    name: "Urgent".into(),
                    color: "Magenta".into(),
                },
            ],
            projects: vec![Project {
                name: "Default Board".to_string(),
                columns: vec![
                    Column {
                        title: "To Do".into(),
                        cards: vec![Card {
                            title: "Welcome".into(),
                            description: "Press `?` for help.".into(),
                            tags: vec![Tag {
                                name: "Bug".into(),
                                color: "Red".into(),
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
        // FIX: Added ::<KanbanData> type annotation
        match serde_yaml::from_str::<KanbanData>(&content) {
            Ok(mut data) => {
                if data.known_tags.is_empty() {
                    data.known_tags = Self::default().known_tags;
                }
                Ok(data)
            }
            Err(_) => Ok(Self::default()),
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_yaml::to_string(self).context("Failed to serialize data")?;
        fs::write(path, content).context("Failed to write data file")?;
        Ok(())
    }
}
