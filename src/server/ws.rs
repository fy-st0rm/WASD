use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket};
use axum::{extract::WebSocketUpgrade, response::Response};

use tokio::sync::broadcast;
use turbojpeg::{Image, PixelFormat, Subsamp, compress};

use crate::capture::x11::X11Capture;

pub type FrameSender = broadcast::Sender<Vec<u8>>;

pub fn start_capture(
  display: crate::config::DisplayConfig,
  x: i16,
  y: i16,
) -> (FrameSender, Arc<AtomicBool>, thread::JoinHandle<()>) {
  let (tx, _) = broadcast::channel::<Vec<u8>>(1);

  let stop = Arc::new(AtomicBool::new(false));
  let capture_stop = Arc::clone(&stop);

  let capture_tx = tx.clone();

  let thread = thread::spawn(move || {
    let mut capture = match X11Capture::new(x, y, display.width, display.height) {
      Ok(capture) => capture,

      Err(e) => {
        eprintln!("Capture error: {e}");
        return;
      }
    };

    let frame_time = Duration::from_secs_f64(1.0 / display.fps as f64);

    loop {
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
          break;
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
          break;
        }
      };

      let _ = capture_tx.send(jpeg.to_vec());

      let elapsed = start.elapsed();

      if elapsed < frame_time {
        thread::sleep(frame_time - elapsed);
      }
    }

    println!("Capture thread stopped");
  });

  (tx, stop, thread)
}

pub async fn websocket(ws: WebSocketUpgrade, tx: FrameSender) -> Response {
  ws.on_upgrade(move |socket| handle_socket(socket, tx))
}

async fn handle_socket(mut socket: WebSocket, tx: FrameSender) {
  let mut rx = tx.subscribe();

  loop {
    let jpeg = match rx.recv().await {
      Ok(jpeg) => jpeg,

      Err(broadcast::error::RecvError::Lagged(_)) => {
        // Client was too slow.
        // Skip old frames and get the newest one.
        continue;
      }

      Err(broadcast::error::RecvError::Closed) => {
        break;
      }
    };

    if socket.send(Message::Binary(jpeg.into())).await.is_err() {
      println!("Client disconnected");
      break;
    }
  }
}
