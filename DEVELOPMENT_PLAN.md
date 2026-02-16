# Claude Remote — План разработки

## Описание продукта

Сервис для удалённого управления Claude Code с мобильного устройства или любого браузера.
Клиент устанавливает Mac-приложение (Tauri), которое связывает веб-чат с локальным Claude Code через Firebase Realtime Database.

## Архитектура

```
┌──────────────────────────────────────────────────┐
│  Firebase (проект: claude-remote)                 │
│                                                    │
│  Realtime DB        Auth           Hosting         │
│  /sessions/{uid}/   email/pass     веб-чат +       │
│    messages/                       лендинг +        │
│    status/                         регистрация      │
└───────┬───────────────────────────────┬────────────┘
        │ WebSocket                     │ HTTPS
        │                               │
┌───────▼────────┐              ┌───────▼────────┐
│  Tauri App     │              │  iPhone/Web    │
│  (Mac клиента) │              │  браузер       │
│                │              │                │
│  Rust:         │              │  Chat UI       │
│  - WS listener │              │  (JS + RTDB)   │
│  - spawn claude│              │                │
│  - tray icon   │              │                │
│  - auto-update │              │                │
└────────────────┘              └────────────────┘
```

## Структура Realtime Database

```
users/{uid}/
  profile: { email, createdAt }
  settings: { workingDir, claudeConfig }

sessions/{uid}/{sessionId}/
  meta: { title, createdAt, updatedAt }
  messages/{pushId}/
    role: "user" | "assistant"
    text: "..."
    status: "pending" | "processing" | "streaming" | "done"
    timestamp: serverTimestamp

presence/{uid}/
  online: true/false
  lastSeen: timestamp
  appVersion: "1.0.0"
```

## RTDB Security Rules

```json
{
  "rules": {
    "sessions": {
      "$uid": {
        ".read": "auth.uid === $uid",
        ".write": "auth.uid === $uid"
      }
    },
    "presence": {
      "$uid": {
        ".read": "auth.uid === $uid",
        ".write": "auth.uid === $uid"
      }
    },
    "users": {
      "$uid": {
        ".read": "auth.uid === $uid",
        ".write": "auth.uid === $uid"
      }
    }
  }
}
```

---

## Фазы разработки

### Фаза 1 — Firebase проект и веб-часть

**1.1 Создание Firebase проекта**
- Создать проект `claude-remote` в Firebase Console
- Включить Authentication (Email/Password)
- Включить Realtime Database
- Включить Hosting
- Задеплоить RTDB security rules

**1.2 Лендинг + регистрация**
- Страница: описание продукта, кнопка регистрации, скачать приложение
- Регистрация/логин через Firebase Auth
- После логина — редирект на чат
- Responsive дизайн (mobile-first)

**1.3 Веб-чат**
- Chat UI (на основе прототипа из claude-chat-bridge)
- Подключение к Realtime Database (вместо Firestore)
- Markdown рендеринг (marked.js + highlight.js)
- Индикатор статуса Mac-компьютера (online/offline из presence)
- Список сессий (sidebar на десктопе, drawer на мобильном)
- Кнопка "New Chat"
- Push-уведомления при получении ответа (опционально, FCM)

### Фаза 2 — Tauri приложение (MVP)

**2.1 Scaffold Tauri проекта**
- Инициализация Tauri + Rust
- Структура: src-tauri/ (Rust) + src/ (HTML/JS для окна настроек)

**2.2 System Tray**
- Иконка в трее macOS
- Статусы: зелёный (подключён), жёлтый (нет Claude Code), красный (ошибка)
- Меню: Settings, Status, Quit

**2.3 Аутентификация**
- Окно логина (Firebase Auth REST API из Rust)
- Сохранение токена в macOS Keychain
- Автоматический refresh токена

**2.4 Realtime Database listener**
- WebSocket подключение к RTDB из Rust
- Слушает `sessions/{uid}/` на новые сообщения со status=pending
- Обновляет presence/{uid} (online/offline, heartbeat)

