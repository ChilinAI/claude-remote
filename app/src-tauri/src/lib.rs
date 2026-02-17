use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::Arc;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager, State,
};
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use p256::{ecdh::EphemeralSecret, EncodedPoint, PublicKey};
use rand::rngs::OsRng;

// === App State ===

#[derive(Default, Serialize, Deserialize, Clone)]
struct AppConfig {
    working_dir: String,
    claude_path: String,
    firebase_api_key: String,
    firebase_db_url: String,
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct SavedSession {
    email: String,
    uid: String,
    refresh_token: String,
}

#[derive(Default)]
struct AppState {
    auth_token: Mutex<Option<String>>,
    uid: Mutex<Option<String>>,
    email: Mutex<Option<String>>,
    refresh_token: Mutex<Option<String>>,
    config: Mutex<AppConfig>,
    running: Mutex<bool>,
    busy: Mutex<bool>,
}

// === E2E Encryption State ===
// Per-session ECDH keys and derived AES key
// HashMap<session_id, AES key bytes>

struct CryptoState {
    // session_id -> (AES-256 key bytes, browser_pub_key_b64 used to derive)
    session_keys: Mutex<std::collections::HashMap<String, ([u8; 32], String)>>,
}

impl Default for CryptoState {
    fn default() -> Self {
        Self {
            session_keys: Mutex::new(std::collections::HashMap::new()),
        }
    }
}

fn make_cipher(key: &[u8; 32]) -> Aes256Gcm {
    Aes256Gcm::new_from_slice(key).unwrap()
}

fn encrypt_message(cipher: &Aes256Gcm, plaintext: &str) -> Result<(String, String), String> {
    let iv_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&iv_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption error: {}", e))?;
    Ok((B64.encode(&ciphertext), B64.encode(&iv_bytes)))
}

fn decrypt_message(cipher: &Aes256Gcm, ciphertext_b64: &str, iv_b64: &str) -> Result<String, String> {
    let ciphertext = B64.decode(ciphertext_b64).map_err(|e| format!("Base64 decode error: {}", e))?;
    let iv_bytes = B64.decode(iv_b64).map_err(|e| format!("IV decode error: {}", e))?;
    let nonce = Nonce::from_slice(&iv_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("Decryption error: {}", e))?;
    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 error: {}", e))
}

/// Generate ECDH keypair, return (secret, public_key_base64)
fn generate_ecdh_keypair() -> (EphemeralSecret, String) {
    let secret = EphemeralSecret::random(&mut OsRng);
    let public_key = EncodedPoint::from(secret.public_key());
    let pub_b64 = B64.encode(public_key.as_bytes());
    (secret, pub_b64)
}

/// Derive AES-256 key bytes from our secret + browser's public key
fn derive_aes_key(secret: EphemeralSecret, browser_pub_b64: &str) -> Result<[u8; 32], String> {
    let pub_bytes = B64.decode(browser_pub_b64).map_err(|e| format!("Base64 decode: {}", e))?;
    let browser_pub = PublicKey::from_sec1_bytes(&pub_bytes)
        .map_err(|e| format!("Invalid public key: {}", e))?;
    let shared_secret = secret.diffie_hellman(&browser_pub);
    let raw = shared_secret.raw_secret_bytes();
    let mut key = [0u8; 32];
    key.copy_from_slice(raw);
    Ok(key)
}

// === Config persistence ===

fn get_config_dir() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("claude-remote"))
}

fn load_session_from_disk() -> Option<SavedSession> {
    let dir = get_config_dir()?;
    let path = dir.join("session.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_session_to_disk(session: &SavedSession) {
    if let Some(dir) = get_config_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.json");
        if let Ok(data) = serde_json::to_string_pretty(session) {
            let _ = std::fs::write(path, data);
        }
    }
}

fn delete_session_from_disk() {
    if let Some(dir) = get_config_dir() {
        let path = dir.join("session.json");
        let _ = std::fs::remove_file(path);
    }
}

