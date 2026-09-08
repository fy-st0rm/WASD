use axum::{
  extract::ws::{Message, WebSocket, WebSocketUpgrade},
  response::Response,
};

use std::time::Instant;
use turbojpeg::{Image, PixelFormat, Subsamp, compress};

use crate::capture::x11::X11Capture;

pub async fn websocket(ws: WebSocketUpgrade) -> Response {
  ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
  let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(1);

  std::thread::spawn(move || {
    let capture = match X11Capture::new(1366, 0, 1024, 768) {
      Ok(capture) => capture,
      Err(e) => {
        eprintln!("Capture error: {e}");
        return;
      }
    };

    let mut frames = 0;
    let mut last = Instant::now();

    loop {
      let pixels = match capture.capture() {
        Ok(pixels) => pixels,
        Err(e) => {
          eprintln!("Capture error: {e}");
          return;
        }
      };

      // X11 gives us BGRX.
      // Convert to RGB for JPEG.
      let mut rgb = Vec::with_capacity(pixels.len() / 4 * 3);

      for pixel in pixels.chunks_exact(4) {
        rgb.push(pixel[2]); // R
        rgb.push(pixel[1]); // G
        rgb.push(pixel[0]); // B
      }

      let image = Image {
        pixels: rgb.as_slice(),
        width: capture.width as usize,
        pitch: capture.width as usize * 3,
        height: capture.height as usize,
        format: PixelFormat::RGB,
      };

      let jpeg = match compress(image, 80, Subsamp::Sub2x2) {
        Ok(jpeg) => jpeg,
        Err(e) => {
          eprintln!("JPEG error: {e}");
          return;
        }
      };

      let _ = tx.try_send(jpeg.to_vec());

      frames += 1;

      if last.elapsed().as_secs() >= 1 {
        println!("JPEG FPS: {frames}");
        frames = 0;
        last = Instant::now();
      }
    }
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
}
