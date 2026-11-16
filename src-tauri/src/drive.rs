//! The parish library in Google Drive, reached directly (no Google Drive for
//! desktop needed).
//!
//! The parish creates a Google Cloud project once and gives each computer
//! its OAuth client file ("Desktop app" credentials, `google-client.json`).
//! Each person signs in with their Google account in the browser and picks
//! the shared parish folder. The app then keeps a full copy of that folder on
//! the computer, the **mirror**, and works from it exactly as it would from a
//! folder synced by Google Drive for desktop: storage.rs treats the mirror as
//! the parish library folder, and sync.rs, handoff.rs and the rest are
//! unchanged.
//!
//! Keeping the mirror and Drive in step ([`sync`]):
//!
//! 1. The first time (and every few hours, to be safe) the whole folder is
//!    listed and compared file by file: a file on one side only goes to the
//!    other; where both have it and the contents differ, the side that
//!    changed since the last sync wins, else the newer.
//! 2. Every few seconds Drive's change list says what changed since last
//!    time (only those files are downloaded), and the mirror is checked for
//!    files changed here (only those are uploaded). A file deleted on one
//!    side is deleted on the other.
//!
//! Files are written whole (to a temporary name, then renamed), so nothing
//! ever reads a half-downloaded file, and Drive's own checksums show when a
//! change is just this computer's upload coming back.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Digest;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

/// Full access to the folders the user can reach (a shared folder is not
/// one this app created, so the narrower `drive.file` cannot see it).
const SCOPE: &str = "https://www.googleapis.com/auth/drive";
const AUTH: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN: &str = "https://oauth2.googleapis.com/token";
const API: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3";
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
const FIELDS: &str = "id,name,mimeType,md5Checksum,modifiedTime,size,parents,trashed";
const CLIENT_FILE: &str = "google-client.json";
const ACCOUNT_FILE: &str = "google-account.json";

/// Emitted when the Drive connection or its sync changes (the dialog and the
/// top bar show it).
pub const EVENT: &str = "episco://drive";

// ---------------------------------------------------------------------------
// The parish's Google client
// ---------------------------------------------------------------------------

/// The parish's OAuth client ("Desktop app" credentials from the Google
/// Cloud console). For a desktop app the secret is not secret: Google's
/// sign-in protects the account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Client {
    pub client_id: String,
    pub client_secret: String,
}

/// Reads the client file as the Cloud console downloads it
/// (`{"installed": {...}}`), or just its two values.
pub fn parse_client(text: &str) -> Result<Client, String> {
    let v: Value = serde_json::from_str(text).map_err(|_| "This is not a Google client file (JSON).".to_string())?;
    let inner = v.get("installed").or_else(|| v.get("web")).unwrap_or(&v);
    let get = |k: &str| inner.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    match (get("client_id"), get("client_secret")) {
        (Some(client_id), Some(client_secret)) => Ok(Client { client_id, client_secret }),
        _ => Err("The file has no client ID and secret. Download the “Desktop app” OAuth client's JSON from the Google Cloud console.".into()),
    }
}

fn config_dir(app: &AppHandle) -> CmdResult<PathBuf> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

pub fn load_client(app: &AppHandle) -> Option<Client> {
    parse_client(&std::fs::read_to_string(config_dir(app).ok()?.join(CLIENT_FILE)).ok()?).ok()
}

// ---------------------------------------------------------------------------
// The signed-in account
// ---------------------------------------------------------------------------

/// The account signed in on this computer: its refresh token and address.
/// Kept in the app's settings folder, readable by this user only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Account {
    refresh_token: String,
    email: String,
}

fn load_account(app: &AppHandle) -> Option<Account> {
    let text = std::fs::read_to_string(config_dir(app).ok()?.join(ACCOUNT_FILE)).ok()?;
    serde_json::from_str::<Account>(&text).ok().filter(|a| !a.refresh_token.is_empty())
}

fn save_private(path: &Path, text: &str) -> CmdResult<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path).map_err(|e| format!("could not save {}: {e}", path.display()))?;
    f.write_all(text.as_bytes()).map_err(|e| e.to_string())
}

/// The access token in use and when it expires.
static ACCESS: Mutex<Option<(String, Instant)>> = Mutex::new(None);

fn http() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

