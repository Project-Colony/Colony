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
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = client
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", CLIENT_ID), ("scope", "read:user")])
        .send()
        .await?;

    let device: DeviceCodeResponse = resp.json().await?;

    tracing::info!("Enter code {} at {}", device.user_code, device.verification_uri);

    // Open browser with pre-filled user code
    let url = format!(
        "{}?user_code={}",
        device.verification_uri, device.user_code
    );
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
pub async fn poll_for_token(device: DeviceCode) -> Result<OAuthSession> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let poll_interval = Duration::from_secs(device.interval.max(5));
    let deadline =
        std::time::Instant::now() + Duration::from_secs(device.expires_in);

    let access_token = loop {
        tokio::time::sleep(poll_interval).await;

        if std::time::Instant::now() > deadline {
            anyhow::bail!("Device flow expired — the user did not authorize in time");
        }

        let resp = client
            .post(TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", CLIENT_ID),
                ("device_code", device.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;

        let token_resp: TokenResponse = resp.json().await?;

        match token_resp.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
            Some(err) => {
                let desc = token_resp.error_description.unwrap_or_default();
                anyhow::bail!("Device flow failed: {err} — {desc}");
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
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
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
            base.join("Colony").join("Colony").join("auth").join("github_token.json")
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

    // Try keychain
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => {
            if entry.set_password(&json).is_ok() {
                tracing::info!("Token saved to OS keychain");
            } else {
                tracing::warn!("Keychain unavailable");
            }
        }
        Err(e) => {
            tracing::warn!("Keychain error: {e}");
        }
    }

    // Always save file-based backup alongside keychain
    let path = token_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    tracing::info!("Token saved to {}", path.display());
    Ok(())
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
