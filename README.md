# Reverb Desktop

The native desktop app for your self-hosted [Reverb](https://github.com/BubbyWoodz/swingmusic) music server. Built with [Tauri v2](https://tauri.app).

## What it is

A thin native shell around the Reverb web client. The webview loads your Reverb server (entered once on first run); this app adds the native layer that makes it feel like a real desktop app:

- **Native window** — real title bar, taskbar/dock presence, no browser chrome
- **Global media keys** — play/pause, next, previous, stop work system-wide (Windows/Linux; macOS may need accessibility permission)
- **System tray** — Play/Pause, Next/Previous, Start-on-login toggle, Change server, Quit. Left-click shows the window; closing the window hides to tray (Spotify-style)
- **Native notifications** — track-change notifications via the OS
- **Single instance** — launching again focuses the running app
- **Server memory** — your server URL is saved; reachable servers connect automatically

## Connect sync

The webview runs the full Reverb web client, so [Connect sync](https://github.com/BubbyWoodz/swingmusic) (per-user playback state, device handoff, remote control) works out of the box — control this desktop player from your phone and vice versa.

## Development

Prerequisites: Rust (stable), Node 20+.

```bash
npm install
npm run tauri dev
```

## Building

```bash
npm run tauri build
```

Installers land in `src-tauri/target/release/bundle/`. GitHub Actions builds Windows (NSIS), macOS (Apple Silicon + Intel DMGs), and Linux (AppImage + deb) on every push to `main`.

## Platform notes

- **Windows**: media keys, tray, notifications, autostart all supported.
- **macOS**: media-key capture can require accessibility permission; tray becomes a menu-bar icon.
- **Linux**: requires WebKitGTK; tray visibility depends on the desktop environment (a menu is always set so it appears where supported).
