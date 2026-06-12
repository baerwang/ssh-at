# SSH-AT

[中文文档](./README-zh.md)

## Overview

**SSH-AT** is a desktop GUI tool for managing SSH keys, hosts, and configurations. Generate keys, edit `~/.ssh/config` visually, and keep automatic backups—all in one app.

Built with Tauri 2, React, and Rust for a native cross-platform experience.

## Features

- **📝 SSH Config Management**
  - Visual editor for `~/.ssh/config`
  - Add, edit, delete, and search SSH hosts
  - Real-time syntax highlighting with Monaco Editor
  - Automatic backup before every save

- **🔑 SSH Key Management**
  - Generate SSH keys (RSA, Ed25519, ECDSA)
  - View key fingerprints
  - One-click copy public keys to clipboard
  - Delete keys with confirmation

- **💾 Backup & Restore**
  - Automatic timestamped backups
  - Browse and restore previous configs
  - Delete old backups

- **🌍 Multi-language Support**
  - English / 中文
  - Auto-detect system language

- **🎨 Modern UI**
  - Material Design with MUI
  - Light/Dark theme toggle
  - System tray integration (minimize to tray, not quit)
  - macOS Dock icon auto-hide when minimized

## Screenshots

### Hosts Management
![Hosts Management](./screenshots/hosts.png)

### SSH Keys
![SSH Keys](./screenshots/keys.png)

### Config Editor
![Config Editor](./screenshots/editor.png)

### Backups
![Backups](./screenshots/backups.png)

### Settings
![Settings](./screenshots/settings.png)

## Tech Stack

**Frontend:**
- React 19 + TypeScript
- Material-UI (MUI)
- Monaco Editor
- React Router
- React Query
- i18next

**Backend:**
- Rust
- Tauri 2
- Tokio (async runtime)
- Custom SSH config parser

**Build:**
- Vite
- pnpm

## Installation

### Download Pre-built Binaries

Go to [Releases](https://github.com/baerwang/ssh-at/releases) and download:

- **macOS**: `.dmg` (Universal binary for Intel & Apple Silicon)
- **Windows**: `.msi`
- **Linux**: `.AppImage` or `.deb`

#### Build from Source

**Prerequisites:**
- Node.js 20+
- pnpm 8+
- Rust 1.70+
- Platform-specific dependencies:
  - **macOS**: Xcode Command Line Tools, `create-dmg` (`brew install create-dmg`)
  - **Linux**: `libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf`
  - **Windows**: WebView2 (usually pre-installed on Windows 10+)

**Build Steps:**

```bash
# Clone the repository
git clone https://github.com/baerwang/ssh-at.git
cd ssh-at

# Install dependencies
pnpm install

# Development mode
pnpm tauri dev

# Production build
pnpm tauri build
```

Build outputs:
- macOS: `src-tauri/target/release/bundle/macos/SSH-AT.app` and `.dmg`
- Windows: `src-tauri/target/release/bundle/msi/*.msi`
- Linux: `src-tauri/target/release/bundle/appimage/*.AppImage` and `.deb`

## Usage

1. **Launch the app**
   - macOS: Drag `SSH-AT.app` to `/Applications` and launch
   - Windows: Run the `.msi` installer
   - Linux: Make `.AppImage` executable and run, or install `.deb`

2. **Manage SSH Hosts**
   - Navigate to **Hosts** tab
   - Click **+** to add a new host
   - Edit inline or use the **Config Editor** for raw editing

3. **Generate SSH Keys**
   - Go to **Keys** tab
   - Click **Generate Key**
   - Choose algorithm (Ed25519 recommended), name, passphrase
   - Keys are saved to `~/.ssh-at/creds/`

4. **Backups**
   - Visit **Backups** tab to view all automatic backups
   - Restore or delete as needed

5. **Settings**
   - Toggle theme (light/dark)
   - Switch language (English/中文)
   - Open config directory

## License

Apache 2.0
