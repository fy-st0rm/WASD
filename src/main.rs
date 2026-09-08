mod capture;
mod config;
mod display;
mod server;

use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let config = Config::load()?;

  let (x, y, output) = display::configure(
    config.display.width,
    config.display.height,
    &config.display.direction,
  )?;
  display::move_workspace(config.display.workspace, &output)?;

  let server = server::run(config.server.port, config.display.clone(), x, y);

  tokio::select! {
      result = server => {
          result?;
      }

      _ = tokio::signal::ctrl_c() => {
          println!("\nShutting down...");
      }
  }

  display::cleanup()?;

  Ok(())
}