/// An access token for the signed-in account (renewed when it runs out).
fn access_token(app: &AppHandle) -> CmdResult<String> {
    if let Some((t, until)) = ACCESS.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        if Instant::now() < until {
            return Ok(t);
        }
    }
    let client = load_client(app).ok_or("No Google client file on this computer.")?;
    let account = load_account(app).ok_or("Not signed in to Google.")?;
    let resp = http()
        .post(TOKEN)
        .form(&[
            ("client_id", client.client_id.as_str()),
            ("client_secret", client.client_secret.as_str()),
            ("refresh_token", account.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .map_err(|e| format!("could not reach Google: {e}"))?;
    let status = resp.status();
    let v: Value = serde_json::from_str(&resp.text().unwrap_or_default()).unwrap_or(Value::Null);
    if !status.is_success() {
        let why = v.get("error").and_then(Value::as_str).unwrap_or("error");
        return Err(if why == "invalid_grant" {
            "The Google sign-in has expired or was removed: sign in again.".into()
        } else {
            format!("Google refused the sign-in ({why}).")
        });
    }
    let token = v.get("access_token").and_then(Value::as_str).ok_or("Google sent no access token.")?.to_string();
    let secs = v.get("expires_in").and_then(Value::as_u64).unwrap_or(3600);
    *ACCESS.lock().unwrap_or_else(|e| e.into_inner()) =
        Some((token.clone(), Instant::now() + Duration::from_secs(secs.saturating_sub(120))));
    Ok(token)
}

fn random_b64(n: usize) -> String {
    let mut buf = vec![0u8; n];
    let _ = getrandom::getrandom(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

/// Opens `url` in the default browser.
fn open_browser(url: &str) -> CmdResult<()> {
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let r = std::process::Command::new("xdg-open").arg(url).spawn();
    r.map(|_| ()).map_err(|e| format!("could not open the browser: {e}"))
}

/// Signs in in the browser (Google's sign-in for desktop apps: a one-time
/// code sent back to this computer, protected by PKCE). Waits up to five
/// minutes. Returns the account's address.
fn sign_in(app: &AppHandle) -> CmdResult<String> {
    let client = load_client(app).ok_or("Add the parish's Google client file first.")?;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}");
    let verifier = random_b64(48);
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(verifier.as_bytes()));
    let state = random_b64(16);
    let url = reqwest::Url::parse_with_params(
        AUTH,
        &[
            ("client_id", client.client_id.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("response_type", "code"),
            ("scope", SCOPE),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("access_type", "offline"),
            ("prompt", "consent"),
            ("state", state.as_str()),
        ],
    )
    .map_err(|e| e.to_string())?;
    open_browser(url.as_str())?;

    // Wait for the browser to come back with the code.
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let until = Instant::now() + Duration::from_secs(300);
    let code = loop {
        if Instant::now() > until {
            return Err("No answer from the Google sign-in page within five minutes.".into());
        }
        let (mut stream, _) = match listener.accept() {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        let path = request.split_whitespace().nth(1).unwrap_or("/");
        let parsed = reqwest::Url::parse(&format!("http://localhost{path}")).ok();
        let get = |k: &str| {
            parsed.as_ref().and_then(|u| u.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.into_owned()))
        };
        let (found, error) = (get("code"), get("error"));
        let body = if found.is_some() {
            "<h2 style='font-family:sans-serif'>Signed in. You can close this tab and return to the app.</h2>"
        } else {
            "<h2 style='font-family:sans-serif'>The sign-in did not finish. Return to the app and try again.</h2>"
        };
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        );
        if let Some(e) = error {
            return Err(format!("Google sign-in was not completed ({e})."));
        }
        if let Some(c) = found {
            if get("state").as_deref() != Some(state.as_str()) {
                return Err("The sign-in answer did not match: try again.".into());
            }
            break c;
        }
    };

    let resp = http()
        .post(TOKEN)
        .form(&[
            ("client_id", client.client_id.as_str()),
            ("client_secret", client.client_secret.as_str()),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect.as_str()),
        ])
        .send()
        .map_err(|e| format!("could not reach Google: {e}"))?;
    let status = resp.status();
    let v: Value = serde_json::from_str(&resp.text().unwrap_or_default()).unwrap_or(Value::Null);
    if !status.is_success() {
        return Err(format!(
            "Google refused the sign-in ({}).",
            v.get("error_description").or_else(|| v.get("error")).and_then(Value::as_str).unwrap_or("error")
        ));
    }
    let refresh =
        v.get("refresh_token").and_then(Value::as_str).ok_or("Google sent no lasting sign-in: try again.")?.to_string();
    let access = v.get("access_token").and_then(Value::as_str).unwrap_or_default().to_string();
    *ACCESS.lock().unwrap_or_else(|e| e.into_inner()) =
        Some((access.clone(), Instant::now() + Duration::from_secs(3000)));
    let about: Value = http()
        .get(format!("{API}/about"))
        .bearer_auth(&access)
        .query(&[("fields", "user(emailAddress)")])
        .send()
        .and_then(|r| r.json_value())
        .unwrap_or(Value::Null);
    let email =
        about.pointer("/user/emailAddress").and_then(Value::as_str).unwrap_or("your Google account").to_string();
    let account = Account { refresh_token: refresh, email: email.clone() };
    save_private(&config_dir(app)?.join(ACCOUNT_FILE), &serde_json::to_string(&account).map_err(|e| e.to_string())?)?;
    Ok(email)
}

/// `.json()` without reqwest's json feature.
trait JsonValue {
    fn json_value(self) -> reqwest::Result<Value>;
}

impl JsonValue for reqwest::blocking::Response {
    fn json_value(self) -> reqwest::Result<Value> {
        Ok(serde_json::from_str(&self.text()?).unwrap_or(Value::Null))
    }
}

// ---------------------------------------------------------------------------
// Drive, as the mirror needs it
// ---------------------------------------------------------------------------

/// A file or folder in Drive.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RFile {
    pub id: String,
    pub name: String,
    pub folder: bool,
    /// A Google Docs, Sheets or Slides file (no contents to copy).
    pub native: bool,
    pub md5: Option<String>,
    /// Seconds since 1970.
    pub modified: u64,
    pub parents: Vec<String>,
    pub trashed: bool,
}

/// One entry of Drive's change list.
#[derive(Debug, Clone, PartialEq)]
pub struct RChange {
    pub id: String,
    pub removed: bool,
    pub file: Option<RFile>,
}

/// What the mirror asks of Drive (a fake stands in for it in the tests).
pub trait Remote {
    fn list(&self, folder: &str) -> Result<Vec<RFile>, String>;
    fn start_token(&self) -> Result<String, String>;
    /// The changes since `token`, and the token for next time.
    fn changes(&self, token: &str) -> Result<(Vec<RChange>, String), String>;
    fn download(&self, id: &str) -> Result<Vec<u8>, String>;
    fn create_folder(&self, parent: &str, name: &str) -> Result<RFile, String>;
    fn create_file(&self, parent: &str, name: &str, bytes: &[u8]) -> Result<RFile, String>;
    fn update_file(&self, id: &str, bytes: &[u8]) -> Result<RFile, String>;
    fn trash(&self, id: &str) -> Result<(), String>;
}

/// "2026-11-16T00:45:06.123Z" as seconds since 1970.
pub fn parse_time(s: &str) -> u64 {
    let n = |a: usize, b: usize| s.get(a..b).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
    let (y, m, d, hh, mm, ss) = (n(0, 4), n(5, 7), n(8, 10), n(11, 13), n(14, 16), n(17, 19));
    if y == 0 {
        return 0;
    }
    // Days from 1970-01-01 (proleptic Gregorian).
    let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    (days * 86400 + hh * 3600 + mm * 60 + ss).max(0) as u64
}

fn rfile(v: &Value) -> RFile {
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    let mime = s("mimeType");
    RFile {
        id: s("id"),
        name: s("name"),
        folder: mime == FOLDER_MIME,
        native: mime.starts_with("application/vnd.google-apps.") && mime != FOLDER_MIME,
        md5: v.get("md5Checksum").and_then(Value::as_str).map(String::from),
        modified: parse_time(&s("modifiedTime")),
        parents: v
            .get("parents")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|p| p.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        trashed: v.get("trashed").and_then(Value::as_bool).unwrap_or(false),
    }
}

/// Google Drive itself, as the signed-in account.
pub struct Http<'a> {
    app: &'a AppHandle,
    client: reqwest::blocking::Client,
}

impl<'a> Http<'a> {
    pub fn new(app: &'a AppHandle) -> Self {
        Http { app, client: http() }
    }

