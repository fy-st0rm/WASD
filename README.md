# WASD

**WASD (Web As Secondary Display)** is a personal/experimental project that lets you use an Android phone as a secondary display for Linux.

It creates a virtual monitor using **VKMS**, captures the virtual display through X11, and streams it to an Android device through a web browser.

## Requirements

- Linux
- X11
- i3 window manager
- Rust
- `xrandr`
- VKMS kernel module
- XFixes
- MIT-SHM
- `nasm` for TurboJPEG

### Arch Linux

Install the required packages:

```bash
sudo pacman -S xorg-xrandr nasm
```

Make sure Rust is installed as well.

## VKMS

WASD requires the VKMS kernel module.

Check whether it is already loaded:

```bash
lsmod | grep vkms
```

If nothing is returned, load it:

```bash
sudo modprobe vkms
```

You can verify that the virtual display exists with:

```bash
xrandr --query
```

You should see an output similar to:

```
Virtual-1-1 connected
```

### Load VKMS automatically

If you don't want to run `modprobe vkms` after every reboot, create:

```bash
sudo nano /etc/modules-load.d/vkms.conf
```

Add:

```
vkms
```

After reboot, VKMS will be loaded automatically.

## Configuration

WASD stores its configuration at:

```
~/.config/wasd/wasd.toml
```

If the configuration file does not exist, WASD automatically creates a default one on the first run.

Example:

```toml
[display]
width = 1366
height = 768

# left, right, top, bottom
direction = "bottom"

# 0, 90, 180, 270
rotation = 90

# i3 workspace that will be moved to the virtual monitor
workspace = 10

[server]
port = 8080
```

### Display options

- **width / height** — resolution of the virtual display.
- **direction** — position of the virtual display relative to your primary monitor.
- **rotation** — rotation applied by the browser on the Android device.
- **workspace** — i3 workspace that WASD moves to the virtual monitor.

The resolution must be an existing mode supported by the VKMS virtual output.

## Installing

Clone the repository and build/install WASD:

```bash
cargo install --path .
```

This installs the `wasd` executable into:

```
~/.cargo/bin/
```

Make sure `~/.cargo/bin` is in your `PATH`.

After installation, you can run WASD from anywhere:

```bash
wasd
```

If you make changes to the source code, reinstall it with:

```bash
cargo install --path .
```

## Running

Start WASD:

```bash
wasd
```

On startup, WASD will:

1. Find the VKMS virtual display.
2. Detect your primary physical monitor automatically.
3. Configure the virtual display using your configuration.
4. Move the configured i3 workspace to the virtual display.
5. Start the streaming server.

WASD will print an address similar to:

```
WASD server: http://192.168.1.100:8080
```

## Connecting an Android device

Make sure your Android device and Linux computer are connected to the same network.

Open the address printed by WASD in your Android browser:

```
http://192.168.1.100:8080
```

For example:

```
http://192.168.1.100:8080
```

Enter fullscreen mode from the WASD interface to use the phone as a display.

## Shutdown

Press:

```
Ctrl+C
```

WASD will disable the virtual display before exiting.

The VKMS module itself remains loaded. This allows WASD to be started again without manually loading VKMS.

## Important

WASD currently supports **i3 only**.

WASD automatically moves a configured i3 workspace to the virtual monitor. Other window managers/desktops are currently not supported because the workspace-management functionality is implemented specifically for i3.

## Caveats

- Currently X11 only; Wayland is not supported.
- Currently i3 only.
- VKMS must be available and loaded.
- The primary physical monitor is detected automatically.
- The virtual monitor must use an existing xrandr mode.
- Rotation is handled by the browser.
- Android currently uses a web browser; there is no native Android application.
- Streaming currently uses JPEG frames over WebSocket.
- The project is primarily intended for personal use and experimentation, so expect rough edges.
