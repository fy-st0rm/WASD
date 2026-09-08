use std::{
    ffi::c_void,
    ptr,
    time::{Duration, Instant},
};

use libc::{
    shmctl, shmat, shmget,
    IPC_CREAT, IPC_EXCL, IPC_PRIVATE, IPC_RMID,
    S_IRUSR, S_IWUSR,
};

use x11rb::{
    connection::Connection,
    protocol::shm::ConnectionExt as ShmConnectionExt,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ------------------------------------------------------------
    // X11
    // ------------------------------------------------------------

    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];

    let x = 1366;
    let y = 0;
    let width = 1024u16;
    let height = 768u16;

    let size = width as usize * height as usize * 4;

    println!(
        "Screen: {}x{}",
        screen.width_in_pixels,
        screen.height_in_pixels
    );

    println!("Capture: {}x{}", width, height);
    println!("Shared memory size: {} bytes", size);

    // ------------------------------------------------------------
    // System V shared memory
    // ------------------------------------------------------------

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

    // Attach shared memory to our process.
    let shmaddr = unsafe { shmat(shmid, ptr::null(), 0) };

    if shmaddr == (-1isize) as *mut c_void {
        unsafe {
            shmctl(shmid, IPC_RMID, ptr::null_mut());
        }

        return Err(std::io::Error::last_os_error().into());
    }

    // Mark for deletion.
    //
    // The memory remains alive until all processes detach from it.
    unsafe {
        shmctl(shmid, IPC_RMID, ptr::null_mut());
    }

    // Create X11 SHM segment ID.
    let shmseg = conn.generate_id()?;

    conn.shm_attach(shmseg, shmid as u32, false)?.check()?;

    println!("MIT-SHM attached successfully");

    // ------------------------------------------------------------
    // Temporary RGBA buffer
    // ------------------------------------------------------------

    let mut rgba = vec![0u8; size];

    // ------------------------------------------------------------
    // 60 FPS
    // ------------------------------------------------------------

    let frame_time = Duration::from_secs_f64(1.0 / 60.0);

    let mut frames = 0;
    let mut fps_timer = Instant::now();

    // ------------------------------------------------------------
    // Main loop
    // ------------------------------------------------------------

    loop {
        let start = Instant::now();

        // --------------------------------------------------------
        // Capture X11 framebuffer into SHM
        // --------------------------------------------------------

        conn.shm_get_image(
            screen.root,
            x,
            y,
            width,
            height,
            !0,
            2, // Z_PIXMAP
            shmseg,
            0,
        )?
        .reply()?;

        // --------------------------------------------------------
        // Read pixels from SHM
        // --------------------------------------------------------

        let pixels = unsafe {
            std::slice::from_raw_parts(
                shmaddr as *const u8,
                size,
            )
        };

        // --------------------------------------------------------
        // X11 pixel format -> RGBA
        //
        // X11 root:
        //   red   = 0x00ff0000
        //   green = 0x0000ff00
        //   blue  = 0x000000ff
        //
        // On our machine each pixel occupies 4 bytes.
        // --------------------------------------------------------

        for (src, dst) in pixels
            .chunks_exact(4)
            .zip(rgba.chunks_exact_mut(4))
        {
            let value = u32::from_ne_bytes([
                src[0],
                src[1],
                src[2],
                src[3],
            ]);

            let r = ((value & 0x00ff0000) >> 16) as u8;
            let g = ((value & 0x0000ff00) >> 8) as u8;
            let b = (value & 0x000000ff) as u8;

            dst[0] = b;
            dst[1] = g;
            dst[2] = r;
            dst[3] = 255;
        }

        // --------------------------------------------------------
        // FPS counter
        // --------------------------------------------------------

        frames += 1;

        if fps_timer.elapsed() >= Duration::from_secs(1) {
            println!("FPS: {}", frames);

            frames = 0;
            fps_timer = Instant::now();
        }

        // --------------------------------------------------------
        // 60 FPS limiter
        // --------------------------------------------------------

        let elapsed = start.elapsed();

        if elapsed < frame_time {
            std::thread::sleep(frame_time - elapsed);
        }
    }
}