    /// Sends a request (signed in), once more with a fresh token if Google
    /// says the old one ran out.
    fn send(
        &self,
        make: impl Fn(&reqwest::blocking::Client, &str) -> reqwest::blocking::RequestBuilder,
    ) -> CmdResult<reqwest::blocking::Response> {
        for attempt in 0..2 {
            let token = access_token(self.app)?;
            let resp = make(&self.client, &token).send().map_err(|e| format!("could not reach Google Drive: {e}"))?;
            if resp.status() == reqwest::StatusCode::UNAUTHORIZED && attempt == 0 {
                *ACCESS.lock().unwrap_or_else(|e| e.into_inner()) = None;
                continue;
            }
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                let msg = serde_json::from_str::<Value>(&body)
                    .ok()
                    .and_then(|v| v.pointer("/error/message").and_then(Value::as_str).map(String::from))
                    .unwrap_or_else(|| body.chars().take(200).collect());
                return Err(format!("Google Drive: {msg} ({})", status.as_u16()));
            }
            return Ok(resp);
        }
        Err("Google Drive refused the sign-in.".into())
    }

    fn json(
        &self,
        make: impl Fn(&reqwest::blocking::Client, &str) -> reqwest::blocking::RequestBuilder,
    ) -> CmdResult<Value> {
        self.send(make)?.json_value().map_err(|e| e.to_string())
    }

    /// Folders the account can reach (its own and shared with it), for
    /// choosing the parish folder.
    pub fn folders(&self, search: &str) -> CmdResult<Vec<Value>> {
        let mut q = format!("mimeType='{FOLDER_MIME}' and trashed=false");
        let search = search.trim().replace('\\', "").replace('\'', "\\'");
        if !search.is_empty() {
            q.push_str(&format!(" and name contains '{search}'"));
        }
        let v = self.json(|c, t| {
            c.get(format!("{API}/files")).bearer_auth(t).query(&[
                ("q", q.as_str()),
                ("fields", "files(id,name,shared,ownedByMe,owners(displayName),modifiedTime)"),
                ("pageSize", "60"),
                ("orderBy", "modifiedTime desc"),
                ("supportsAllDrives", "true"),
                ("includeItemsFromAllDrives", "true"),
            ])
        })?;
        Ok(v.get("files").and_then(Value::as_array).cloned().unwrap_or_default())
    }
}

/// A multipart/related upload body: the file's details, then its contents.
fn multipart(meta: &Value, bytes: &[u8], boundary: &str) -> Vec<u8> {
    let mut body = Vec::with_capacity(bytes.len() + 512);
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

impl Remote for Http<'_> {
    fn list(&self, folder: &str) -> Result<Vec<RFile>, String> {
        let q = format!("'{folder}' in parents and trashed=false");
        let fields = format!("nextPageToken,files({FIELDS})");
        let mut out = Vec::new();
        let mut page: Option<String> = None;
        loop {
            let v = self.json(|c, t| {
                let mut r = c.get(format!("{API}/files")).bearer_auth(t).query(&[
                    ("q", q.as_str()),
                    ("fields", fields.as_str()),
                    ("pageSize", "1000"),
                    ("supportsAllDrives", "true"),
                    ("includeItemsFromAllDrives", "true"),
                ]);
                if let Some(p) = &page {
                    r = r.query(&[("pageToken", p.as_str())]);
                }
                r
            })?;
            out.extend(v.get("files").and_then(Value::as_array).into_iter().flatten().map(rfile));
            match v.get("nextPageToken").and_then(Value::as_str) {
                Some(p) => page = Some(p.to_string()),
                None => return Ok(out),
            }
        }
    }

    fn start_token(&self) -> Result<String, String> {
        let v = self.json(|c, t| {
            c.get(format!("{API}/changes/startPageToken")).bearer_auth(t).query(&[("supportsAllDrives", "true")])
        })?;
        v.get("startPageToken")
            .and_then(Value::as_str)
            .map(String::from)
            .ok_or("Google Drive sent no change token".into())
    }

    fn changes(&self, token: &str) -> Result<(Vec<RChange>, String), String> {
        let fields = format!("nextPageToken,newStartPageToken,changes(fileId,removed,file({FIELDS}))");
        let mut out = Vec::new();
        let mut page = token.to_string();
        loop {
            let v = self.json(|c, t| {
                c.get(format!("{API}/changes")).bearer_auth(t).query(&[
                    ("pageToken", page.as_str()),
                    ("fields", fields.as_str()),
                    ("pageSize", "1000"),
                    ("includeRemoved", "true"),
                    ("supportsAllDrives", "true"),
                    ("includeItemsFromAllDrives", "true"),
                ])
            })?;
            for c in v.get("changes").and_then(Value::as_array).into_iter().flatten() {
                out.push(RChange {
                    id: c.get("fileId").and_then(Value::as_str).unwrap_or_default().to_string(),
                    removed: c.get("removed").and_then(Value::as_bool).unwrap_or(false),
                    file: c.get("file").map(rfile),
                });
            }
            if let Some(next) = v.get("newStartPageToken").and_then(Value::as_str) {
                return Ok((out, next.to_string()));
            }
            match v.get("nextPageToken").and_then(Value::as_str) {
                Some(p) => page = p.to_string(),
                None => return Ok((out, page)),
            }
        }
    }

    fn download(&self, id: &str) -> Result<Vec<u8>, String> {
        let resp = self.send(|c, t| {
            c.get(format!("{API}/files/{id}")).bearer_auth(t).query(&[("alt", "media"), ("supportsAllDrives", "true")])
        })?;
        resp.bytes().map(|b| b.to_vec()).map_err(|e| format!("could not download from Google Drive: {e}"))
    }

    fn create_folder(&self, parent: &str, name: &str) -> Result<RFile, String> {
        let meta = serde_json::json!({ "name": name, "mimeType": FOLDER_MIME, "parents": [parent] });
        let v = self.json(|c, t| {
            c.post(format!("{API}/files"))
                .bearer_auth(t)
                .query(&[("fields", FIELDS), ("supportsAllDrives", "true")])
                .header("Content-Type", "application/json")
                .body(meta.to_string())
        })?;
        Ok(rfile(&v))
    }

    fn create_file(&self, parent: &str, name: &str, bytes: &[u8]) -> Result<RFile, String> {
        let boundary = format!("episco{}", random_b64(12).replace(['-', '_'], "x"));
        let meta = serde_json::json!({ "name": name, "parents": [parent] });
        let body = multipart(&meta, bytes, &boundary);
        let v = self.json(|c, t| {
            c.post(format!("{UPLOAD}/files"))
                .bearer_auth(t)
                .query(&[("uploadType", "multipart"), ("fields", FIELDS), ("supportsAllDrives", "true")])
                .header("Content-Type", format!("multipart/related; boundary={boundary}"))
                .body(body.clone())
        })?;
        Ok(rfile(&v))
    }

    fn update_file(&self, id: &str, bytes: &[u8]) -> Result<RFile, String> {
        let v = self.json(|c, t| {
            c.patch(format!("{UPLOAD}/files/{id}"))
                .bearer_auth(t)
                .query(&[("uploadType", "media"), ("fields", FIELDS), ("supportsAllDrives", "true")])
                .header("Content-Type", "application/octet-stream")
                .body(bytes.to_vec())
        })?;
        Ok(rfile(&v))
    }

    fn trash(&self, id: &str) -> Result<(), String> {
        self.send(|c, t| {
            c.patch(format!("{API}/files/{id}"))
                .bearer_auth(t)
                .query(&[("supportsAllDrives", "true")])
                .header("Content-Type", "application/json")
                .body(r#"{"trashed":true}"#)
        })
        .map(|_| ())
    }
}

// ---------------------------------------------------------------------------
// The mirror
// ---------------------------------------------------------------------------

/// A file's modification time (ns) and size.
type Stamp = (u128, u64);

/// A file the mirror knows: its Drive id and checksum when last in step, and
/// the local file's stamp then.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Known {
    pub id: String,
    pub md5: Option<String>,
    pub local: Option<Stamp>,
}

