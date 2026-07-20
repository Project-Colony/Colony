use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

/// GitHub OAuth App Client ID.
/// App settings: https://github.com/settings/applications/new
/// - Homepage URL: https://github.com/MotherSphere
/// - Device Flow must be enabled in the app settings.
pub const CLIENT_ID: &str = "Ov23liW0bM8skbk585D9";

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

/// Stored OAuth session.
#[derive(Debug, Clone)]
pub struct OAuthSession {
    pub access_token: String,
    pub username: Option<String>,
}

/// Pending device code, returned by the first step of the flow.
#[derive(Debug, Clone)]
pub struct DeviceCode {
    pub user_code: String,
    #[allow(dead_code)]
    pub verification_uri: String,
    pub(crate) device_code: String,
    pub(crate) expires_in: u64,
    pub(crate) interval: u64,
}

/// Response from the device code request.
#[derive(serde::Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

/// Response from the token polling request.
#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Step 1: Request a device code from GitHub and open the browser.
/// Returns the device code info so the UI can display the user_code.
pub async fn request_device_code() -> Result<DeviceCode> {
    let client = http_client()?;

    let resp = client
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", CLIENT_ID), ("scope", "read:user")])
        .send()
        .await?;

    let device: DeviceCodeResponse = resp.json().await?;

    tracing::info!(
        "Enter code {} at {}",
        device.user_code,
        device.verification_uri
    );

    // Open browser with pre-filled user code
    let url = format!("{}?user_code={}", device.verification_uri, device.user_code);
    let _ = open::that(&url);

    Ok(DeviceCode {
        user_code: device.user_code,
        verification_uri: device.verification_uri,
        device_code: device.device_code,
        expires_in: device.expires_in,
        interval: device.interval,
    })
}

/// Step 2: Poll GitHub until the user authorizes (or timeout).
/// Call this after `request_device_code`.
/// The OAuth HTTP client. One builder instead of three copies - and a build
/// failure now propagates instead of silently falling back to a client with
/// NO timeout (which turned a 30s bound into a potential forever-hang).
fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(format!("Colony-Launcher/{}", crate::github::APP_VERSION))
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()?)
}

pub async fn poll_for_token(device: DeviceCode) -> Result<OAuthSession> {
    let client = http_client()?;

    let poll_interval = Duration::from_secs(device.interval.max(5));
    let mut consecutive_errors: u32 = 0;
    let deadline = std::time::Instant::now() + Duration::from_secs(device.expires_in);

    let access_token = loop {
        tokio::time::sleep(poll_interval).await;

        if std::time::Instant::now() > deadline {
            anyhow::bail!("{}", crate::i18n::t("oauth_device_expired"));
        }

        // One WiFi blip used to kill the whole login even though the device
        // code was still valid; tolerate a few consecutive transient errors.
        let token_resp: TokenResponse = match async {
            client
                .post(TOKEN_URL)
                .header("Accept", "application/json")
                .form(&[
                    ("client_id", CLIENT_ID),
                    ("device_code", device.device_code.as_str()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ])
                .send()
                .await?
                .json::<TokenResponse>()
                .await
        }
        .await
        {
            Ok(r) => {
                consecutive_errors = 0;
                r
            }
            Err(e) => {
                consecutive_errors += 1;
                if consecutive_errors >= 3 {
                    return Err(e.into());
                }
                tracing::warn!("transient error while polling for token (retrying): {e}");
                continue;
            }
        };

        match token_resp.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
            Some(err) => {
                let desc = token_resp.error_description.unwrap_or_default();
                anyhow::bail!(
                    "{}",
                    crate::i18n::t_fmt("oauth_device_failed", &[("error", err), ("desc", &desc)])
                );
            }
            None => {}
        }

        if let Some(token) = token_resp.access_token {
            break token;
        }
    };

    let username = fetch_username(&access_token).await.ok();

    let session = OAuthSession {
        access_token,
        username,
    };

    save_token(&session)?;

    Ok(session)
}

/// Fetch the authenticated user's login name.
async fn fetch_username(token: &str) -> Result<String> {
    let client = http_client()?;
    let resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "Colony-Launcher/0.1")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;

    #[derive(serde::Deserialize)]
    struct User {
        login: String,
    }

    let user: User = resp.json().await?;
    Ok(user.login)
}

// --- Token persistence ---
// Uses OS keychain (via keyring crate) with file fallback.

const KEYRING_SERVICE: &str = "colony-launcher";
const KEYRING_USER: &str = "github-oauth";

fn token_path() -> PathBuf {
    match crate::github::colony_data_dir() {
        Ok(dir) => {
            let auth_dir = dir.join("auth");
            let _ = std::fs::create_dir_all(&auth_dir);
            auth_dir.join("github_token.json")
        }
        Err(_) => {
            let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
            base.join("Colony")
                .join("Colony")
                .join("auth")
                .join("github_token.json")
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredToken {
    access_token: String,
    username: Option<String>,
}

fn save_token(session: &OAuthSession) -> Result<()> {
    let stored = StoredToken {
        access_token: session.access_token.clone(),
        username: session.username.clone(),
    };
    let json = serde_json::to_string_pretty(&stored)?;

    // Try the OS keychain first.
    let keychain_ok = match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => match entry.set_password(&json) {
            Ok(()) => {
                tracing::info!("Token saved to OS keychain");
                true
            }
            Err(e) => {
                tracing::warn!("Keychain unavailable ({e}); using file fallback");
                false
            }
        },
        Err(e) => {
            tracing::warn!("Keychain error ({e}); using file fallback");
            false
        }
    };

    let path = token_path();
    if keychain_ok {
        // Keychain holds the secret — never leave a plaintext copy on disk.
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        return Ok(());
    }

    // Fallback only: write the token to a file created with 0600 from the start
    // (no world-readable window between create and chmod).
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_private(&path, json.as_bytes())?;
    tracing::warn!("Token saved to plaintext file fallback: {}", path.display());
    Ok(())
}

/// Write a file that is owner-only (0600) from creation on Unix, avoiding the
/// world-readable window of write-then-chmod.
fn write_private(path: &std::path::Path, contents: &[u8]) -> Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        // Remove any pre-existing file first so `.mode(0o600)` on create always
        // applies (a truncated-open of a 0644 file would keep the loose bits).
        let _ = std::fs::remove_file(path);
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(contents)?;
        // Re-assert perms in case the file pre-existed with looser bits.
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, contents)?;
        Ok(())
    }
}

/// Try to load a previously stored token.
/// Checks OS keychain first, then falls back to file.
pub fn load_saved_token() -> Option<OAuthSession> {
    // Try keychain first
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        if let Ok(json) = entry.get_password() {
            if let Ok(stored) = serde_json::from_str::<StoredToken>(&json) {
                tracing::debug!("Token loaded from OS keychain");
                return Some(OAuthSession {
                    access_token: stored.access_token,
                    username: stored.username,
                });
            }
        }
    }

    // Fallback: file-based
    let path = token_path();
    let content = std::fs::read_to_string(&path).ok()?;
    let stored: StoredToken = serde_json::from_str(&content).ok()?;
    tracing::debug!("Token loaded from file: {}", path.display());
    Some(OAuthSession {
        access_token: stored.access_token,
        username: stored.username,
    })
}

/// Delete the stored token (logout).
pub fn delete_saved_token() -> Result<()> {
    // Remove from keychain
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.delete_credential();
    }
    // Remove file too
    let path = token_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    tracing::info!("Token deleted");
    Ok(())
}
