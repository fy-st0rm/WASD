# WASD (Web As Secondary Display)

WASD is a personal/experimental project that lets you use a Browser as a secondary display for Linux.

It captures an X11 virtual display and streams it to the phone through a web browser.

## Usage

1. Make sure VKMS is loaded.
2. Configure `wasd.toml`:

   ```toml
   [display]
   width = 1024
   height = 768
   direction = "right"
   rotation = 90

   [server]
   port = 8080
   ```

3. Run:

   ```bash
   cargo run --release
   ```

4. Open the displayed address on your Android device.

The phone and Linux machine must be on the same network.

## Requirements

- Linux + X11
- Rust
- xrandr
- VKMS kernel module
- XFixes + MIT-SHM
- nasm for TurboJPEG

On Arch Linux:

```bash
sudo pacman -S xorg-xrandr nasm
```

## Caveats

- Currently X11 only; Wayland is not supported.
- VKMS must be available and loaded.
- The physical display is currently hard-coded to `eDP-1`.
- The virtual display must use an existing xrandr mode.
- Rotation is handled by the browser.
- Android currently uses a browser; there is no native app yet.
- Streaming currently uses JPEG over WebSocket.
- Primarily built for personal use and experimentation, so expect rough edges.
