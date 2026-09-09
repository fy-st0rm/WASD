use axum::{
  extract::ws::{Message, WebSocket, WebSocketUpgrade},
  response::Response,
};
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use turbojpeg::{Image, PixelFormat, Subsamp, compress};

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
  let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(1);

  let stop = Arc::new(AtomicBool::new(false));
  let capture_stop = Arc::clone(&stop);

  let capture_thread = std::thread::spawn(move || {
    let mut capture = match X11Capture::new(x, y, display.width, display.height) {
      Ok(capture) => capture,
      Err(e) => {
        eprintln!("Capture error: {e}");
        return;
      }
    };

    let frame_time = Duration::from_secs_f64(1.0 / display.fps as f64);

    loop {
      // Client disconnected?
      if capture_stop.load(Ordering::Relaxed) {
        break;
      }

      let start = Instant::now();

      let width = capture.width as usize;
      let height = capture.height as usize;

      let pixels = match capture.capture() {
        Ok(pixels) => pixels,
        Err(e) => {
          eprintln!("Capture error: {e}");
          return;
        }
      };

      let image = Image {
        pixels,
        width,
        pitch: width * 4,
        height,
        format: PixelFormat::BGRX,
      };

      let jpeg = match compress(image, 80, Subsamp::Sub2x2) {
        Ok(jpeg) => jpeg,
        Err(e) => {
          eprintln!("JPEG error: {e}");
          return;
        }
      };

      let _ = tx.try_send(jpeg.to_vec());

      let elapsed = start.elapsed();

      if elapsed < frame_time {
        std::thread::sleep(frame_time - elapsed);
      }
    }

    println!("Capture thread stopped");
  });

  loop {
    let jpeg = match rx.recv() {
      Ok(jpeg) => jpeg,
      Err(_) => break,
    };

    if socket.send(Message::Binary(jpeg.into())).await.is_err() {
      println!("Client disconnected");
      break;
    }
  }

  // Tell capture thread to stop.
  stop.store(true, Ordering::Relaxed);

  // Wait for capture thread to finish.
  let _ = capture_thread.join();
}
