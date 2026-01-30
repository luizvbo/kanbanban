use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

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
            "black" => Color::Black,
            "lightred" => Color::LightRed,
            "lightgreen" => Color::LightGreen,
            "lightblue" => Color::LightBlue,
            "orange" => Color::Rgb(255, 165, 0),
            _ => Color::White,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Card {
    pub title: String,
    pub description: String,
    pub category: Option<String>,
    pub tags: Vec<Tag>,
    pub due_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Column {
    pub title: String,
    pub cards: Vec<Card>,
    #[serde(skip)]
    pub scroll_offset: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub name: String,
    pub columns: Vec<Column>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KanbanData {
    pub projects: Vec<Project>,
    #[serde(default)]
    pub known_tags: Vec<Tag>,
}

impl Default for KanbanData {
    fn default() -> Self {
        Self {
            known_tags: vec![
                Tag {
                    name: "bug".into(),
                    color: Some("Red".into()),
                },
                Tag {
                    name: "feature".into(),
                    color: Some("Green".into()),
                },
            ],
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
    pub fn current_project_mut(&mut self, index: usize) -> Option<&mut Project> {
        self.projects.get_mut(index)
    }

    pub fn move_card(
        &mut self,
        project_idx: usize,
        from_col: usize,
        to_col: usize,
        card_idx: usize,
    ) -> Option<usize> {
        let project = self.projects.get_mut(project_idx)?;

        // Validation logic moved here
        if from_col >= project.columns.len() || to_col >= project.columns.len() {
            return None;
        }

        let card = project.columns[from_col].cards.remove(card_idx);
        project.columns[to_col].cards.push(card);

        Some(project.columns[to_col].cards.len() - 1) // Return new index
    }

    pub fn load(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path).context("Failed to read data file")?;
        if content.trim().is_empty() {
            return Ok(Self::default());
        }
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

    // FIX: Logic to resolve color: Local -> Global -> Default
    pub fn get_tag_color(&self, tag: &Tag) -> Color {
        // 1. Check if card tag has specific color override
        if let Some(c) = &tag.color {
            return Tag::parse_color_string(c);
        }
        // 2. Check global known_tags
        if let Some(global_tag) = self.known_tags.iter().find(|t| t.name == tag.name)
            && let Some(c) = &global_tag.color
        {
            return Tag::parse_color_string(c);
        }
        // 3. Fallback
        Color::White
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
