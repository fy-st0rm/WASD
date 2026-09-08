mod capture;
mod config;
mod display;
mod server;

use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let config = Config::load()?;

  println!("Config: {:#?}", config);

  let (x, y) = display::configure(
    config.display.width,
    config.display.height,
    &config.display.direction,
  )?;

  server::run(config.server.port, config.display, x, y).await?;

  Ok(())
}
