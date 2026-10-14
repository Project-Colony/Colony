use anyhow::Result;
use keyring_core::{CredentialStore, Entry};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// GitHub OAuth App Client ID.
/// App settings: https://github.com/settings/applications/new
/// - Homepage URL: https://github.com/MotherSphere
/// - Device Flow must be enabled in the app settings.
pub const CLIENT_ID: &str = "Ov23liW0bM8skbk585D9";

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

/// Stored OAuth session.
#[derive(Clone)]
pub struct OAuthSession {
    pub access_token: String,
    pub username: Option<String>,
}

/// Hand-written so the token can never be printed. `Message` derives Debug and
/// carries an `OAuthSession` in `GitHubLoginCompleted`, so the single most
/// natural debugging line anyone will ever add to `App::update` -
/// `tracing::debug!(?message)` - would otherwise put the plaintext token on
/// stderr and into the log file for any user running with RUST_LOG=debug. The
/// redaction is inherited by every type that contains a session.
impl std::fmt::Debug for OAuthSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthSession")
            .field("access_token", &"<redacted>")
            .field("username", &self.username)
            .finish()
    }
}

/// Pending device code, returned by the first step of the flow.
#[derive(Debug, Clone)]
pub struct DeviceCode {
    pub user_code: String,
    /// Shown in the panel so the flow survives a browser that would not open.
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

