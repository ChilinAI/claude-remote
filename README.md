# Claude Remote

Control [Claude Code](https://claude.ai/claude-code) on your Mac from any browser or phone.

Send coding tasks remotely — get AI-powered responses streamed back in real time.

![Claude Remote — control Claude Code from your phone](web/public/assets/hero.jpg)

## How It Works

1. **Install the app** on your Mac — it lives in the menu bar
2. **Open the [web chat](https://clauderemote.web.app)** from your phone or any browser
3. **Send a message** — it reaches your Mac instantly, Claude Code runs the task and streams back the result

## Desktop App

Lightweight Tauri tray app with neumorphic UI, auto-detect Claude Code path, and live connection status.

![Claude Remote App](web/public/assets/screenshot-app.png)

## Features

- **Mobile access** — full chat with Claude Code from iPhone, Android, or [any browser](https://clauderemote.web.app)
- **Instant delivery** — Firebase Realtime Database, messages delivered in milliseconds
- **&#128274; End-to-end encrypted** — all messages are encrypted in the browser and decrypted only on your Mac (see below)
- **Lightweight** — just an icon in your menu bar, minimal resource usage
- **Auto-updates** — built-in updater checks for new versions on launch

## &#128274; End-to-End Encryption

All communication between the web chat and your Mac is end-to-end encrypted. Messages stored in Firebase are ciphertext — **the server never sees your data in plain text**.

### How it works

1. **Key exchange** — when you open a chat session, the browser generates an [ECDH P-256](https://en.wikipedia.org/wiki/Elliptic-curve_Diffie%E2%80%93Hellman) key pair and publishes the public key to Firebase. The desktop app does the same. Both sides compute an identical shared secret without ever transmitting it.

2. **Encryption** — every message you type is encrypted with [AES-256-GCM](https://en.wikipedia.org/wiki/Galois/Counter_Mode) using the shared key and a unique random IV (nonce) before leaving the browser. Only the ciphertext + IV are stored in Firebase.

3. **Decryption** — the desktop app decrypts incoming messages, sends them to Claude Code, encrypts the response, and writes it back. The browser decrypts the response and renders it.

4. **Session isolation** — each chat session has its own key pair. Messages from other sessions or browsers appear blurred and unreadable.

```
Browser                    Firebase RTDB                  Mac (Tauri)
  |                             |                            |
  |-- ECDH public key --------->|                            |
  |                             |<------ ECDH public key ----|
  |                             |                            |
  |  (derive shared AES key)    |    (derive shared AES key) |
  |                             |                            |
  |-- AES-GCM(message) ------->|------- ciphertext -------->|
  |                             |                            |  decrypt -> Claude Code
  |                             |                            |  encrypt response
  |<-------- ciphertext --------|<-- AES-GCM(response) -----|
  |  decrypt & render           |                            |
```

**What Firebase sees:** only public keys (safe to share) and encrypted blobs. Private keys never leave the device.

## Download

**[Download Claude Remote for macOS (Apple Silicon)](https://clauderemote.web.app/releases/Claude%20Remote_0.3.1_aarch64.dmg)**

### Installation

macOS may show a warning for apps downloaded outside the App Store:

1. Open the `.dmg` and drag Claude Remote to Applications
2. Try to open the app — macOS will block it
3. Go to **System Settings → Privacy & Security**, scroll down and click **Open Anyway**
4. Enter your password — done! The app opens normally from now on

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Desktop app | [Tauri v2](https://v2.tauri.app/) (Rust + HTML/JS) |
| Backend | Firebase Realtime Database |
| Auth | Firebase Authentication |
| Web chat | Vanilla JS + marked.js + highlight.js |
| Hosting | Firebase Hosting |
| Auto-update | Tauri updater (minisign) |

## Project Structure

```
claude-remote/
  app/                    # Tauri desktop application
    src/                  # Frontend (HTML/JS settings window)
    src-tauri/            # Rust backend (auth, daemon, tray)
  web/                    # Firebase Hosting
    public/
      index.html          # Landing page (EN)
      ru/index.html       # Landing page (RU)
      chat.html           # Web chat
      releases/           # DMG downloads + update manifest
```

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/)
- [Node.js](https://nodejs.org/) (v18+)
- [Claude Code](https://claude.ai/claude-code) installed on your Mac

### Build

```bash
cd app
npm install
npm run tauri build
```

The built `.app` and `.dmg` will be in `app/src-tauri/target/release/bundle/`.

## Links

- **Download:** [Claude Remote for macOS](https://clauderemote.web.app/releases/Claude%20Remote_0.3.1_aarch64.dmg)
- **Website:** [clauderemote.web.app](https://clauderemote.web.app)
- **Web chat:** [clauderemote.web.app/chat.html](https://clauderemote.web.app/chat.html)

## License

MIT