/// What the mirror remembers between syncs (kept beside it).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mirror {
    /// The parish folder's Drive id.
    pub root: String,
    /// Drive's change token, from the last sync.
    pub token: Option<String>,
    /// Folder id → path in the mirror ("" for the parish folder itself).
    pub folders: BTreeMap<String, String>,
    /// Path in the mirror → the file as last in step.
    pub files: BTreeMap<String, Known>,
    /// The first full comparison is done.
    pub ready: bool,
    /// When the last full comparison ran (seconds since 1970).
    pub last_full: u64,
}

/// How a sync went.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Outcome {
    /// Files written or removed here.
    pub received: usize,
    /// Files sent to Drive (or removed there).
    pub sent: usize,
}

/// A full comparison every few hours catches anything the change list missed.
const FULL_EVERY: u64 = 6 * 3600;
/// A file changed here less than this long ago may still be being written.
const SETTLE: Duration = Duration::from_secs(2);

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn stamp(path: &Path) -> Option<Stamp> {
    let m = std::fs::metadata(path).ok().filter(|m| m.is_file())?;
    let t = m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos();
    Some((t, m.len()))
}

fn md5_hex(bytes: &[u8]) -> String {
    let d = md5::Md5::digest(bytes);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn skipped(name: &str) -> bool {
    name.starts_with('.')
        || name.starts_with("~$")
        || name.ends_with(".tmp")
        || name == "desktop.ini"
        || name == "Thumbs.db"
        || name.starts_with("Icon\r")
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// A Drive name that can be a file name here (Drive allows "/" and more).
fn safe_name(name: &str) -> Option<String> {
    let n = name.trim();
    (!n.is_empty() && n != "." && n != ".." && !n.contains(['/', '\\', '\0']) && !skipped(n)).then(|| n.to_string())
}

fn local_path(dir: &Path, rel: &str) -> PathBuf {
    rel.split('/').fold(dir.to_path_buf(), |p, part| p.join(part))
}

/// Every file in the mirror: path → stamp.
fn walk(dir: &Path) -> BTreeMap<String, Stamp> {
    let mut out = BTreeMap::new();
    let mut stack = vec![(dir.to_path_buf(), String::new())];
    while let Some((d, rel)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if skipped(&name) {
                continue;
            }
            let Ok(m) = e.metadata() else { continue };
            let r = join(&rel, &name);
            if m.is_dir() {
                stack.push((e.path(), r));
            } else if let Some(s) = stamp(&e.path()) {
                out.insert(r, s);
            }
        }
    }
    out
}

/// Writes a downloaded file whole: to a temporary name, then into place.
fn write_whole(path: &Path, bytes: &[u8]) -> Result<Stamp, String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!(
        "{}.episco-download.tmp",
        path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default()
    ));
    std::fs::write(&tmp, bytes).map_err(|e| format!("could not save {}: {e}", path.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("could not save {}: {e}", path.display()))?;
    stamp(path).ok_or_else(|| format!("could not read {}", path.display()))
}

fn local_secs(s: Stamp) -> u64 {
    (s.0 / 1_000_000_000) as u64
}

impl Mirror {
    pub fn new(root: &str) -> Mirror {
        Mirror { root: root.to_string(), ..Default::default() }
    }

    fn folder_id(&self, rel: &str) -> Option<String> {
        self.folders.iter().find(|(_, r)| r.as_str() == rel).map(|(id, _)| id.clone())
    }

    fn rel_of_id(&self, id: &str) -> Option<String> {
        self.files.iter().find(|(_, k)| k.id == id).map(|(r, _)| r.clone())
    }

