pub mod ws;

use axum::{Router, response::Html, routing::get, serve::ListenerExt};
use std::net::UdpSocket;

use crate::config::DisplayConfig;

fn local_ip() -> Result<std::net::IpAddr, Box<dyn std::error::Error>> {
  let socket = UdpSocket::bind("0.0.0.0:0")?;

  socket.connect("8.8.8.8:80")?;

  Ok(socket.local_addr()?.ip())
}

pub async fn run(
  port: u16,
  display: DisplayConfig,
  x: i16,
  y: i16,
) -> Result<(), Box<dyn std::error::Error>> {
  let rotation = display.rotation;

  let html = include_str!("../../web/index.html").replace("__ROTATION__", &rotation.to_string());

  let app = Router::new()
    .route(
      "/",
      get(move || {
        let html = html.clone();

        async move { Html(html) }
      }),
    )
    .route(
      "/ws",
      get(move |ws| ws::websocket(ws, display.clone(), x, y)),
    );

  let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

  let ip = local_ip()?;

  println!("WASD server: http://{}:{}", ip, port);

  let listener = tokio::net::TcpListener::bind(addr).await?;

  axum::serve(
    listener.tap_io(|stream| {
      let _ = stream.set_nodelay(true);
    }),
    app,
  )
  .await?;

  Ok(())
}
