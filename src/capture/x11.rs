use std::{ffi::c_void, ptr};

use libc::{
  IPC_CREAT, IPC_EXCL, IPC_PRIVATE, IPC_RMID, S_IRUSR, S_IWUSR, shmat, shmctl, shmdt, shmget,
};

use x11rb::{connection::Connection, protocol::shm::ConnectionExt as ShmConnectionExt};

pub struct X11Capture {
  conn: x11rb::rust_connection::RustConnection,
  root: u32,

  shmseg: u32,
  shmaddr: *mut c_void,

  pub width: u16,
  pub height: u16,

  size: usize,
}

impl X11Capture {
  pub fn new(x: i16, y: i16, width: u16, height: u16) -> Result<Self, Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen_num].root;

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

    // Mark the segment for deletion.
    unsafe {
      shmctl(shmid, IPC_RMID, ptr::null_mut());
    }

    let shmseg = conn.generate_id()?;

    conn.shm_attach(shmseg, shmid as u32, false)?.check()?;

    println!("MIT-SHM attached successfully");

    Ok(Self {
      conn,
      root,

      shmseg,
      shmaddr,

      width,
      height,

      size,
    })
  }

  pub fn capture(&self) -> Result<&[u8], Box<dyn std::error::Error>> {
    self
      .conn
      .shm_get_image(
        self.root,
        0,
        0,
        self.width,
        self.height,
        !0,
        2, // Z_PIXMAP
        self.shmseg,
        0,
      )?
      .reply()?;

    let pixels = unsafe { std::slice::from_raw_parts(self.shmaddr as *const u8, self.size) };

    Ok(pixels)
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