    /// Brings the mirror at `dir` and Drive into step.
    pub fn sync(&mut self, remote: &dyn Remote, dir: &Path) -> Result<Outcome, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let mut out = Outcome::default();
        if !self.ready || now().saturating_sub(self.last_full) > FULL_EVERY {
            self.full(remote, dir, &mut out)?;
        } else {
            self.pull(remote, dir, &mut out)?;
        }
        self.push(remote, dir, &mut out)?;
        Ok(out)
    }

    /// The whole folder compared with Drive, file by file.
    fn full(&mut self, remote: &dyn Remote, dir: &Path, out: &mut Outcome) -> Result<(), String> {
        // The token first, so nothing changed during the listing is missed.
        let token = remote.start_token()?;
        let mut folders = BTreeMap::from([(self.root.clone(), String::new())]);
        let mut theirs: BTreeMap<String, RFile> = BTreeMap::new();
        let mut queue = vec![self.root.clone()];
        while let Some(id) = queue.pop() {
            let base = folders.get(&id).cloned().unwrap_or_default();
            for f in remote.list(&id)? {
                let Some(name) = safe_name(&f.name) else { continue };
                let rel = join(&base, &name);
                if f.folder {
                    if !folders.values().any(|r| *r == rel) {
                        let _ = std::fs::create_dir_all(local_path(dir, &rel));
                        folders.insert(f.id.clone(), rel);
                        queue.push(f.id.clone());
                    }
                } else if !f.native && !theirs.contains_key(&rel) {
                    theirs.insert(rel, f);
                }
            }
        }
        self.folders = folders;
        for (rel, f) in &theirs {
            let path = local_path(dir, rel);
            let known = self.files.get(rel).cloned();
            match stamp(&path) {
                None => {
                    // Deleted here since the last sync: Drive's copy goes too (push).
                    if known.as_ref().is_some_and(|k| k.local.is_some() && k.md5 == f.md5 && k.id == f.id) {
                        continue;
                    }
                    let s = write_whole(&path, &remote.download(&f.id)?)?;
                    self.files.insert(rel.clone(), Known { id: f.id.clone(), md5: f.md5.clone(), local: Some(s) });
                    out.received += 1;
                }
                Some(s) => {
                    let same = f.md5.is_some() && std::fs::read(&path).ok().map(|b| md5_hex(&b)) == f.md5.clone();
                    if same {
                        self.files.insert(rel.clone(), Known { id: f.id.clone(), md5: f.md5.clone(), local: Some(s) });
                        continue;
                    }
                    let here_changed = known.as_ref().is_none_or(|k| k.local != Some(s));
                    let there_changed = known.as_ref().is_none_or(|k| k.md5 != f.md5);
                    let take_theirs = match (here_changed, there_changed) {
                        (false, _) => true,
                        (true, false) => false,
                        (true, true) => f.modified > local_secs(s),
                    };
                    if take_theirs {
                        let s = write_whole(&path, &remote.download(&f.id)?)?;
                        self.files.insert(rel.clone(), Known { id: f.id.clone(), md5: f.md5.clone(), local: Some(s) });
                        out.received += 1;
                    } else {
                        // Ours goes to Drive (push sees the stamp differ).
                        self.files.insert(rel.clone(), Known { id: f.id.clone(), md5: f.md5.clone(), local: None });
                    }
                }
            }
        }
        // Files Drive no longer has.
        let gone: Vec<String> = self.files.keys().filter(|r| !theirs.contains_key(*r)).cloned().collect();
        for rel in gone {
            let k = self.files.remove(&rel).unwrap_or_default();
            let path = local_path(dir, &rel);
            if k.local.is_some() && stamp(&path) == k.local {
                // Unchanged here, removed there: remove it here.
                let _ = std::fs::remove_file(&path);
                out.received += 1;
            }
            // Changed here: push sends it again as a new file.
        }
        self.token = Some(token);
        self.ready = true;
        self.last_full = now();
        Ok(())
    }

    /// Drive's changes since last time.
    fn pull(&mut self, remote: &dyn Remote, dir: &Path, out: &mut Outcome) -> Result<(), String> {
        let Some(token) = self.token.clone() else {
            self.ready = false;
            return self.full(remote, dir, out);
        };
        let (changes, next) = remote.changes(&token)?;
        let mut rescan = false;
        for c in changes {
            let gone = c.removed || c.file.as_ref().is_none_or(|f| f.trashed);
            let inside =
                c.file.as_ref().and_then(|f| f.parents.iter().find(|p| self.folders.contains_key(*p)).cloned());
            if self.folders.contains_key(&c.id) {
                // A folder of ours moved, renamed or removed: compare again.
                let unchanged = !gone
                    && c.file.as_ref().is_some_and(|f| {
                        inside.as_ref().and_then(|p| self.folders.get(p)).map(|base| join(base, &f.name))
                            == self.folders.get(&c.id).cloned()
                    });
                if !unchanged && c.id != self.root {
                    rescan = true;
                }
                continue;
            }
            let was = self.rel_of_id(&c.id);
            let (Some(f), Some(parent), false) = (c.file.as_ref(), inside, gone) else {
                // Removed, or moved out of the parish folder.
                if let Some(rel) = was {
                    let k = self.files.remove(&rel).unwrap_or_default();
                    let path = local_path(dir, &rel);
                    if k.local.is_some() && stamp(&path) == k.local {
                        let _ = std::fs::remove_file(&path);
                        out.received += 1;
                    }
                }
                continue;
            };
            let Some(name) = safe_name(&f.name) else { continue };
            let rel = join(&self.folders[&parent], &name);
            if f.folder {
                let _ = std::fs::create_dir_all(local_path(dir, &rel));
                self.folders.insert(f.id.clone(), rel);
                continue;
            }
            if f.native {
                continue;
            }
            // Renamed or moved within the folder: the old name goes.
            if let Some(old) = was.filter(|o| *o != rel) {
                let k = self.files.remove(&old).unwrap_or_default();
                let p = local_path(dir, &old);
                if k.local.is_some() && stamp(&p) == k.local {
                    let _ = std::fs::remove_file(&p);
                }
            }
            let path = local_path(dir, &rel);
            let known = self.files.get(&rel).cloned();
            if known.as_ref().is_some_and(|k| k.id == f.id && k.md5 == f.md5 && f.md5.is_some()) {
                continue; // nothing new (often this computer's own upload)
            }
            let here = stamp(&path);
            let here_changed = here.is_some() && known.as_ref().is_none_or(|k| k.local != here);
            if here_changed && here.is_some_and(|s| local_secs(s) >= f.modified) {
                // Changed here too, and more recently: ours goes to Drive.
                self.files.insert(rel, Known { id: f.id.clone(), md5: f.md5.clone(), local: None });
                continue;
            }
            let s = write_whole(&path, &remote.download(&f.id)?)?;
            self.files.insert(rel, Known { id: f.id.clone(), md5: f.md5.clone(), local: Some(s) });
            out.received += 1;
        }
        self.token = Some(next);
        if rescan {
            self.ready = false;
        }
        Ok(())
    }

    /// The parish folder's subfolder at `rel` in Drive (made if missing).
    fn ensure_folder(&mut self, remote: &dyn Remote, rel: &str) -> Result<String, String> {
        if let Some(id) = self.folder_id(rel) {
            return Ok(id);
        }
        let (parent, name) = rel.rsplit_once('/').unwrap_or(("", rel));
        let parent_id = self.ensure_folder(remote, parent)?;
        let f = remote.create_folder(&parent_id, name)?;
        self.folders.insert(f.id.clone(), rel.to_string());
        Ok(f.id)
    }

    /// Files changed here since last time, to Drive.
    fn push(&mut self, remote: &dyn Remote, dir: &Path, out: &mut Outcome) -> Result<(), String> {
        let ours = walk(dir);
        let settle =
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) - SETTLE.as_nanos();
        for (rel, s) in &ours {
            let known = self.files.get(rel).cloned();
            if known.as_ref().is_some_and(|k| k.local == Some(*s)) || s.0 > settle {
                continue;
            }
            let path = local_path(dir, rel);
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let md5 = md5_hex(&bytes);
            if let Some(k) = known.as_ref().filter(|k| k.md5.as_deref() == Some(md5.as_str()) && !k.id.is_empty()) {
                // Touched, not changed.
                self.files.insert(rel.clone(), Known { local: Some(*s), ..k.clone() });
                continue;
            }
            let updated = match known.as_ref().filter(|k| !k.id.is_empty()) {
                Some(k) => remote.update_file(&k.id, &bytes).ok(),
                None => None,
            };
            let f = match updated {
                Some(f) => f,
                None => {
                    let (parent, name) = rel.rsplit_once('/').unwrap_or(("", rel.as_str()));
                    let parent_id = self.ensure_folder(remote, parent)?;
                    remote.create_file(&parent_id, name, &bytes)?
                }
            };
            self.files.insert(rel.clone(), Known { id: f.id, md5: f.md5.or(Some(md5)), local: Some(*s) });
            out.sent += 1;
        }
        // Deleted here: deleted in Drive.
        let gone: Vec<String> = self
            .files
            .iter()
            .filter(|(r, k)| k.local.is_some() && !ours.contains_key(*r))
            .map(|(r, _)| r.clone())
            .collect();
        for rel in gone {
            let k = self.files.remove(&rel).unwrap_or_default();
            if !k.id.is_empty() {
                if let Err(e) = remote.trash(&k.id) {
                    tracing::warn!("could not remove {rel} from Google Drive: {e}");
                }
                out.sent += 1;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Running it
// ---------------------------------------------------------------------------

/// The Drive folder chosen on this computer (kept with the library settings).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DriveLink {
    /// The parish folder's Drive id.
    pub id: String,
    /// Its name, for showing.
    pub name: String,
}

/// Where this computer keeps its copy of a Drive folder.
pub fn mirror_dir(app: &AppHandle, id: &str) -> CmdResult<PathBuf> {
    let safe: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("google-drive").join(safe))
}