fn load_config_from_disk() -> Option<AppConfig> {
    let dir = get_config_dir()?;
    let path = dir.join("config.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_config_to_disk(config: &AppConfig) {
    if let Some(dir) = get_config_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.json");
        if let Ok(data) = serde_json::to_string_pretty(config) {
            let _ = std::fs::write(path, data);
        }
    }
}

// === Firebase Auth (REST API) ===

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthResponse {
    id_token: String,
    local_id: String,
    refresh_token: String,
}

#[derive(Deserialize)]
struct AuthError {
    error: AuthErrorDetail,
}

#[derive(Deserialize)]
struct AuthErrorDetail {
    message: String,
}

#[derive(Deserialize)]
struct RefreshResponse {
    id_token: String,
    refresh_token: String,
    user_id: String,
}

async fn refresh_auth_token(api_key: &str, refresh_token: &str) -> Result<RefreshResponse, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://securetoken.googleapis.com/v1/token?key={}",
        api_key
    );

    let body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh_token
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        resp.json().await.map_err(|e| e.to_string())
    } else {
        Err("Refresh token expired".to_string())
    }
}

async fn save_auth_state(state: &AppState, email: &str, uid: &str, id_token: &str, refresh_tok: &str) {
    *state.auth_token.lock().await = Some(id_token.to_string());
    *state.uid.lock().await = Some(uid.to_string());
    *state.email.lock().await = Some(email.to_string());
    *state.refresh_token.lock().await = Some(refresh_tok.to_string());

    save_session_to_disk(&SavedSession {
        email: email.to_string(),
        uid: uid.to_string(),
        refresh_token: refresh_tok.to_string(),
    });
}

#[derive(Serialize)]
struct SessionInfo {
    email: String,
    uid: String,
}

#[tauri::command]
async fn restore_session(
    state: State<'_, Arc<AppState>>,
) -> Result<SessionInfo, String> {
    let session = load_session_from_disk().ok_or("No saved session")?;

    let config = state.config.lock().await;
    let api_key = &config.firebase_api_key;

    let refreshed = refresh_auth_token(api_key, &session.refresh_token).await?;

    drop(config);

    save_auth_state(
        &state,
        &session.email,
        &refreshed.user_id,
        &refreshed.id_token,
        &refreshed.refresh_token,
    ).await;

    println!("[auth] Session restored for {}", session.email);

    Ok(SessionInfo {
        email: session.email,
        uid: refreshed.user_id,
    })
}