    // Open browser with pre-filled user code. The URI comes from the device-flow
    // response, so it goes through the same http(s)-only gate as every other
    // string that reaches the desktop URI opener.
    let url = format!("{}?user_code={}", device.verification_uri, device.user_code);
    match crate::download::web_url(&url) {
        Some(safe) => {
            if let Err(e) = open::that(&safe) {
                // Not fatal: the panel shows the URL and the code, and the poll
                // keeps running until the device code expires.
                tracing::warn!("could not open the verification page: {e}");
            }
        }
        None => tracing::warn!("refusing to open non-http(s) verification uri {url:?}"),
    }

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
// The OS credential store (keyring 4) first, a 0600 file as the fallback.

const KEYRING_SERVICE: &str = "colony-launcher";
const KEYRING_USER: &str = "github-oauth";

/// The OS credential store for this target.
///
/// Built per call rather than once: the three callers run once per launch,
/// login and logout, and a Secret Service daemon that starts after Colony is
/// then picked up instead of being missed for the whole session. There is no
/// arm for other targets on purpose: keyring 3 quietly fell back to an
/// in-memory mock there, so a missing store is now a build error instead.
fn os_store() -> keyring_core::Result<Arc<CredentialStore>> {
    #[cfg(target_os = "macos")]
    let store = apple_native_keyring_store::keychain::Store::new()?;
    #[cfg(windows)]
    let store = windows_native_keyring_store::Store::new()?;
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    let store = dbus_secret_service_keyring_store::Store::new()?;
    Ok(store)
}

/// The token's entry in the OS credential store. The service and user match
/// keyring 3, whose stores used the same identifiers on every OS, so a token
/// saved before the keyring 4 upgrade is read and overwritten in place.
fn os_entry() -> keyring_core::Result<Entry> {
    os_store()?.build(KEYRING_SERVICE, KEYRING_USER, None)
}

fn token_path() -> PathBuf {
    match crate::persistence::colony_data_dir() {
        Ok(dir) => {
            let auth_dir = dir.join("auth");
            let _ = std::fs::create_dir_all(&auth_dir);
            auth_dir.join("github_token.json")
        }
        // Only the creation failed; ask for the same path without creating it
        // rather than rebuilding the layout by hand - spelling it out here with
        // dirs::config_dir() put the token in Roaming on Windows, which is the
        // one place the shared layout says it must not be.
        Err(_) => colony_ui::paths::locate::config_dir("Colony")
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("auth")
            .join("github_token.json"),
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredToken {
    access_token: String,
    username: Option<String>,
}

fn save_token(session: &OAuthSession) -> Result<()> {
    save_token_with(os_entry(), &token_path(), session)
}

fn save_token_with(
    entry: keyring_core::Result<Entry>,
    path: &Path,
    session: &OAuthSession,
) -> Result<()> {
    let stored = StoredToken {
        access_token: session.access_token.clone(),
        username: session.username.clone(),
    };
    let json = serde_json::to_string_pretty(&stored)?;

    match store_verified(entry, &json) {
        Ok(()) => {
            tracing::info!("Token saved to the OS credential store");
            // The store holds a copy that read back intact: never leave a
            // plaintext one on disk.
            if path.exists() {
                if let Err(e) = std::fs::remove_file(path) {
                    tracing::warn!("could not delete the plaintext token file: {e}");
                }
            }
            return Ok(());
        }
        Err(why) => tracing::warn!("{why}; using the file fallback"),
    }

    // Fallback only: write the token to a file created with 0600 from the start
    // (no world-readable window between create and chmod).
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_private(path, json.as_bytes())?;
    tracing::warn!("Token saved to plaintext file fallback: {}", path.display());
    Ok(())
}

/// Store the secret, then read it back. Only an identical read-back counts as
/// saved: an Ok from `set_password` alone is what keyring 3's mock returned
/// while keeping nothing, and trusting it cost users the plaintext copy.
fn store_verified(entry: keyring_core::Result<Entry>, secret: &str) -> Result<(), String> {
    let entry = entry.map_err(|e| format!("OS credential store unavailable ({e})"))?;
    entry
        .set_password(secret)
        .map_err(|e| format!("OS credential store refused the token ({e})"))?;
    match entry.get_password() {
        Ok(back) if back == secret => Ok(()),
        Ok(_) => Err("OS credential store returned a different token on read-back".into()),
        Err(e) => Err(format!(
            "OS credential store lost the token on read-back ({e})"
        )),
    }
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
        // create_new, not create: with `create`, a symlink that wins the race
        // between the unlink above and this open is FOLLOWED, and `.mode()`
        // does not apply to an already-existing inode - so the token would be
        // written through the link, at the link target's permissions. The
        // unlink makes create_new succeed on the normal path, and because it
        // guarantees a fresh inode, the mode it sets is the mode that sticks
        // (the old set_permissions re-assertion chmod'd the link's target).
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(contents)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, contents)?;
        Ok(())
    }
}

/// Try to load a previously stored token.
/// Checks the OS credential store first, then falls back to the file.
pub fn load_saved_token() -> Option<OAuthSession> {
    load_token_with(os_entry(), &token_path())
}

fn load_token_with(entry: keyring_core::Result<Entry>, path: &Path) -> Option<OAuthSession> {
    match entry.and_then(|entry| entry.get_password()) {
        Ok(json) => match serde_json::from_str::<StoredToken>(&json) {
            Ok(stored) => {
                tracing::debug!("Token loaded from the OS credential store");
                return Some(OAuthSession {
                    access_token: stored.access_token,
                    username: stored.username,
                });
            }
            Err(e) => {
                tracing::warn!("ignoring an unreadable token in the OS credential store: {e}")
            }
        },
        Err(keyring_core::Error::NoEntry) => {}
        Err(e) => tracing::debug!("OS credential store not read ({e}); trying the file"),
    }

    let content = std::fs::read_to_string(path).ok()?;
    let stored: StoredToken = serde_json::from_str(&content).ok()?;
    tracing::debug!("Token loaded from file: {}", path.display());
    Some(OAuthSession {
        access_token: stored.access_token,
        username: stored.username,
    })
}

/// Delete the stored token (logout).
/// Forget the stored credential, reporting whether it actually went away.
///
/// `save_token` checks and logs every keyring outcome, and deliberately deletes
/// the plaintext fallback once the keychain accepted the secret. The delete path
/// used to get none of that care: the keyring result was dropped on the floor,
/// so a failed deletion still reported "Disconnected" and the next launch
/// silently restored the session from the credential the user just revoked.
pub fn delete_saved_token() -> Result<()> {
    delete_token_with(os_entry(), &token_path())
}

fn delete_token_with(entry: keyring_core::Result<Entry>, path: &Path) -> Result<()> {
    let mut failure: Option<String> = None;

    match entry.and_then(|entry| entry.delete_credential()) {
        // Nothing stored is the desired end state, not an error.
        Ok(()) | Err(keyring_core::Error::NoEntry) => {}
        Err(e) => {
            tracing::warn!("could not delete the credential from the OS store: {e}");
            failure = Some(e.to_string());
        }
    }

    if path.exists() {
        if let Err(e) = std::fs::remove_file(path) {
            tracing::warn!("could not delete the token file: {e}");
            failure = Some(e.to_string());
        }
    }

    match failure {
        None => {
            tracing::info!("Token deleted");
            Ok(())
        }
        Some(e) => anyhow::bail!("{e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyring_core::api::{CredentialApi, CredentialStoreApi};
    use keyring_core::{mock, Credential, Error};
    use std::any::Any;

    // The mock store is used here and nowhere else: production code never
    // sets a default store, and `os_store` has no mock arm.
    fn mock_entry(store: &Arc<mock::Store>) -> keyring_core::Result<Entry> {
        store.build(KEYRING_SERVICE, KEYRING_USER, None)
    }

    fn session(token: &str) -> OAuthSession {
        OAuthSession {
            access_token: token.to_string(),
            username: Some("octocat".to_string()),
        }
    }

    /// keyring 3's trap, reproduced: the write answers Ok and keeps nothing.
    #[derive(Debug)]
    struct Forgetful;

    impl CredentialApi for Forgetful {
        fn set_secret(&self, _: &[u8]) -> keyring_core::Result<()> {
            Ok(())
        }
        fn get_secret(&self) -> keyring_core::Result<Vec<u8>> {
            Err(Error::NoEntry)
        }
        fn delete_credential(&self) -> keyring_core::Result<()> {
            Err(Error::NoEntry)
        }
        fn get_credential(&self) -> keyring_core::Result<Option<Arc<Credential>>> {
            Ok(None)
        }
        fn get_specifiers(&self) -> Option<(String, String)> {
            None
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn os_store_is_the_native_store_never_the_mock() {
        let expected = if cfg!(target_os = "macos") {
            "macOS Keychain"
        } else if cfg!(windows) {
            "Windows Credential Manager"
        } else {
            "Secret Service"
        };
        match os_store() {
            Ok(store) => assert!(
                store.vendor().starts_with(expected),
                "unexpected credential store: {}",
                store.vendor()
            ),
            // The Keychain and Credential Manager stores connect to nothing,
            // so they must always build.
            Err(e) if cfg!(any(target_os = "macos", windows)) => {
                panic!("the native store failed to build: {e}")
            }
            // Building the Secret Service store connects to the session bus,
            // which a CI runner may not have.
            Err(_) => {}
        }
    }

    #[test]
    fn verified_save_removes_the_plaintext_copy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("github_token.json");
        std::fs::write(&path, "stale").unwrap();
        let store = mock::Store::new().unwrap();

        save_token_with(mock_entry(&store), &path, &session("gho_new")).unwrap();

        assert!(
            !path.exists(),
            "the plaintext copy must go once the store holds the token"
        );
        let loaded = load_token_with(mock_entry(&store), &path).unwrap();
        assert_eq!(loaded.access_token, "gho_new");
        assert_eq!(loaded.username.as_deref(), Some("octocat"));
    }

    #[test]
    fn a_write_that_does_not_read_back_keeps_the_plaintext_copy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("github_token.json");
        let forgetful = Entry::new_with_credential(Arc::new(Forgetful));

        save_token_with(Ok(forgetful), &path, &session("gho_kept")).unwrap();

        let loaded = load_token_with(Err(Error::NoDefaultStore), &path).unwrap();
        assert_eq!(loaded.access_token, "gho_kept");
    }

    #[test]
    fn a_refused_write_keeps_the_plaintext_copy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("github_token.json");
        let store = mock::Store::new().unwrap();
        let entry = mock_entry(&store).unwrap();
        let cred: &mock::Cred = entry.as_any().downcast_ref().unwrap();
        cred.set_error(Error::NoStorageAccess("locked".into()));

        save_token_with(Ok(entry), &path, &session("gho_file")).unwrap();

        let loaded = load_token_with(Err(Error::NoDefaultStore), &path).unwrap();
        assert_eq!(loaded.access_token, "gho_file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn logout_clears_both_copies_and_tolerates_an_empty_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("github_token.json");
        let store = mock::Store::new().unwrap();
        mock_entry(&store)
            .unwrap()
            .set_password("{\"access_token\":\"gho_x\",\"username\":null}")
            .unwrap();
        std::fs::write(&path, "{\"access_token\":\"gho_x\",\"username\":null}").unwrap();

        delete_token_with(mock_entry(&store), &path).unwrap();

        assert!(!path.exists());
        assert!(load_token_with(mock_entry(&store), &path).is_none());
        // Nothing left to delete is the desired end state, not a failure.
        delete_token_with(mock_entry(&store), &path).unwrap();
    }
}
