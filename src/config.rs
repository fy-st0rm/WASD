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
    let text = fs::read_to_string("wasd.toml")?;
    let config: Config = toml::from_str(&text)?;

    if !matches!(config.display.rotation, 0 | 90 | 180 | 270) {
      return Err("rotation must be 0, 90, 180, or 270".into());
    }

    Ok(config)
  }
}
