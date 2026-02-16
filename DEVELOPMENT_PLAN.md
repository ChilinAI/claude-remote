# Claude Remote — Development Plan

## Architecture

```
┌──────────────────────────────────────────────────┐
│  Firebase (project: chilin1)                      │
│                                                    │
│  Realtime DB        Auth           Hosting         │
│  /sessions/{uid}/   email/pass     web chat +      │
│    messages/                       landing +        │
│    _heartbeat/                     releases         │
└───────┬───────────────────────────────┬────────────┘
        │ HTTP polling                  │ HTTPS
        │                               │
┌───────▼────────┐              ┌───────▼────────┐
│  Tauri App     │              │  iPhone/Web    │
│  (user's Mac)  │              │  browser       │
│                │              │                │
│  Rust:         │              │  Chat UI       │
│  - auth        │              │  (JS + RTDB)   │
│  - poll daemon │              │                │
│  - spawn claude│              │                │
│  - tray icon   │              │                │
│  - auto-update │              │                │
└────────────────┘              └────────────────┘
```

## Completed

### Phase 1 — Firebase + Web
- [x] Firebase project (chilin1): RTDB, Auth, Hosting
- [x] RTDB security rules (user can only access own sessions)
- [x] Landing page (EN + RU) with neumorphic design
- [x] Web chat with markdown rendering (marked.js + highlight.js)
- [x] Installation instructions on landing page

### Phase 2 — Tauri App (MVP)
- [x] Tauri v2 scaffold (Rust + HTML/JS)
- [x] System tray icon with status menu
- [x] Firebase Auth (REST API from Rust): login, register, session restore
- [x] RTDB polling daemon (2 sec interval)
- [x] Claude Code integration (spawn `claude -p`, env setup)
- [x] Settings window (Claude path with auto-detect, working directory)
- [x] Heartbeat (30 sec interval, status in RTDB)
- [x] Auto-start daemon on app launch (after session restore)
- [x] Auto token refresh on 401

### Phase 3 — Distribution
- [x] DMG build via Tauri bundler (aarch64)
- [x] Ad-hoc code signing (no Apple Developer subscription needed)
- [x] Auto-update via Tauri updater plugin (minisign)
- [x] Update manifest on Firebase Hosting
- [x] Published on GitHub (open-source)

## Future Improvements

### Web Chat
- [ ] Session history (sidebar)
- [ ] Delete sessions
- [ ] Response streaming
- [ ] Quick actions (configurable buttons)
- [ ] Search chat history
- [ ] Auto-title sessions

### Desktop App
- [ ] Multiple working directories (project switching)
- [ ] macOS notifications on response
- [ ] Message queue (process accumulated while offline)
- [ ] LaunchAgent for auto-start on boot
- [ ] x86_64 build (Intel Macs)

### Security
- [ ] PIN code for dangerous commands (rm, git push, etc.)
- [ ] Command whitelist/blacklist
- [ ] 2FA for login