#[tauri::command]
async fn login(
    email: String,
    password: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let config = state.config.lock().await;
    let api_key = config.firebase_api_key.clone();
    drop(config);

    let client = reqwest::Client::new();
    let url = format!(
        "https://identitytoolkit.googleapis.com/v1/accounts:signInWithPassword?key={}",
        api_key
    );

    let body = serde_json::json!({
        "email": email,
        "password": password,
        "returnSecureToken": true
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        let auth: AuthResponse = resp.json().await.map_err(|e| e.to_string())?;
        save_auth_state(&state, &email, &auth.local_id, &auth.id_token, &auth.refresh_token).await;
        Ok(auth.local_id)
    } else {
        let err: AuthError = resp.json().await.map_err(|e| e.to_string())?;
        Err(err.error.message)
    }
}

#[tauri::command]
async fn register(
    email: String,
    password: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let config = state.config.lock().await;
    let api_key = config.firebase_api_key.clone();
    drop(config);

    let client = reqwest::Client::new();
    let url = format!(
        "https://identitytoolkit.googleapis.com/v1/accounts:signUp?key={}",
        api_key
    );

    let body = serde_json::json!({
        "email": email,
        "password": password,
        "returnSecureToken": true
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        let auth: AuthResponse = resp.json().await.map_err(|e| e.to_string())?;
        save_auth_state(&state, &email, &auth.local_id, &auth.id_token, &auth.refresh_token).await;
        Ok(auth.local_id)
    } else {
        let err: AuthError = resp.json().await.map_err(|e| e.to_string())?;
        Err(err.error.message)
    }
}

#[tauri::command]
async fn logout(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    *state.auth_token.lock().await = None;
    *state.uid.lock().await = None;
    *state.email.lock().await = None;
    *state.refresh_token.lock().await = None;
    *state.running.lock().await = false;
    delete_session_from_disk();
    Ok(())
}

// === Save/Load Config ===

#[tauri::command]
async fn save_config(
    working_dir: String,
    claude_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.working_dir = working_dir;
    config.claude_path = claude_path;
    save_config_to_disk(&config);
    Ok(())
}

#[tauri::command]
async fn get_config(state: State<'_, Arc<AppState>>) -> Result<AppConfig, String> {
    Ok(state.config.lock().await.clone())
}

// === Claude Code Runner ===

async fn run_claude(claude_path: &str, working_dir: &str, prompt: &str) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/aleksandr".to_string());
    let path = format!(
        "{}/.local/bin:{}/.cargo/bin:{}/.local/node/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
        home, home, home
    );

    // Inherit full env, then override specific vars (like Node.js { ...process.env, ... })
    let mut envs: std::collections::HashMap<String, String> = std::env::vars().collect();
    envs.remove("CLAUDECODE");
    envs.insert("PATH".into(), path);
    envs.insert("HOME".into(), home.clone());
    envs.insert("TERM".into(), "xterm-256color".into());
    // Use CLAUDE_CONFIG_DIR from environment if set, otherwise default (~/.claude)
    if let Ok(config_dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        envs.insert("CLAUDE_CONFIG_DIR".into(), config_dir);
    }

    let mut child = tokio::process::Command::new(claude_path)
        .args(["-p", "--continue", "--dangerously-skip-permissions", prompt])
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(&envs)
        .spawn()
        .map_err(|e| format!("Failed to start Claude: {}", e))?;

    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let mut output = String::new();
    let mut err_output = String::new();
    stdout
        .read_to_string(&mut output)
        .await
        .map_err(|e| e.to_string())?;
    stderr
        .read_to_string(&mut err_output)
        .await
        .map_err(|e| e.to_string())?;

    let status = child.wait().await.map_err(|e| e.to_string())?;
    if status.success() {
        Ok(output.trim().to_string())
    } else {
        // Claude writes errors to stdout (e.g. rate limits), stderr may be empty
        let msg = if !output.trim().is_empty() {
            output.trim().to_string()
        } else if !err_output.trim().is_empty() {
            err_output.trim().to_string()
        } else {
            format!("Claude exited with code: {:?}", status.code())
        };
        Err(msg)
    }
}

// === RTDB Polling Daemon ===

async fn send_heartbeat(client: &reqwest::Client, state: &Arc<AppState>) {
    let token = state.auth_token.lock().await.clone();
    let uid = state.uid.lock().await.clone();
    let config = state.config.lock().await.clone();
    let is_running = *state.running.lock().await;
    let is_busy = *state.busy.lock().await;

    let (token, uid) = match (token, uid) {
        (Some(t), Some(u)) => (t, u),
        _ => return,
    };

    let url = format!(
        "{}/sessions/{}/_heartbeat.json?auth={}",
        config.firebase_db_url, uid, token
    );

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let payload = serde_json::json!({
        "status": if !is_running { "stopped" } else if is_busy { "busy" } else { "idle" },
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        "hostname": hostname,
        "lastHeartbeat": {".sv": "timestamp"}
    });

    match client.put(&url).json(&payload).send().await {
        Ok(resp) => println!("[heartbeat] Sent: HTTP {}", resp.status()),
        Err(e) => println!("[heartbeat] Error: {}", e),
    }
}

async fn heartbeat_loop(state: Arc<AppState>) {
    let client = reqwest::Client::new();
    // First heartbeat after 2 sec
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    loop {
        send_heartbeat(&client, &state).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
}

async fn poll_messages(state: Arc<AppState>, crypto: Arc<CryptoState>) {
    let client = reqwest::Client::new();

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let is_running = *state.running.lock().await;
        if !is_running {
            continue;
        }

        let token = state.auth_token.lock().await.clone();
        let uid = state.uid.lock().await.clone();
        let config = state.config.lock().await.clone();

        let (token, uid) = match (token, uid) {
            (Some(t), Some(u)) => (t, u),
            _ => continue,
        };

        // Poll all sessions for this user
        let url = format!(
            "{}/sessions/{}.json?auth={}",
            config.firebase_db_url, uid, token
        );

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                println!("[daemon] Poll error: {}", e);
                continue;
            }
        };

        if !resp.status().is_success() {
            // Token might be expired, try refresh
            if resp.status().as_u16() == 401 {
                if let Some(ref_tok) = state.refresh_token.lock().await.clone() {
                    if let Ok(refreshed) = refresh_auth_token(&config.firebase_api_key, &ref_tok).await {
                        *state.auth_token.lock().await = Some(refreshed.id_token.clone());
                        *state.refresh_token.lock().await = Some(refreshed.refresh_token.clone());
                        if let Some(email) = state.email.lock().await.clone() {
                            save_session_to_disk(&SavedSession {
                                email,
                                uid: refreshed.user_id,
                                refresh_token: refreshed.refresh_token,
                            });
                        }
                        println!("[daemon] Token refreshed");
                    }
                }
            } else {
                println!("[daemon] Poll HTTP {}", resp.status());
            }
            continue;
        }

        let body: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        if body.is_null() {
            continue;
        }

        let sessions = match body.as_object() {
            Some(s) => s,
            None => continue,
        };

        for (session_id, session_data) in sessions {
            // === E2E Key Exchange ===
            // Check if browser posted its public key
            if let Some(keys) = session_data.get("keys") {
                let browser_key = keys.get("browser").and_then(|k| k.as_str());

                if let Some(browser_pub) = browser_key {
                    // Check if we need to (re-)derive: no cipher yet, or browser key changed
                    let needs_derive = {
                        let keys_map = crypto.session_keys.lock().await;
                        match keys_map.get(session_id) {
                            None => true,
                            Some((_, stored_browser_key)) => stored_browser_key != browser_pub,
                        }
                    };

                    if needs_derive {
                        let (secret, our_pub_b64) = generate_ecdh_keypair();

                        match derive_aes_key(secret, browser_pub) {
                            Ok(key_bytes) => {
                                crypto.session_keys.lock().await.insert(
                                    session_id.clone(),
                                    (key_bytes, browser_pub.to_string()),
                                );
                                println!("[crypto] Derived AES key for session {}", session_id);

                                // Always write our new public key (browser deleted the old one)
                                let key_url = format!(
                                    "{}/sessions/{}/{}/keys/daemon.json?auth={}",
                                    config.firebase_db_url, uid, session_id, token
                                );
                                let _ = client
                                    .put(&key_url)
                                    .json(&serde_json::json!(our_pub_b64))
                                    .send()
                                    .await;
                                println!("[crypto] Published daemon public key for session {}", session_id);
                            }
                            Err(e) => {
                                println!("[crypto] Key derivation failed for {}: {}", session_id, e);
                            }
                        }
                    }
                }
            }

            let messages = match session_data.get("messages").and_then(|m| m.as_object()) {
                Some(m) => m,
                None => continue,
            };

            // Get cipher for this session (if encryption is set up)
            let session_cipher = crypto.session_keys.lock().await.get(session_id).map(|(k, _)| make_cipher(k));

            for (msg_id, msg_data) in messages {
                let status = msg_data
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let role = msg_data
                    .get("role")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");

                if role != "user" {
                    continue;
                }

                // Accept "pending" messages, and also "processing" messages
                // that got stuck (e.g. token expired during Claude execution)
                let is_busy = *state.busy.lock().await;
                if status == "processing" && !is_busy {
                    println!("[daemon] Retrying stuck message: {}", msg_id);
                } else if status != "pending" {
                    continue;
                }

                let raw_text = msg_data
                    .get("text")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");

                if raw_text.is_empty() {
                    continue;
                }

                // Decrypt if message is encrypted
                let is_encrypted = msg_data
                    .get("encrypted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let text = if is_encrypted {
                    let iv = msg_data.get("iv").and_then(|v| v.as_str()).unwrap_or("");
                    if let Some(ref cipher) = session_cipher {
                        match decrypt_message(cipher, raw_text, iv) {
                            Ok(decrypted) => decrypted,
                            Err(e) => {
                                println!("[crypto] Decrypt failed for {}: {}", msg_id, e);
                                continue;
                            }
                        }
                    } else {
                        println!("[crypto] No cipher for encrypted message in session {}", session_id);
                        continue;
                    }
                } else {
                    raw_text.to_string()
                };

                let preview: String = text.chars().take(50).collect();
                println!("[daemon] Processing: \"{}\"", preview);

                *state.busy.lock().await = true;

                // Mark as processing
                let update_url = format!(
                    "{}/sessions/{}/{}/messages/{}/status.json?auth={}",
                    config.firebase_db_url, uid, session_id, msg_id, token
                );
                let _ = client
                    .put(&update_url)
                    .json(&serde_json::json!("processing"))
                    .send()
                    .await;

                // Run Claude
                let response = run_claude(&config.claude_path, &config.working_dir, &text).await;

                let (response_text, response_status) = match response {
                    Ok(text) => (text, "done"),
                    Err(err) => (err, "error"),
                };

                // Refresh token before writing response (Claude may have run for a long time)
                let fresh_token = match state.auth_token.lock().await.clone() {
                    Some(t) => {
                        // Try a test read to check if token is still valid
                        let test_url = format!(
                            "{}/sessions/{}/_heartbeat.json?auth={}",
                            config.firebase_db_url, uid, t
                        );
                        let test = client.get(&test_url).send().await;
                        if let Ok(r) = test {
                            if r.status().as_u16() == 401 {
                                // Token expired, refresh it
                                if let Some(ref_tok) = state.refresh_token.lock().await.clone() {
                                    if let Ok(refreshed) = refresh_auth_token(&config.firebase_api_key, &ref_tok).await {
                                        *state.auth_token.lock().await = Some(refreshed.id_token.clone());
                                        *state.refresh_token.lock().await = Some(refreshed.refresh_token.clone());
                                        if let Some(email) = state.email.lock().await.clone() {
                                            save_session_to_disk(&SavedSession {
                                                email,
                                                uid: refreshed.user_id,
                                                refresh_token: refreshed.refresh_token,
                                            });
                                        }
                                        println!("[daemon] Token refreshed before writing response");
                                        refreshed.id_token
                                    } else {
                                        println!("[daemon] Failed to refresh token");
                                        t
                                    }
                                } else { t }
                            } else { t }
                        } else { t }
                    }
                    None => {
                        println!("[daemon] No token available for response");
                        *state.busy.lock().await = false;
                        continue;
                    }
                };

                // Write response message (encrypted if cipher available)
                let resp_url = format!(
                    "{}/sessions/{}/{}/messages.json?auth={}",
                    config.firebase_db_url, uid, session_id, fresh_token
                );

                let resp_payload = if let Some(ref cipher) = session_cipher {
                    match encrypt_message(cipher, &response_text) {
                        Ok((enc_text, iv)) => {
                            serde_json::json!({
                                "role": "assistant",
                                "text": enc_text,
                                "iv": iv,
                                "encrypted": true,
                                "status": response_status,
                                "timestamp": {".sv": "timestamp"}
                            })
                        }
                        Err(e) => {
                            println!("[crypto] Encrypt failed, sending plaintext: {}", e);
                            serde_json::json!({
                                "role": "assistant",
                                "text": response_text,
                                "status": response_status,
                                "timestamp": {".sv": "timestamp"}
                            })
                        }
                    }
                } else {
                    serde_json::json!({
                        "role": "assistant",
                        "text": response_text,
                        "status": response_status,
                        "timestamp": {".sv": "timestamp"}
                    })
                };

                let _ = client
                    .post(&resp_url)
                    .json(&resp_payload)
                    .send()
                    .await;

                // Mark user message as done
                let update_url_fresh = format!(
                    "{}/sessions/{}/{}/messages/{}/status.json?auth={}",
                    config.firebase_db_url, uid, session_id, msg_id, fresh_token
                );
                let _ = client
                    .put(&update_url_fresh)
                    .json(&serde_json::json!("done"))
                    .send()
                    .await;

                println!("[daemon] Response sent");
                *state.busy.lock().await = false;
            }
        }
    }
}

