mod capture;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  server::http::start().await?;

  Ok(())
}