fn state_path(dir: &Path) -> PathBuf {
    dir.with_extension("state.json")
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DriveStatus {
    /// The parish's Google client file is on this computer.
    pub client: bool,
    /// The signed-in account.
    pub account: Option<String>,
    /// The parish folder in use.
    pub folder: Option<String>,
    /// The first full copy is made.
    pub ready: bool,
    /// When the last sync finished (seconds since 1970).
    pub at: Option<u64>,
    pub error: Option<String>,
    pub last: Outcome,
    /// A sync is running now.
    pub busy: bool,
}

static STATUS: Mutex<Option<DriveStatus>> = Mutex::new(None);
static RUNNING: Mutex<()> = Mutex::new(());

fn set_status(app: &AppHandle, f: impl FnOnce(&mut DriveStatus)) {
    {
        let mut s = STATUS.lock().unwrap_or_else(|e| e.into_inner());
        f(s.get_or_insert_with(Default::default));
    }
    let _ = app.emit(EVENT, ());
}

/// Whether the copy of the Drive folder is ready to work from (the first
/// download finished). Until then storage.rs works from this computer.
pub fn mirror_ready() -> bool {
    STATUS.lock().unwrap_or_else(|e| e.into_inner()).as_ref().is_some_and(|s| s.ready)
}

/// One sync, if a Drive folder is chosen and someone is signed in.
pub fn run(app: &AppHandle) -> CmdResult<Option<Outcome>> {
    let Some(link) = crate::storage::settings().drive else { return Ok(None) };
    if load_account(app).is_none() || load_client(app).is_none() {
        return Ok(None);
    }
    let _one = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    let dir = mirror_dir(app, &link.id)?;
    let state = state_path(&dir);
    let mut m: Mirror = std::fs::read_to_string(&state)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .filter(|m: &Mirror| m.root == link.id)
        .unwrap_or_else(|| Mirror::new(&link.id));
    set_status(app, |s| s.busy = true);
    let remote = Http::new(app);
    let result = m.sync(&remote, &dir);
    let _ = std::fs::write(&state, serde_json::to_string(&m).unwrap_or_default());
    let ready = m.ready;
    set_status(app, |s| {
        s.busy = false;
        s.ready = s.ready || ready;
        match &result {
            Ok(o) => {
                s.at = Some(now());
                s.error = None;
                s.last = *o;
            }
            Err(e) => s.error = Some(e.clone()),
        }
    });
    let outcome = result?;
    if outcome.received > 0 {
        // Bring this computer's library copy along (sync.rs), and let the
        // Services screen look again.
        crate::sync::soon();
        let _ = app.emit(crate::sync::LIBRARY_EVENT, ());
    }
    Ok(Some(outcome))
}

static NUDGE: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());

/// Asks for a Drive sync in a moment (after the library or a service changed).
pub fn soon() {
    let (flag, wake) = &NUDGE;
    *flag.lock().unwrap_or_else(|e| e.into_inner()) = true;
    wake.notify_one();
}

/// How often Drive is asked for changes.
const EVERY: Duration = Duration::from_secs(10);

