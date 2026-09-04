use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

/// Represents a label attached to a card.
/// Tags can have an optional color. if None, the app looks up
/// the color in the global `known_tags` list.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Tag {
    pub name: String,
    pub color: Option<String>,
}

impl Tag {
    // Helper to parse string to Color
    pub fn parse_color_string(s: &str) -> Color {
        if s.starts_with('#')
            && let Ok(c) = Color::from_str(s)
        {
            return c;
        }
        match s.to_lowercase().as_str() {
            "red" => Color::Red,
            "green" => Color::Green,
            "blue" => Color::Blue,
            "yellow" => Color::Yellow,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "white" => Color::White,
            "gray" => Color::Gray,
            "darkgray" => Color::DarkGray,
            "black" => Color::Black,
            "lightred" => Color::LightRed,
            "lightgreen" => Color::LightGreen,
            "lightblue" => Color::LightBlue,
            "lightyellow" => Color::LightYellow,
            "lightmagenta" => Color::LightMagenta,
            "lightcyan" => Color::LightCyan,
            "orange" => Color::Rgb(255, 165, 0),
            _ => Color::White,
        }
    }
}

/// The fundamental unit of the Kanban board.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Card {
    pub title: String,
    pub description: String,
    pub category: Option<String>,
    pub tags: Vec<Tag>,
    pub due_date: Option<String>,
}

/// A vertical list containing cards.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Column {
    pub title: String,
    pub cards: Vec<Card>,
    /// UI-only state: tracks vertical scrolling within this column
    /// ignored during YAML serialization.
    #[serde(skip)]
    pub scroll_offset: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub name: String,
    pub columns: Vec<Column>,
}

/// Root structure for YAML persistence.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KanbanData {
    pub projects: Vec<Project>,
    /// Global registry of tags and their colors to ensure
    /// consistency across the whole board.
    #[serde(default)]
    pub known_tags: Vec<Tag>,
    #[serde(default)]
    pub known_categories: Vec<Tag>,
}

impl Default for KanbanData {
    fn default() -> Self {
        Self {
            known_tags: vec![
                Tag {
                    name: "Bug".into(),
                    color: Some("Red".into()),
                },
                Tag {
                    name: "Feature".into(),
                    color: Some("Green".into()),
                },
            ],
            known_categories: vec![Tag {
                name: "General".into(),
                color: Some("Blue".into()),
            }],
            projects: vec![Project {
                name: "Default Board".to_string(),
                columns: vec![
                    Column {
                        title: "To Do".into(),
                        cards: vec![Card {
                            title: "Welcome".into(),
                            description:
                                "Press `v` to view details.\nPress `o` to open in external editor."
                                    .into(),
                            category: Some("General".into()),
                            tags: vec![Tag {
                                name: "Bug".into(),
                                color: None,
                            }],
                            due_date: None,
                        }],
                        scroll_offset: 0,
                    },
                    Column {
                        title: "In Progress".into(),
                        cards: vec![],
                        scroll_offset: 0,
                    },
                    Column {
                        title: "Done".into(),
                        cards: vec![],
                        scroll_offset: 0,
                    },
                ],
            }],
        }
    }
}

impl KanbanData {
    pub fn load(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path).context("Failed to read data file")?;
        if content.trim().is_empty() {
            return Ok(Self::default());
        }
        let mut data = serde_yaml::from_str::<KanbanData>(&content)
            .with_context(|| format!("Failed to parse data file {:?}", path))?;
        if data.known_tags.is_empty() {
            data.known_tags = Self::default().known_tags;
        }
        Ok(data)
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_yaml::to_string(self).context("Failed to serialize data")?;
        fs::write(path, content).context("Failed to write data file")?;
        Ok(())
    }

    /// Resolves a tag's color using a hierarchy:
    /// 1. Use specific color on the card's tag if present.
    /// 2. Use color from global `known_tags` if names match.
    /// 3. Fallback to White.
    pub fn get_tag_color(&self, tag: &Tag) -> Color {
        // 1. Check if card tag has specific color override
        if let Some(c) = &tag.color {
            return Tag::parse_color_string(c);
        }
        // 2. Check global known_tags
        if let Some(global) = self.known_tags.iter().find(|t| t.name == tag.name)
            && let Some(c) = &global.color
        {
            return Tag::parse_color_string(c);
        }
        // 3. Fallback
        Color::White
    }

    pub fn get_category_color(&self, category_name: &str) -> Color {
        if let Some(cat) = self
            .known_categories
            .iter()
            .find(|c| c.name == category_name)
            && let Some(c) = &cat.color
        {
            return Tag::parse_color_string(c);
        }
        Color::White
    }

    /// Returns a color from a predefined distinct palette based on index.
    pub fn get_next_color(index: usize) -> String {
        let palette = [
            "Red",
            "Green",
            "Blue",
            "Yellow",
            "Magenta",
            "Cyan",
            "LightRed",
            "LightGreen",
            "LightBlue",
            "LightYellow",
            "LightMagenta",
            "LightCyan",
            "Orange",
            "Gray",
            "DarkGray",
        ];
        palette[index % palette.len()].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_color_parsing() {
        assert_eq!(Tag::parse_color_string("Red"), Color::Red);
        assert_eq!(Tag::parse_color_string("red"), Color::Red);
        assert_eq!(Tag::parse_color_string("#ff0000"), Color::Rgb(255, 0, 0));
        assert_eq!(Tag::parse_color_string("unknown_color"), Color::White); // Fallback
    }

    #[test]
    fn test_tag_color_resolution() {
        let mut data = KanbanData::default();

        // 1. Setup Global Tag
        data.known_tags.push(Tag {
            name: "GlobalTag".into(),
            color: Some("Blue".into()),
        });

        // Case A: Tag has specific color (override)
        let local_tag = Tag {
            name: "GlobalTag".into(),
            color: Some("Red".into()),
        };
        assert_eq!(data.get_tag_color(&local_tag), Color::Red);

        // Case B: Tag has no color, should use Global
        let local_tag_inherit = Tag {
            name: "GlobalTag".into(),
            color: None,
        };
        assert_eq!(data.get_tag_color(&local_tag_inherit), Color::Blue);

        // Case C: Tag unknown, fallback to White
        let unknown_tag = Tag {
            name: "Mystery".into(),
            color: None,
        };
        assert_eq!(data.get_tag_color(&unknown_tag), Color::White);
    }

    #[test]
    fn test_default_data_integrity() {
        let data = KanbanData::default();
        assert!(!data.projects.is_empty());
        assert!(!data.projects[0].columns.is_empty());
        assert!(!data.known_tags.is_empty());
    }
}
