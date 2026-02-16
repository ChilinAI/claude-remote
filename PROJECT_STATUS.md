# Claude Remote — Статус разработки

## Что это
Сервис для удалённого управления Claude Code с iPhone/браузера. Пользователь ставит Mac-приложение (Tauri), которое связывает веб-чат с локальным Claude Code через Firebase Realtime Database.

## Что сделано

### 1. Прототип (claude-chat-bridge) — РАБОТАЕТ
- Путь: `/Users/aleksandr/claude-chat-bridge/`
- Веб-чат на Firebase Hosting: https://claude-chat-bridge.web.app
- Node.js демон (`daemon.js`) слушает Firestore, вызывает `claude -p`
- Использует существующий Firebase проект `mail-firestore`
- Firebase Auth для тестов (credentials не публикуются)
- Markdown рендеринг ответов (marked.js + highlight.js)
- **Важные находки при разработке:**
  - Claude Code не запускается внутри другой сессии — нужно `CLAUDECODE=''`
  - `--print` зависает в spawn — использовать `-p`
  - Нужно `stdin.end()` при spawn, иначе claude ждёт ввода
  - `CLAUDE_CONFIG_DIR` нужен для выбора аккаунта (account1/account2)

### 2. Tauri приложение (MVP) — РАБОТАЕТ ✅
- Путь: `/Users/aleksandr/claude-remote/app/`
- Запуск: `cd app && PATH="$HOME/.local/node/bin:$HOME/.cargo/bin:/usr/bin:/bin:$PATH" npm run tauri dev`

#### Что работает:
- **Полный цикл** — web → RTDB → Tauri → Claude Code → RTDB → web ✅
- **Firebase проект chilin1** (отдельный аккаунт, не mail-firestore)
  - API Key: `AIzaSyCxV6rBIk88Ur7qDMknibWZYs2D5zmVoFI`
  - RTDB URL: `https://chilin1-default-rtdb.europe-west1.firebasedatabase.app`
  - Auth UID: `oh4yEDmKCCPt3sHvcL0NlxQbNpl2`
- **Tray icon** — SF Symbol `macbook.and.iphone`, белая 42x42, клик открывает настройки
- **Окно настроек** — неоморфизм дизайн, светлая/тёмная тема (auto по macOS):
  - Header: заголовок + статус + Start/Stop + Quit
  - Account (login/register через Firebase Auth REST API)
  - Settings (Claude Code path с auto-detect, Working Directory)
  - Log (растягивается на всё свободное пространство)
- **Закрытие окна** — скрывает вместо закрытия, открывается через tray
- **Tray menu** — правый клик: Settings, Quit
- **Rust backend:**
  - Firebase Auth (REST API) — login, register
  - Claude Code detection (поиск бинарника, auto-detect при старте)
  - RTDB polling daemon (каждые 2 сек, перебирает все сессии пользователя)
  - Claude Code spawn (`-p` flag, `stdin: null`, `CLAUDECODE=''`)
  - Config management (working_dir, claude_path)
  - Quit command

### 3. Веб-чат (chilin1) — РАБОТАЕТ ✅
- Путь: `/Users/aleksandr/claude-remote/web/`
- Задеплоен: **https://chilin1.web.app**
- Firebase RTDB (не Firestore как в прототипе)
- Markdown рендеринг (marked.js + highlight.js)
- Структура RTDB: `/sessions/{uid}/{sessionId}/messages/{msgId}`
- RTDB Rules: авторизованный пользователь читает/пишет только свои данные

### 4. Firebase (проект chilin1)
- Отдельный Google аккаунт (chilin1), не смешан с mail-firestore
- RTDB: `https://chilin1-default-rtdb.europe-west1.firebasedatabase.app`
- Auth: Email/Password включён
- Hosting: https://chilin1.web.app
- Firebase CLI авторизован локально: `PATH="$HOME/.local/node/bin:$PATH" firebase`
- Деплой: `PATH="$HOME/.local/node/bin:/usr/bin:/bin:$PATH" firebase deploy --only hosting --project chilin1 --config /Users/aleksandr/claude-remote/web/firebase.json`

## Технический стек

