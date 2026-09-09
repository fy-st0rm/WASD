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
  // XFixes serial — increments on every cursor shape change.
  cursor_serial: u32,
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
      cursor_serial: u32::MAX, // force first fetch
    })
  }

  fn update_cursor(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    let cursor = self.conn.xfixes_get_cursor_image()?.reply()?;

    // Skip expensive pixel copy when the cursor shape hasn't changed
    // (only position/hotspot may differ).
    if cursor.cursor_serial != self.cursor_serial {
      self.cursor_serial = cursor.cursor_serial;
      self.cursor = Some(Cursor {
        x: cursor.x,
        y: cursor.y,
        width: cursor.width,
        height: cursor.height,
        xhot: cursor.xhot,
        yhot: cursor.yhot,
        pixels: cursor.cursor_image,
      });
    } else if let Some(ref mut c) = self.cursor {
      // Shape unchanged — only update position.
      c.x = cursor.x;
      c.y = cursor.y;
    }

    Ok(())
  }

  fn draw_cursor_inplace(&mut self) {
    let cursor = match &self.cursor {
      Some(cursor) => cursor,
      None => return,
    };

    // Convert global X11 cursor coordinates into
    // coordinates relative to our capture rectangle.
    let cursor_x = cursor.x as i32 - self.x as i32 - cursor.xhot as i32;
    let cursor_y = cursor.y as i32 - self.y as i32 - cursor.yhot as i32;

    let cap_w = self.width as i32;
    let cap_h = self.height as i32;
    let cur_w = cursor.width as i32;

    for cy in 0..cursor.height as i32 {
      let dst_y = cursor_y + cy;
      if dst_y < 0 || dst_y >= cap_h {
        continue;
      }

      for cx in 0..cur_w {
        let dst_x = cursor_x + cx;
        if dst_x < 0 || dst_x >= cap_w {
          continue;
        }

        let src_index = (cy * cur_w + cx) as usize;
        let pixel = cursor.pixels[src_index];

        let a = ((pixel >> 24) & 0xff) as u32;
        if a == 0 {
          continue;
        }

        let r = ((pixel >> 16) & 0xff) as u32;
        let g = ((pixel >> 8) & 0xff) as u32;
        let b = (pixel & 0xff) as u32;

        let dst_index = ((dst_y * cap_w + dst_x) * 4) as usize;

        // Our framebuffer is BGRX.
        if a == 255 {
          self.frame[dst_index] = b as u8;
          self.frame[dst_index + 1] = g as u8;
          self.frame[dst_index + 2] = r as u8;
        } else {
          let inv_a = 255 - a;
          let old_b = self.frame[dst_index] as u32;
          let old_g = self.frame[dst_index + 1] as u32;
          let old_r = self.frame[dst_index + 2] as u32;
          self.frame[dst_index] = ((b * a + old_b * inv_a) / 255) as u8;
          self.frame[dst_index + 1] = ((g * a + old_g * inv_a) / 255) as u8;
          self.frame[dst_index + 2] = ((r * a + old_r * inv_a) / 255) as u8;
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

    // Raw X11 framebuffer lives in shared memory — copy into our writable buffer.
    let pixels = unsafe { std::slice::from_raw_parts(self.shmaddr as *const u8, self.size) };
    self.frame.copy_from_slice(pixels);

    // Update cursor (skips pixel re-fetch when shape serial is unchanged).
    self.update_cursor()?;

    // Draw cursor onto BGRX framebuffer.
    self.draw_cursor_inplace();

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
