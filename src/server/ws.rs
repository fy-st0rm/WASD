use axum::{
  extract::ws::{Message, WebSocket, WebSocketUpgrade},
  response::Response,
};
use bytes::Bytes;
use libjpeg_turbo_rs::{Encoder, PixelFormat, Subsampling};

use crate::capture::x11::X11Capture;

pub async fn websocket(
  ws: WebSocketUpgrade,
  display: crate::config::DisplayConfig,
  x: i16,
  y: i16,
) -> Response {
  ws.on_upgrade(move |socket| handle_socket(socket, display, x, y))
}

async fn handle_socket(
  mut socket: WebSocket,
  display: crate::config::DisplayConfig,
  x: i16,
  y: i16,
) {
  // flume bounded channels: sync `.send()` is lock-free (no tokio semaphore
  // parking); async `.recv_async()` integrates naturally with tokio select!.
  let (request_tx, request_rx) = flume::bounded::<()>(2);
  // Buffer up to 2 frames so the encoder can pipeline one frame ahead while
  // the previous one is in-flight over the wire.
  let (frame_tx, frame_rx) = flume::bounded::<Bytes>(2);

  std::thread::spawn(move || {
    let mut capture = match X11Capture::new(x, y, display.width, display.height) {
      Ok(capture) => capture,
      Err(e) => {
        eprintln!("Capture error: {e}");
        return;
      }
    };

    while let Ok(()) = request_rx.recv() {
      let width = capture.width as usize;
      let height = capture.height as usize;

      let pixels = match capture.capture() {
        Ok(pixels) => pixels,
        Err(e) => {
          eprintln!("Capture error: {e}");
          return;
        }
      };

      // libjpeg-turbo-rs: pure Rust SIMD encoder, no FFI overhead.
      // Encoder::new borrows the pixel slice — no copy needed.
      let jpeg = match Encoder::new(pixels, width, height, PixelFormat::Bgrx)
        .quality(75)
        .subsampling(Subsampling::S420)
        .encode()
      {
        Ok(jpeg) => jpeg,
        Err(e) => {
          eprintln!("JPEG error: {e}");
          return;
        }
      };

      // Bytes::from(Vec<u8>) is a zero-copy move — no heap allocation.
      if frame_tx.send(Bytes::from(jpeg)).is_err() {
        break;
      }
    }
  });

  // Request initial frame immediately upon connection.
  let _ = request_tx.try_send(());

  loop {
    tokio::select! {
      Ok(jpeg) = frame_rx.recv_async() => {
        if socket.send(Message::Binary(jpeg)).await.is_err() {
          println!("Client disconnected");
          break;
        }
      }

      msg = socket.recv() => {
        match msg {
          Some(Ok(Message::Text(text))) if text == "ack" => {
            let _ = request_tx.try_send(());
          }
          Some(Ok(Message::Binary(bin))) if bin.as_ref() == b"ack" => {
            let _ = request_tx.try_send(());
          }
          Some(Ok(Message::Close(_))) | None => {
            println!("Client disconnected");
            break;
          }
          Some(Err(e)) => {
            eprintln!("WebSocket error: {e}");
            break;
          }
          _ => {}
        }
      }
    }
  }
}
