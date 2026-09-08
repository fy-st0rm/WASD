mod capture;

use capture::x11::X11Capture;

use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let capture = X11Capture::new(1366, 0, 1024, 768)?;

  let frame_time = Duration::from_secs_f64(1.0 / 60.0);

  let mut frames = 0;
  let mut fps_timer = Instant::now();

  loop {
    let start = Instant::now();

    let pixels = capture.capture()?;

    // For now, we're just proving that
    // the capture module works.
    let _ = pixels;

    frames += 1;

    if fps_timer.elapsed() >= Duration::from_secs(1) {
      println!("FPS: {}", frames);

      frames = 0;
      fps_timer = Instant::now();
    }

    let elapsed = start.elapsed();

    if elapsed < frame_time {
      std::thread::sleep(frame_time - elapsed);
    }
  }
}
