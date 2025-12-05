use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub theme: String,
    pub columns_in_view: u16,
    pub database_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "dracula".to_string(),
            columns_in_view: 3,
            database_path: None,
        }
    }
}

pub fn load_config() -> Result<(Config, PathBuf)> {
    let proj_dirs = ProjectDirs::from("", "", "kanbanban")
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

    let config_dir = proj_dirs.config_dir();
    let data_dir = proj_dirs.data_dir();

    if !config_dir.exists() {
        std::fs::create_dir_all(config_dir)?;
    }
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir)?;
    }

    let config_path = config_dir.join("config.toml");
    let db_path = data_dir.join("kanbanban.yaml");

    let config = if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)?;
        toml::from_str(&contents).unwrap_or_default()
    } else {
        Config::default()
    };

    Ok((config, db_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.theme, "dracula");
        assert_eq!(config.columns_in_view, 3);
        assert_eq!(config.database_path, None);
    }

    #[test]
    fn test_parse_config() {
        let toml_str = r#"
            theme = "monokai"
            columns_in_view = 5
            database_path = "/tmp/db.yaml"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.theme, "monokai");
        assert_eq!(config.columns_in_view, 5);
        assert_eq!(config.database_path, Some("/tmp/db.yaml".to_string()));
    }

    #[test]
    fn test_load_config_logic() {
        let mut tmp_file = NamedTempFile::new().unwrap();
        write!(tmp_file, "theme = 'test'").unwrap();

        let content = std::fs::read_to_string(tmp_file.path()).unwrap();
        let config: Config = toml::from_str(&content).unwrap();

        assert_eq!(config.theme, "test");
        assert_eq!(config.columns_in_view, 3);
    }
}