/// Syncs the Drive folder at start-up, every 10 seconds and soon after a
/// change here, apart from the windows.
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || loop {
        match run(&app) {
            Ok(Some(_)) | Ok(None) => {}
            Err(e) => tracing::warn!("Google Drive sync: {e}"),
        }
        let (flag, wake) = &NUDGE;
        let guard = flag.lock().unwrap_or_else(|e| e.into_inner());
        let (mut nudged, _) = wake.wait_timeout_while(guard, EVERY, |n| !*n).unwrap_or_else(|e| e.into_inner());
        let was = *nudged;
        *nudged = false;
        drop(nudged);
        if was {
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn drive_status(app: AppHandle) -> DriveStatus {
    let mut s = STATUS.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default();
    s.client = load_client(&app).is_some();
    s.account = load_account(&app).map(|a| a.email);
    s.folder = crate::storage::settings().drive.map(|d| d.name);
    s
}

/// Adds the parish's Google client file (asked for with a file dialog).
#[tauri::command]
pub async fn drive_import_client(app: AppHandle) -> CmdResult<bool> {
    use tauri_plugin_dialog::DialogExt;
    let Some(picked) = app
        .dialog()
        .file()
        .set_title("Choose the parish's Google client file (JSON)")
        .add_filter("Google client file", &["json"])
        .blocking_pick_file()
    else {
        return Ok(false);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let client = parse_client(&text)?;
    save_private(
        &config_dir(&app)?.join(CLIENT_FILE),
        &serde_json::to_string_pretty(&client).map_err(|e| e.to_string())?,
    )?;
    let _ = app.emit(EVENT, ());
    Ok(true)
}

/// Signs in with Google in the browser; returns the account's address.
#[tauri::command]
pub async fn drive_sign_in(app: AppHandle) -> CmdResult<String> {
    let a = app.clone();
    let email = tauri::async_runtime::spawn_blocking(move || sign_in(&a)).await.map_err(|e| e.to_string())??;
    let _ = app.emit(EVENT, ());
    soon();
    Ok(email)
}

/// Signs out on this computer (the copy of the folder stays).
#[tauri::command]
pub fn drive_sign_out(app: AppHandle) -> CmdResult<()> {
    let path = config_dir(&app)?.join(ACCOUNT_FILE);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    *ACCESS.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let _ = app.emit(EVENT, ());
    Ok(())
}

/// Folders in the signed-in account's Drive (and shared with it).
#[derive(Serialize)]
pub struct DriveFolder {
    id: String,
    name: String,
    /// Who owns it, when it is shared with this account.
    owner: Option<String>,
}

#[tauri::command]
pub async fn drive_folders(app: AppHandle, search: Option<String>) -> CmdResult<Vec<DriveFolder>> {
    tauri::async_runtime::spawn_blocking(move || {
        let list = Http::new(&app).folders(search.as_deref().unwrap_or(""))?;
        Ok(list
            .into_iter()
            .map(|f| DriveFolder {
                id: f.get("id").and_then(Value::as_str).unwrap_or_default().to_string(),
                name: f.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
                owner: (!f.get("ownedByMe").and_then(Value::as_bool).unwrap_or(true))
                    .then(|| f.pointer("/owners/0/displayName").and_then(Value::as_str).map(String::from))
                    .flatten(),
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Uses a Drive folder as the parish library (and sets the user's name).
#[tauri::command]
pub fn drive_use_folder(
    app: AppHandle,
    engine: State<'_, Engine>,
    id: String,
    name: String,
    user: String,
) -> CmdResult<crate::storage::StorageInfo> {
    if load_account(&app).is_none() {
        return Err("Sign in with Google first.".into());
    }
    let dir = mirror_dir(&app, &id)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // A new folder: its copy starts again.
    set_status(&app, |s| {
        s.ready = false;
        s.error = None;
        s.at = None;
    });
    let info = crate::storage::apply(
        &app,
        engine.inner(),
        crate::storage::StorageSettings {
            folder: Some(dir),
            name: user.trim().to_string(),
            drive: Some(DriveLink { id, name }),
        },
    )?;
    soon();
    Ok(info)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Drive in memory.
    #[derive(Default)]
    struct Fake {
        files: Mutex<BTreeMap<String, (RFile, Vec<u8>)>>,
        log: Mutex<Vec<String>>,
        next: AtomicU64,
        clock: AtomicU64,
    }

    impl Fake {
        fn with_root() -> Fake {
            let f = Fake::default();
            f.clock.store(1_000_000, Ordering::Relaxed);
            f.files.lock().unwrap().insert(
                "root".into(),
                (RFile { id: "root".into(), name: "Parish".into(), folder: true, ..Default::default() }, vec![]),
            );
            f
        }
        fn tick(&self) -> u64 {
            self.clock.fetch_add(10, Ordering::Relaxed) + 10
        }
        fn id(&self) -> String {
            format!("f{}", self.next.fetch_add(1, Ordering::Relaxed))
        }
        fn changed(&self, id: &str) {
            self.log.lock().unwrap().push(id.to_string());
        }
        /// Another computer adds or changes a file.
        fn put(&self, parent: &str, name: &str, text: &str) -> String {
            let existing = self
                .files
                .lock()
                .unwrap()
                .values()
                .find(|(f, _)| f.name == name && f.parents == [parent] && !f.trashed)
                .map(|(f, _)| f.id.clone());
            match existing {
                Some(id) => self.update_file(&id, text.as_bytes()).unwrap().id,
                None => self.create_file(parent, name, text.as_bytes()).unwrap().id,
            }
        }
        fn text(&self, parent: &str, name: &str) -> Option<String> {
            self.files
                .lock()
                .unwrap()
                .values()
                .find(|(f, _)| f.name == name && f.parents == [parent] && !f.trashed)
                .map(|(_, b)| String::from_utf8_lossy(b).into_owned())
        }
        fn count(&self, name: &str) -> usize {
            self.files.lock().unwrap().values().filter(|(f, _)| f.name == name && !f.trashed).count()
        }
        fn folder(&self, parent: &str, name: &str) -> String {
            self.files
                .lock()
                .unwrap()
                .values()
                .find(|(f, _)| f.folder && f.name == name && f.parents == [parent] && !f.trashed)
                .map(|(f, _)| f.id.clone())
                .expect("folder")
        }
    }

    impl Remote for Fake {
        fn list(&self, folder: &str) -> Result<Vec<RFile>, String> {
            Ok(self
                .files
                .lock()
                .unwrap()
                .values()
                .filter(|(f, _)| f.parents.iter().any(|p| p == folder) && !f.trashed)
                .map(|(f, _)| f.clone())
                .collect())
        }
        fn start_token(&self) -> Result<String, String> {
            Ok(self.log.lock().unwrap().len().to_string())
        }
        fn changes(&self, token: &str) -> Result<(Vec<RChange>, String), String> {
            let log = self.log.lock().unwrap().clone();
            let from: usize = token.parse().unwrap_or(0);
            let files = self.files.lock().unwrap();
            let out = log[from.min(log.len())..]
                .iter()
                .map(|id| RChange { id: id.clone(), removed: false, file: files.get(id).map(|(f, _)| f.clone()) })
                .collect();
            Ok((out, log.len().to_string()))
        }
        fn download(&self, id: &str) -> Result<Vec<u8>, String> {
            self.files.lock().unwrap().get(id).map(|(_, b)| b.clone()).ok_or("not found".into())
        }
        fn create_folder(&self, parent: &str, name: &str) -> Result<RFile, String> {
            let f = RFile {
                id: self.id(),
                name: name.into(),
                folder: true,
                parents: vec![parent.into()],
                modified: self.tick(),
                ..Default::default()
            };
            self.files.lock().unwrap().insert(f.id.clone(), (f.clone(), vec![]));
            self.changed(&f.id);
            Ok(f)
        }
        fn create_file(&self, parent: &str, name: &str, bytes: &[u8]) -> Result<RFile, String> {
            let f = RFile {
                id: self.id(),
                name: name.into(),
                md5: Some(md5_hex(bytes)),
                parents: vec![parent.into()],
                modified: self.tick(),
                ..Default::default()
            };
            self.files.lock().unwrap().insert(f.id.clone(), (f.clone(), bytes.to_vec()));
            self.changed(&f.id);
            Ok(f)
        }
        fn update_file(&self, id: &str, bytes: &[u8]) -> Result<RFile, String> {
            let t = self.tick();
            let mut files = self.files.lock().unwrap();
            let (f, b) = files.get_mut(id).ok_or("not found")?;
            f.md5 = Some(md5_hex(bytes));
            f.modified = t;
            *b = bytes.to_vec();
            let f = f.clone();
            drop(files);
            self.changed(id);
            Ok(f)
        }
        fn trash(&self, id: &str) -> Result<(), String> {
            self.files.lock().unwrap().get_mut(id).ok_or("not found")?.0.trashed = true;
            self.changed(id);
            Ok(())
        }
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("episco-drive-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Writes a file here, dated in the past so it is not "still being written".
    fn put_here(dir: &Path, rel: &str, text: &str) {
        let p = local_path(dir, rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        let f = std::fs::File::options().write(true).open(&p).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(5)).unwrap();
    }

    fn read_here(dir: &Path, rel: &str) -> Option<String> {
        std::fs::read_to_string(local_path(dir, rel)).ok()
    }

    #[test]
    fn times_from_drive() {
        assert_eq!(parse_time("1970-01-01T00:00:00.000Z"), 0);
        assert_eq!(parse_time("2026-11-16T00:45:06.123Z"), 1_794_789_906);
        assert_eq!(parse_time(""), 0);
    }

    #[test]
    fn client_files_as_google_sends_them() {
        let c = parse_client(r#"{"installed":{"client_id":"123.apps.googleusercontent.com","client_secret":"GOCSPX-abc","redirect_uris":["http://localhost"]}}"#).unwrap();
        assert_eq!(c.client_id, "123.apps.googleusercontent.com");
        assert!(parse_client(r#"{"installed":{}}"#).is_err());
        assert!(parse_client("not json").is_err());
    }

    #[test]
    fn the_folder_comes_down_and_changes_go_both_ways() {
        let drive = Fake::with_root();
        let lib = drive.create_folder("root", "library").unwrap().id;
        drive.put(&lib, "hymn-436.ewm", "Lift up your heads");
        drive.put("root", "parish.json", "{}");
        let a = tmp("a");
        let mut m = Mirror::new("root");

        // The first time: everything comes down.
        let o = m.sync(&drive, &a).unwrap();
        assert_eq!(o.received, 2);
        assert_eq!(read_here(&a, "library/hymn-436.ewm").as_deref(), Some("Lift up your heads"));
        assert!(m.ready);

        // Nothing changed: nothing moves (this computer's uploads are not
        // downloaded again either).
        assert_eq!(m.sync(&drive, &a).unwrap(), Outcome::default());

        // A new file here goes up, into a new folder made for it.
        put_here(&a, "services/2026-11-22/service.toml", "title = \"T\"");
        let o = m.sync(&drive, &a).unwrap();
        assert_eq!(o.sent, 1);
        let services = drive.folder("root", "services");
        let day = drive.folder(&services, "2026-11-22");
        assert_eq!(drive.text(&day, "service.toml").as_deref(), Some("title = \"T\""));
        assert_eq!(m.sync(&drive, &a).unwrap(), Outcome::default(), "its own upload is not downloaded again");

        // A change made elsewhere comes down.
        drive.put(&lib, "hymn-436.ewm", "Lift up your heads, ye mighty gates");
        assert_eq!(m.sync(&drive, &a).unwrap().received, 1);
        assert_eq!(read_here(&a, "library/hymn-436.ewm").as_deref(), Some("Lift up your heads, ye mighty gates"));

        // A change here goes up (the same file, not a copy).
        put_here(&a, "parish.json", "{\"name\":\"St. James\"}");
        assert_eq!(m.sync(&drive, &a).unwrap().sent, 1);
        assert_eq!(drive.text("root", "parish.json").as_deref(), Some("{\"name\":\"St. James\"}"));
        assert_eq!(drive.count("parish.json"), 1);

        // Deleted here: deleted there; deleted there: deleted here.
        std::fs::remove_file(local_path(&a, "parish.json")).unwrap();
        m.sync(&drive, &a).unwrap();
        assert_eq!(drive.count("parish.json"), 0);
        let id = drive.files.lock().unwrap().values().find(|(f, _)| f.name == "hymn-436.ewm").unwrap().0.id.clone();
        drive.trash(&id).unwrap();
        m.sync(&drive, &a).unwrap();
        assert!(read_here(&a, "library/hymn-436.ewm").is_none());
        let _ = std::fs::remove_dir_all(a);
    }

    #[test]
    fn a_second_computer_and_the_full_comparison() {
        let drive = Fake::with_root();
        let lib = drive.create_folder("root", "library").unwrap().id;
        drive.put(&lib, "a.ewm", "A");
        // The second computer already has some files (e.g. from Drive for
        // desktop): the same one stays, its own goes up, Drive's comes down.
        let b = tmp("b");
        put_here(&b, "library/a.ewm", "A");
        put_here(&b, "library/b.ewm", "B");
        let mut m = Mirror::new("root");
        let o = m.sync(&drive, &b).unwrap();
        assert_eq!((o.received, o.sent), (0, 1));
        assert_eq!(drive.text(&lib, "b.ewm").as_deref(), Some("B"));
        assert_eq!(drive.count("a.ewm"), 1, "no copy of a file both had");

        // Changed in both places: the later change wins.
        drive.put(&lib, "a.ewm", "A from Drive");
        put_here(&b, "library/a.ewm", "A from here");
        m.sync(&drive, &b).unwrap();
        assert_eq!(
            read_here(&b, "library/a.ewm").as_deref(),
            Some("A from here"),
            "ours is later (the fake clock is in 1970)"
        );
        assert_eq!(drive.text(&lib, "a.ewm").as_deref(), Some("A from here"));

        // A full comparison after a missed change still finds it.
        drive.files.lock().unwrap().values_mut().find(|(f, _)| f.name == "b.ewm").unwrap().1 = b"B2".to_vec();
        drive.files.lock().unwrap().values_mut().find(|(f, _)| f.name == "b.ewm").unwrap().0.md5 = Some(md5_hex(b"B2"));
        m.ready = false;
        m.sync(&drive, &b).unwrap();
        assert_eq!(read_here(&b, "library/b.ewm").as_deref(), Some("B2"));
        let _ = std::fs::remove_dir_all(b);
    }

    #[test]
    fn google_docs_and_odd_names_are_left_alone() {
        let drive = Fake::with_root();
        drive.files.lock().unwrap().insert(
            "doc".into(),
            (
                RFile {
                    id: "doc".into(),
                    name: "Notes".into(),
                    native: true,
                    parents: vec!["root".into()],
                    ..Default::default()
                },
                vec![],
            ),
        );
        drive.put("root", "a/b.txt", "slash");
        drive.put("root", ".DS_Store", "x");
        let c = tmp("c");
        let mut m = Mirror::new("root");
        m.sync(&drive, &c).unwrap();
        assert!(walk(&c).is_empty(), "{:?}", walk(&c));
        let _ = std::fs::remove_dir_all(c);
    }
}
