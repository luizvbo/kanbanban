use serde::{Deserialize, Serialize};
use ratatui::style::Color;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub tag_colors: HashMap<String, String>,
}

impl Config {
    pub fn default() -> Self {
        let mut map = HashMap::new();
        // Default presets
        map.insert("bug".to_string(), "Red".to_string());
        map.insert("feature".to_string(), "Green".to_string());
        map.insert("urgent".to_string(), "Magenta".to_string());
        map.insert("info".to_string(), "Blue".to_string());

        Self { tag_colors: map }
    }

    pub fn load(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content).unwrap_or(Self::default());
        Ok(config)
    }

    pub fn get_color(&self, tag_name: &str) -> Color {
        // 1. Check exact match
        if let Some(color_str) = self.tag_colors.get(tag_name) {
            return parse_color(color_str);
        }
        // 2. Fallback: Hash string to a stable color
        let hash: usize = tag_name.bytes().map(|b| b as usize).sum();
        let colors = [
            Color::Red, Color::Green, Color::Yellow, Color::Blue,
            Color::Magenta, Color::Cyan, Color::LightRed, Color::LightGreen
        ];
        colors[hash % colors.len()]
    }
}

fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "red" => Color::Red,
        "green" => Color::Green,
        "blue" => Color::Blue,
        "yellow" => Color::Yellow,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "black" => Color::Black,
        "gray" => Color::Gray,
        _ => Color::White,
    }
}
