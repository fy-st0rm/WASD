use std::{ffi::c_void, ptr};

use libc::{
  IPC_CREAT, IPC_EXCL, IPC_PRIVATE, IPC_RMID, S_IRUSR, S_IWUSR, shmat, shmctl, shmdt, shmget,
};

use x11rb::{
  connection::Connection,
  protocol::{
    shm::ConnectionExt as ShmConnectionExt, xfixes::ConnectionExt as XFixesConnectionExt,
  },
};

pub struct X11Capture {
  conn: x11rb::rust_connection::RustConnection,
  root: u32,

  x: i16,
  y: i16,

  shmseg: u32,
  shmaddr: *mut c_void,

  pub width: u16,
  pub height: u16,

  size: usize,

  // Writable BGRX framebuffer used for cursor compositing.
  frame: Vec<u8>,

  cursor: Option<Cursor>,
}

struct Cursor {
  x: i16,
  y: i16,

  width: u16,
  height: u16,

  xhot: u16,
  yhot: u16,

  // XFixes returns the cursor as ARGB pixels.
  pixels: Vec<u32>,
}

impl X11Capture {
  pub fn new(x: i16, y: i16, width: u16, height: u16) -> Result<Self, Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen_num].root;
    let xfixes_version = conn.xfixes_query_version(6, 0)?.reply()?;

    println!(
      "XFixes version: {}.{}",
      xfixes_version.major_version, xfixes_version.minor_version
    );

    let size = width as usize * height as usize * 4;

    println!(
      "Screen: {}x{}",
      conn.setup().roots[screen_num].width_in_pixels,
      conn.setup().roots[screen_num].height_in_pixels
    );

    println!("Capture: {}x{} at ({}, {})", width, height, x, y);

    println!("Shared memory size: {} bytes", size);

    let shmid = unsafe {
      shmget(
        IPC_PRIVATE,
        size,
        IPC_CREAT | IPC_EXCL | S_IRUSR as i32 | S_IWUSR as i32,
      )
    };

    if shmid < 0 {
      return Err(std::io::Error::last_os_error().into());
    }

    let shmaddr = unsafe { shmat(shmid, ptr::null(), 0) };

    if shmaddr == (-1isize) as *mut c_void {
      unsafe {
        shmctl(shmid, IPC_RMID, ptr::null_mut());
      }

      return Err(std::io::Error::last_os_error().into());
    }

    // Mark the shared memory for deletion.
    // It will disappear automatically after detach.
    unsafe {
      shmctl(shmid, IPC_RMID, ptr::null_mut());
    }

    let shmseg = conn.generate_id()?;

    conn.shm_attach(shmseg, shmid as u32, false)?.check()?;

    println!("MIT-SHM attached successfully");

    Ok(Self {
      conn,
      root,

      x,
      y,

      shmseg,
      shmaddr,

      width,
      height,

      size,

      frame: vec![0u8; size],

      cursor: None,
    })
  }

  fn update_cursor(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    let cursor = self.conn.xfixes_get_cursor_image()?.reply()?;

    self.cursor = Some(Cursor {
      x: cursor.x,
      y: cursor.y,

      width: cursor.width,
      height: cursor.height,

      xhot: cursor.xhot,
      yhot: cursor.yhot,

      pixels: cursor.cursor_image,
    });

    Ok(())
  }

  fn draw_cursor(&self, frame: &mut [u8]) {
    let cursor = match &self.cursor {
      Some(cursor) => cursor,
      None => return,
    };

    // Convert global X11 cursor coordinates into
    // coordinates relative to our capture rectangle.
    let cursor_x = cursor.x as i32 - self.x as i32 - cursor.xhot as i32;

    let cursor_y = cursor.y as i32 - self.y as i32 - cursor.yhot as i32;

    for cy in 0..cursor.height as i32 {
      for cx in 0..cursor.width as i32 {
        let dst_x = cursor_x + cx;
        let dst_y = cursor_y + cy;

        // Cursor is outside the capture region.
        if dst_x < 0 || dst_y < 0 || dst_x >= self.width as i32 || dst_y >= self.height as i32 {
          continue;
        }

        let src_index = (cy * cursor.width as i32 + cx) as usize;

        let pixel = cursor.pixels[src_index];

        // XFixes cursor format:
        //
        // 0xAARRGGBB
        //
        let a = ((pixel >> 24) & 0xff) as u32;

        // Completely transparent.
        if a == 0 {
          continue;
        }

        let r = ((pixel >> 16) & 0xff) as u32;

        let g = ((pixel >> 8) & 0xff) as u32;

        let b = (pixel & 0xff) as u32;

        let dst_index = ((dst_y * self.width as i32 + dst_x) * 4) as usize;

        // Our framebuffer is BGRX.
        if a == 255 {
          frame[dst_index] = b as u8;
          frame[dst_index + 1] = g as u8;
          frame[dst_index + 2] = r as u8;

          // X byte can stay unchanged.
        } else {
          // Alpha blend cursor over framebuffer.

          let inv_a = 255 - a;

          let old_b = frame[dst_index] as u32;

          let old_g = frame[dst_index + 1] as u32;

          let old_r = frame[dst_index + 2] as u32;

          frame[dst_index] = ((b * a + old_b * inv_a) / 255) as u8;

          frame[dst_index + 1] = ((g * a + old_g * inv_a) / 255) as u8;

          frame[dst_index + 2] = ((r * a + old_r * inv_a) / 255) as u8;
        }
      }
    }
  }

  pub fn capture(&mut self) -> Result<&[u8], Box<dyn std::error::Error>> {
    self
      .conn
      .shm_get_image(
        self.root,
        self.x,
        self.y,
        self.width,
        self.height,
        !0,
        2, // Z_PIXMAP
        self.shmseg,
        0,
      )?
      .reply()?;

    // Raw X11 framebuffer.
    let pixels = unsafe { std::slice::from_raw_parts(self.shmaddr as *const u8, self.size) };

    // Copy X11 framebuffer into our writable BGRX buffer.
    self.frame.copy_from_slice(pixels);

    // Get current cursor image and position.
    self.update_cursor()?;

    // Draw cursor onto BGRX framebuffer.
    //
    // The result is still BGRX, so TurboJPEG
    // can consume it directly.
    let mut frame = std::mem::take(&mut self.frame);

    self.draw_cursor(&mut frame);

    self.frame = frame;

    Ok(&self.frame)
  }
}

impl Drop for X11Capture {
  fn drop(&mut self) {
    let _ = self.conn.shm_detach(self.shmseg);

    unsafe {
      shmdt(self.shmaddr);
    }
  }
}