| Компонент | Технология | Версия |
|-----------|-----------|--------|
| Node.js (хост) | Бинарник в `~/.local/node/` | v20.18.1 |
| npm | Исправлен путь к cli.js | v10.8.2 |
| Rust | rustup | 1.93.1 |
| Cargo | | 1.93.1 |
| Tauri | v2 | 2.10.2 |
| Firebase CLI | Локально `~/.local/node/bin/firebase` | 15.6.0 |

## Структура файлов

```
claude-remote/
  DEVELOPMENT_PLAN.md          # Полный план разработки (4 фазы)
  PROJECT_STATUS.md             # Этот файл
  gen-icons.js                  # Генерация app иконок (node-canvas)
  app/
    package.json
    src/
      index.html                # UI окна настроек (neumorphism, auto dark/light)
    src-tauri/
      Cargo.toml                # Зависимости: tauri, reqwest, tokio, serde, dirs, png, tauri-plugin-updater
      tauri.conf.json           # Конфиг: окно 576x480, visible:false
      src/
        lib.rs                  # Весь Rust код (auth, daemon, claude runner, tray, quit)
        main.rs                 # Entry point
      icons/
        tray.png                # Tray icon: чёрная "C" 32x32
        32x32.png, 128x128.png  # App иконки (красный фон, крупная белая "C", Arial bold)
        icon.png                # 1024x1024 base icon
        icon.icns               # macOS app icon (из icon.png через iconutil)
  web/
    firebase.json               # Firebase Hosting конфиг
    .firebaserc                 # Проект chilin1
    public/
      index.html                # Веб-чат (RTDB, markdown, dark theme)
      releases/
        latest.json             # Update manifest для Tauri updater
        Claude Remote.app.tar.gz # Updater артефакт (подписанный)
        Claude Remote_0.1.0_aarch64.dmg  # Установщик
```

## Конфигурация macOS

```bash
# ~/.zshrc
export PATH="$HOME/.local/bin:$HOME/.local/node/bin:$PATH"
alias claude1="CLAUDE_CONFIG_DIR=~/.claude-account1 claude"
alias claude2="CLAUDE_CONFIG_DIR=~/.claude-account2 claude"
```

- Claude Code установлен в `~/.local/bin/claude`
- Два аккаунта: account1 (основной/текущая сессия), account2 (для демона)
- Rust установлен через rustup в `~/.cargo/`

## Следующие шаги

1. ~~Создать новый Firebase проект с Realtime Database~~ ✅
2. ~~Прописать API key и DB URL в Tauri приложение~~ ✅
3. ~~Протестировать полный цикл: web → RTDB → Tauri → Claude Code → RTDB → web~~ ✅
4. ~~Веб-чат для нового проекта~~ ✅
5. ~~Сохранение конфига на диск (JSON файл)~~ ✅
6. ~~Сохранение auth credentials (refresh token)~~ ✅
   - Config: `~/Library/Application Support/claude-remote/config.json`
   - Session: `~/Library/Application Support/claude-remote/session.json`
   - Rust: `restore_session` — загружает refresh_token, обменивает на id_token через Firebase
   - Rust: `logout` — очищает state + удаляет session.json
   - Daemon: автоматический refresh при 401
   - Фронт: при init восстанавливает сессию
7. ~~Сборка .dmg release~~ ✅
   - Bundle: `app/src-tauri/target/release/bundle/dmg/Claude Remote_0.1.0_aarch64.dmg`
   - App: `app/src-tauri/target/release/bundle/macos/Claude Remote.app`
   - Identifier: `com.clauderemote.desktop`
8. ~~Auto-update~~ ✅
   - Tauri updater plugin с подписью (minisign)
   - Ключи: `~/.tauri/claude-remote.key` (не публикуется)
   - Update manifest: `https://chilin1.web.app/releases/latest.json`
   - Артефакты: `https://chilin1.web.app/releases/Claude Remote.app.tar.gz`
   - Сборка: `TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/claude-remote.key)" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<password>" npm run tauri build`
   - Rust: `check_for_updates` command с автоустановкой
9. Веб-чат: улучшения (history сессий, удаление, streaming)