// === Start/Stop Daemon ===

#[tauri::command]
async fn start_daemon(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    *state.running.lock().await = true;
    Ok(())
}

#[tauri::command]
async fn stop_daemon(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    *state.running.lock().await = false;
    Ok(())
}

#[tauri::command]
async fn get_status(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let running = *state.running.lock().await;
    let has_auth = state.auth_token.lock().await.is_some();
    if running && has_auth {
        Ok("connected".to_string())
    } else if has_auth {
        Ok("authenticated".to_string())
    } else {
        Ok("disconnected".to_string())
    }
}

// === Quit App ===

#[tauri::command]
async fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

// === Check for Updates ===

#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;
    let update = app
        .updater()
        .map_err(|e| format!("Updater init error: {}", e))?
        .check()
        .await
        .map_err(|e| format!("Update check error: {}", e))?;

    match update {
        Some(u) => {
            let version = u.version.clone();
            println!("[updater] Update available: v{}", version);

            // Download and install synchronously (not in background)
            u.download_and_install(|_, _| {}, || {})
                .await
                .map_err(|e| format!("Install error: {}", e))?;

            println!("[updater] Update installed, restarting...");
            app.restart();
            Ok(version)
        }
        None => Ok("latest".to_string()),
    }
}

// Background update checker: runs every hour, installs when daemon is stopped
async fn background_update_loop(app: tauri::AppHandle, state: Arc<AppState>) {
    use tauri_plugin_updater::UpdaterExt;

    // Initial delay: 60 seconds after startup
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

    loop {
        let is_running = *state.running.lock().await;
        let is_busy = *state.busy.lock().await;

        if !is_running && !is_busy {
            println!("[updater] Background check...");
            match app.updater() {
                Ok(updater) => {
                    match updater.check().await {
                        Ok(Some(update)) => {
                            let version = update.version.clone();
                            println!("[updater] Update v{} found, daemon stopped — installing", version);

                            match update.download_and_install(|_, _| {}, || {}).await {
                                Ok(_) => {
                                    println!("[updater] v{} installed, restarting...", version);
                                    app.restart();
                                }
                                Err(e) => println!("[updater] Install error: {}", e),
                            }
                        }
                        Ok(None) => println!("[updater] Up to date"),
                        Err(e) => println!("[updater] Check error: {}", e),
                    }
                }
                Err(e) => println!("[updater] Init error: {}", e),
            }
        } else {
            println!("[updater] Daemon running, skipping update check");
        }

        // Check every hour
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}

#[tauri::command]
async fn get_version(app: tauri::AppHandle) -> Result<String, String> {
    Ok(app.package_info().version.to_string())
}

// === Detect Claude Code ===

#[tauri::command]
async fn detect_claude() -> Result<String, String> {
    let candidates = vec![
        dirs::home_dir()
            .map(|h| h.join(".claude/local/claude").to_string_lossy().to_string())
            .unwrap_or_default(),
        dirs::home_dir()
            .map(|h| h.join(".local/bin/claude").to_string_lossy().to_string())
            .unwrap_or_default(),
        "/usr/local/bin/claude".to_string(),
        "/opt/homebrew/bin/claude".to_string(),
    ];

    for path in candidates {
        if std::path::Path::new(&path).exists() {
            return Ok(path);
        }
    }

    Err("Claude Code not found. Please install it first.".to_string())
}

// === Tauri Entry ===

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load saved config from disk or use defaults
    let mut saved_config = load_config_from_disk().unwrap_or(AppConfig {
        working_dir: dirs::home_dir()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_default(),
        claude_path: String::new(),
        firebase_api_key: "AIzaSyCxV6rBIk88Ur7qDMknibWZYs2D5zmVoFI".to_string(),
        firebase_db_url: "https://chilin1-default-rtdb.europe-west1.firebasedatabase.app".to_string(),
    });

    // Auto-detect Claude Code path if not configured
    if saved_config.claude_path.is_empty() {
        let candidates = [
            dirs::home_dir().map(|h| h.join(".claude/local/claude").to_string_lossy().to_string()),
            dirs::home_dir().map(|h| h.join(".local/bin/claude").to_string_lossy().to_string()),
            Some("/usr/local/bin/claude".to_string()),
            Some("/opt/homebrew/bin/claude".to_string()),
        ];
        for candidate in candidates.iter().flatten() {
            if std::path::Path::new(candidate).exists() {
                saved_config.claude_path = candidate.clone();
                save_config_to_disk(&saved_config);
                break;
            }
        }
    }

    // Check for --autostart flag
    let autostart = std::env::args().any(|arg| arg == "--autostart");

    let state = Arc::new(AppState {
        config: Mutex::new(saved_config),
        ..Default::default()
    });

    // If --autostart, restore session and start daemon immediately
    if autostart {
        if let Some(session) = load_session_from_disk() {
            let state_clone = state.clone();
            let rt = tokio::runtime::Runtime::new().unwrap();
            let api_key = {
                let config = rt.block_on(state_clone.config.lock());
                config.firebase_api_key.clone()
            };
            match rt.block_on(refresh_auth_token(&api_key, &session.refresh_token)) {
                Ok(refreshed) => {
                    rt.block_on(async {
                        save_auth_state(
                            &state_clone,
                            &session.email,
                            &refreshed.user_id,
                            &refreshed.id_token,
                            &refreshed.refresh_token,
                        ).await;
                        *state_clone.running.lock().await = true;
                    });
                    println!("[autostart] Session restored for {}, daemon started", session.email);
                }
                Err(e) => {
                    println!("[autostart] Failed to restore session: {}", e);
                }
            }
        } else {
            println!("[autostart] No saved session found");
        }
    }

    let crypto_state = Arc::new(CryptoState::default());
    let state_for_daemon = state.clone();
    let crypto_for_daemon = crypto_state.clone();
    let state_for_heartbeat = state.clone();
    let state_for_updater = state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .setup(|app| {
            // Build tray menu
            let show = MenuItemBuilder::with_id("show", "Settings").build(app)?;
            let status = MenuItemBuilder::with_id("status", "Status: Disconnected")
                .enabled(false)
                .build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&status)
                .separator()
                .item(&show)
                .item(&quit)
                .build()?;

            // Decode PNG to RGBA
            let tray_icon_bytes = include_bytes!("../icons/tray.png");
            let decoder = png::Decoder::new(std::io::Cursor::new(tray_icon_bytes));
            let mut reader = decoder.read_info().unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            buf.truncate(info.buffer_size());
            let tray_image = tauri::image::Image::new_owned(buf, info.width, info.height);

            TrayIconBuilder::new()
                .icon(tray_image)
                .icon_as_template(false)
                .menu(&menu)
                .tooltip("Claude Remote")
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        if let Some(win) = tray.app_handle().get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            // Hide window on close instead of quitting
            let win = app.get_webview_window("main").unwrap();
            let win_clone = win.clone();
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = win_clone.hide();
                }
            });

            // Start polling daemon and heartbeat in background
            tauri::async_runtime::spawn(poll_messages(state_for_daemon, crypto_for_daemon));
            tauri::async_runtime::spawn(heartbeat_loop(state_for_heartbeat));
            tauri::async_runtime::spawn(background_update_loop(app.handle().clone(), state_for_updater));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login,
            register,
            logout,
            restore_session,
            save_config,
            get_config,
            start_daemon,
            stop_daemon,
            get_status,
            detect_claude,
            check_for_updates,
            quit_app,
            get_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
