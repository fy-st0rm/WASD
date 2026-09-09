use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
  pub display: DisplayConfig,
  pub server: ServerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DisplayConfig {
  pub width: u16,
  pub height: u16,
  pub direction: Direction,
  pub rotation: i32,
  pub workspace: u32,
  pub fps: u32,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
  pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
  Left,
  Right,
  Top,
  Bottom,
}

impl Config {
  pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
    let config_dir = dirs::config_dir()
      .ok_or("Could not find config directory")?
      .join("wasd");

    let config_path = config_dir.join("wasd.toml");

    // Create default config if it doesn't exist
    if !config_path.exists() {
      fs::create_dir_all(&config_dir)?;

      let default_config = r#"[display]
width = 1024
height = 768
direction = "right"
rotation = 90
workspace = 10
fps = 30

[server]
port = 8080
"#;

      fs::write(&config_path, default_config)?;

      println!("Created default config at {}", config_path.display());
    }

    let text = fs::read_to_string(&config_path)?;
    let config: Config = toml::from_str(&text)?;

    if !matches!(config.display.rotation, 0 | 90 | 180 | 270) {
      return Err("rotation must be 0, 90, 180, or 270".into());
    }

    Ok(config)
  }
}