**2.5 Claude Code integration**
- Обнаружение установленного Claude Code (поиск бинарника)
- Spawn `claude -p <prompt>` через std::process::Command
- Стриминг stdout → обновление сообщения в RTDB (status: streaming → done)
- Обработка ошибок (Claude не установлен, API лимит, etc.)

**2.6 Окно настроек (Settings)**
- Выбор рабочей директории (в которой Claude Code будет работать)
- Выбор Claude config (если несколько аккаунтов)
- Автозапуск при старте macOS (LaunchAgent)
- Логи / история последних запросов

### Фаза 3 — Сборка и дистрибуция

**3.1 Сборка .dmg**
- Tauri bundler → .dmg для macOS (arm64 + x86_64)
- Code signing (опционально, для первой версии без notarization)
- Размещение .dmg на Firebase Hosting (или GitHub Releases)

**3.2 Auto-update**
- Tauri built-in updater
- Update manifest на Firebase Hosting
- При запуске — проверка новой версии, автообновление

**3.3 Инструкция для клиента**
- Установка Claude Code (ссылка на anthropic.com)
- Скачивание и установка Claude Remote
- Логин → выбор рабочей директории → готово

### Фаза 4 — Улучшения

**4.1 Веб-чат**
- Quick actions — настраиваемые кнопки ("git status", "npm test", etc.)
- Поиск по истории чатов
- Названия сессий (auto-title из первого сообщения)
- Копирование кода одним тапом

**4.2 Tauri-приложение**
- Несколько рабочих директорий (переключение проектов)
- Уведомления macOS при получении сообщения
- Очередь сообщений (обработка накопившихся за время офлайн)
- Логирование всех команд

**4.3 Безопасность**
- PIN-код для опасных команд (rm, git push, etc.)
- Whitelist/blacklist команд
- 2FA для логина

**4.4 Подписка (когда будет готово)**
- Stripe интеграция
- Планы: Free (лимит сообщений), Pro (безлимит)
- Проверка подписки в Tauri при старте

---

## Стек технологий

| Компонент | Технология |
|-----------|------------|
| Backend/DB | Firebase Realtime Database |
| Auth | Firebase Authentication |
| Hosting | Firebase Hosting |
| Mac-приложение | Tauri (Rust + HTML/JS) |
| Веб-чат frontend | Vanilla JS + marked.js + highlight.js |
| Лендинг | HTML/CSS (на Firebase Hosting) |
| Дистрибуция | .dmg через Tauri bundler |
| Auto-update | Tauri updater |

## Структура проекта (файловая)

```
claude-remote/
  DEVELOPMENT_PLAN.md        # Этот файл
  firebase/
    firebase.json             # Firebase config
    database.rules.json       # RTDB rules
  web/                        # Firebase Hosting (лендинг + чат)
    public/
      index.html              # Лендинг
      chat.html               # Веб-чат
      assets/
        style.css
        chat.js
        auth.js
  app/                        # Tauri приложение
    src-tauri/
      Cargo.toml
      src/
        main.rs
        auth.rs
        rtdb.rs
        claude.rs
        tray.rs
    src/
      index.html              # Окно настроек
      settings.js
```

---

## Порядок реализации

1. ~~Tauri приложение (Фаза 2)~~ — ГОТОВО
2. ~~Firebase проект (Фаза 1.1)~~ — ГОТОВО (проект chilin1, RTDB, Auth)
3. ~~Сборка и дистрибуция (Фаза 3)~~ — ГОТОВО (PKG installer вместо DMG)
4. **Веб-часть: лендинг + чат (Фаза 1.2, 1.3)** ← СЛЕДУЮЩИЙ ШАГ
5. Улучшения (Фаза 4)

## Текущий статус

- [x] Прототип проверен (claude-chat-bridge) — работает
- [x] Tauri приложение (MVP) — auth, tray, polling daemon, settings, auto-update
- [x] Firebase проект — chilin1 (RTDB + Auth + Hosting)
- [x] Сборка и дистрибуция — PKG installer (`npm run build`), postinstall открывает app
- [ ] Веб-часть (лендинг + чат)
- [ ] Улучшения
