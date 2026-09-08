use axum::{
  extract::ws::{Message, WebSocket, WebSocketUpgrade},
  response::Response,
};

use turbojpeg::{Image, PixelFormat, Subsamp, compress};

use crate::capture::x11::X11Capture;

pub async fn websocket(ws: WebSocketUpgrade) -> Response {
  ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
  let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(1);

  std::thread::spawn(move || {
    let mut capture = match X11Capture::new(1366, 0, 1024, 768) {
      Ok(capture) => capture,
      Err(e) => {
        eprintln!("Capture error: {e}");
        return;
      }
    };

    loop {
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
