//! "Connect with GitHub": GitHub's device flow, so a player gets the token "Propose" uses without
//! making one by hand. The Studio asks GitHub for a short code, the player types it at
//! github.com/login/device and approves, and the token GitHub then hands over is checked and kept
//! in the Credential Manager exactly like a pasted one. The app's client id is public by design;
//! there is no client secret anywhere.

use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// PZ Mod Studio's OAuth app on GitHub (device flow on, no secret).
pub const CLIENT_ID: &str = "Ov23liFK37O7B9WkDAoa";
/// Forking a public repository, a branch on the fork and a pull request need this, and no more.
const SCOPE: &str = "public_repo";

#[derive(Debug, Clone, Serialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

struct Pending {
    device_code: String,
    interval: u64,
    until: Instant,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);
static CANCEL: AtomicBool = AtomicBool::new(false);

fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().provider(ureq::tls::TlsProvider::NativeTls).build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(format!("PZModStudio/{}", env!("CARGO_PKG_VERSION")))
        .http_status_as_error(false)
        .build()
        .into()
}

fn post(url: &str, form: &[(&str, &str)]) -> Result<Value, String> {
    let mut response = agent()
        .post(url)
        .header("Accept", "application/json")
        .send_form(form.iter().copied())
        .map_err(|e| format!("unreachable: GitHub could not be reached: {}", e))?;
    let text = response.body_mut().read_to_string().unwrap_or_default();
    Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
}

/// Asks GitHub for a code to show the player.
pub fn start() -> Result<DeviceCode, String> {
    CANCEL.store(false, Ordering::Relaxed);
    let v = post("https://github.com/login/device/code", &[("client_id", CLIENT_ID), ("scope", SCOPE)])?;
    let field = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    let (Some(device_code), Some(user_code)) = (field("device_code"), field("user_code")) else {
        let why = field("error_description").or_else(|| field("error")).unwrap_or_else(|| "no code came back".into());
        return Err(format!("refused: GitHub gave no code: {}", why));
    };
    let expires_in = v.get("expires_in").and_then(Value::as_u64).unwrap_or(900);
    let interval = v.get("interval").and_then(Value::as_u64).unwrap_or(5).max(1);
    if let Ok(mut p) = PENDING.lock() {
        *p = Some(Pending { device_code, interval, until: Instant::now() + Duration::from_secs(expires_in) });
    }
    Ok(DeviceCode { user_code, verification_uri: field("verification_uri").unwrap_or_else(|| "https://github.com/login/device".into()), expires_in })
}

/// What one look at GitHub says.
#[derive(Debug, PartialEq)]
pub enum Answer {
    Token(String),
    Wait,
    SlowDown,
    Failed(String),
}

pub fn read_answer(v: &Value) -> Answer {
    if let Some(token) = v.get("access_token").and_then(Value::as_str).filter(|t| !t.is_empty()) {
        return Answer::Token(token.to_string());
    }
    match v.get("error").and_then(Value::as_str).unwrap_or("") {
        "authorization_pending" => Answer::Wait,
        "slow_down" => Answer::SlowDown,
        "expired_token" => Answer::Failed("expired: The code expired; press Connect again.".into()),
        "access_denied" => Answer::Failed("denied: The connection was cancelled on GitHub.".into()),
        other => Answer::Failed(format!("refused: GitHub said {}", v.get("error_description").and_then(Value::as_str).unwrap_or(if other.is_empty() { "nothing" } else { other }))),
    }
}

/// Waits for the player to approve on GitHub, then keeps the token; returns the account's login.
pub fn wait() -> Result<String, String> {
    let Some(mut pending) = PENDING.lock().ok().and_then(|mut p| p.take()) else {
        return Err("expired: There is no code waiting; press Connect again.".into());
    };
    loop {
        for _ in 0..pending.interval * 10 {
            if CANCEL.load(Ordering::Relaxed) {
                return Err("cancelled: Cancelled.".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if Instant::now() >= pending.until {
            return Err("expired: The code expired; press Connect again.".into());
        }
        let v = post(
            "https://github.com/login/oauth/access_token",
            &[("client_id", CLIENT_ID), ("device_code", &pending.device_code), ("grant_type", "urn:ietf:params:oauth:grant-type:device_code")],
        )?;
        match read_answer(&v) {
            Answer::Token(token) => return super::propose::save_token(&token),
            Answer::Wait => {}
            Answer::SlowDown => pending.interval += 5,
            Answer::Failed(e) => return Err(e),
        }
    }
}

pub fn cancel() {
    CANCEL.store(true, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn github_answers_are_read() {
        assert_eq!(read_answer(&json!({"access_token": "gho_x", "token_type": "bearer"})), Answer::Token("gho_x".into()));
        assert_eq!(read_answer(&json!({"error": "authorization_pending"})), Answer::Wait);
        assert_eq!(read_answer(&json!({"error": "slow_down", "interval": 10})), Answer::SlowDown);
        assert!(matches!(read_answer(&json!({"error": "access_denied"})), Answer::Failed(e) if e.starts_with("denied:")));
        assert!(matches!(read_answer(&json!({"error": "device_flow_disabled", "error_description": "Device Flow must be explicitly enabled"})), Answer::Failed(e) if e.contains("explicitly enabled")));
    }

    /// Asks GitHub for a real code with the app's client id (proves the app exists and has the
    /// device flow on). cargo test --release device_code_is_given -- --ignored --nocapture
    #[test]
    #[ignore]
    fn device_code_is_given() {
        let code = start().expect("GitHub gives a code");
        println!("{} at {}", code.user_code, code.verification_uri);
        assert!(code.user_code.contains('-'));
    }
}

#[cfg(test)]
mod live {
    /// The saved token's account and scopes (never the token). cargo test --release saved_token_works -- --ignored --nocapture
    #[test]
    #[ignore]
    fn saved_token_works() {
        let token = crate::secrets::get(super::super::propose::GITHUB_TOKEN).unwrap().expect("a token is saved");
        let mut r = super::agent().get("https://api.github.com/user").header("Authorization", &format!("Bearer {}", token)).call().unwrap();
        let kind = token.split('_').next().unwrap_or("").chars().take(10).collect::<String>();
        let scopes = r.headers().get("x-oauth-scopes").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        let v: serde_json::Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        println!("status {} login {} ({}) scopes [{}]", r.status(), v["login"], kind, scopes);
    }
}
