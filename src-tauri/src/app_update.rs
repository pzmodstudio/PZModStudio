//! Whether a newer PZ Mod Studio is out: the repository's latest release on GitHub, asked at most
//! every six hours while the check is on (Settings); the app asks at start and again every six
//! hours while it stays open. Nothing is downloaded here: the notice offers "Update now"
//! (`self_update`) and the release page.

use serde::{Deserialize, Serialize};
use std::fs;

use crate::load_order::mod_info::get_existing_user_zomboid_dirs;

/// A file beside the mod list.
fn cache_path(user_zomboid_dir: &str, file: &str) -> Option<std::path::PathBuf> {
    get_existing_user_zomboid_dirs(user_zomboid_dir).into_iter().next().map(|d| d.join(file))
}

/// One small file over HTTPS, with a ten-second patience.
fn fetch_text(url: &str) -> Result<String, String> {
    // The default provider is Rustls, which this build does not carry: without saying NativeTls,
    // ureq panics on the first https URL.
    let tls = ureq::tls::TlsConfig::builder().provider(ureq::tls::TlsProvider::NativeTls).build();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .user_agent(format!("PZModStudio/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    agent.get(url).call().map_err(|e| e.to_string())?.body_mut().read_to_string().map_err(|e| e.to_string())
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// How long an answer is trusted before GitHub is asked again.
const FRESH_FOR_SECS: i64 = 6 * 60 * 60;

/// GitHub's latest published release (drafts and pre-releases are never "latest").
const LATEST_URL: &str = "https://api.github.com/repos/pzmodstudio/PZModStudio/releases/latest";
/// itch.io's answer for the `windows` channel the releases are pushed to with butler: the version
/// alone, no key needed. Asked when GitHub does not answer (down, suspended, offline for it).
const ITCH_LATEST_URL: &str = "https://itch.io/api/1/x/wharf/latest?target=pichu66/pzmodstudio&channel_name=windows";
const ITCH_PAGE: &str = "https://pichu66.itch.io/pzmodstudio";
/// The setting, the last answer and the version the player said "not now" to.
const STATE_FILE: &str = "PZModStudio_UpdateCheck.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct State {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    checked_at: Option<i64>,
    #[serde(default)]
    latest: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    dismissed: Option<String>,
    /// Who answered last: "github" or "itch".
    #[serde(default)]
    source: Option<String>,
}

/// What the notice and Settings show.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UpdateStatus {
    pub enabled: bool,
    /// This build's version.
    pub current: String,
    /// The latest release's version, without the "v" (None until one answer came).
    pub latest: Option<String>,
    /// The release page, opened by Download.
    pub url: Option<String>,
    /// The latest release is newer than this build.
    pub newer: bool,
    /// The player said "not now" to this very version: no notice until the next one.
    pub dismissed: bool,
    pub checked_at: Option<i64>,
    /// The last attempt failed (offline, GitHub refused): the notice stays quiet.
    pub error: Option<String>,
    /// "Update now" can fetch and verify the release itself (GitHub's signed sums). From itch.io
    /// only the page is offered.
    pub self_update: bool,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
}

#[derive(Deserialize)]
struct Wharf {
    latest: Option<String>,
}

/// itch.io's `{"latest":"1.7.11"}`, as a version; an `errors` answer is None.
fn parse_wharf(text: &str) -> Option<String> {
    serde_json::from_str::<Wharf>(text).ok()?.latest.filter(|v| parse_version(v).is_some())
}

fn load(user_zomboid_dir: &str) -> State {
    cache_path(user_zomboid_dir, STATE_FILE)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(user_zomboid_dir: &str, state: &State) -> Result<(), String> {
    let path = cache_path(user_zomboid_dir, STATE_FILE).ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))?;
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

/// "v1.2.0" or "1.2.0" as numbers; anything after a "-" (a pre-release) is left out.
fn parse_version(v: &str) -> Option<Vec<u64>> {
    let core = v.trim().trim_start_matches(['v', 'V']).split(['-', '+']).next()?;
    let parts: Option<Vec<u64>> = core.split('.').map(|p| p.parse().ok()).collect();
    parts.filter(|p| !p.is_empty())
}

/// Whether `latest` is a later version than `current` (1.10.0 is later than 1.9.0).
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(mut a), Some(mut b)) => {
            let n = a.len().max(b.len());
            a.resize(n, 0);
            b.resize(n, 0);
            a > b
        }
        _ => false,
    }
}

fn status(state: &State, error: Option<String>) -> UpdateStatus {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let newer = state.latest.as_deref().is_some_and(|l| is_newer(l, &current));
    UpdateStatus {
        enabled: state.enabled.unwrap_or(true),
        dismissed: newer && state.dismissed.is_some() && state.dismissed == state.latest,
        current,
        latest: state.latest.clone(),
        url: state.url.clone(),
        newer,
        checked_at: state.checked_at,
        error,
        self_update: state.source.as_deref() != Some("itch"),
    }
}

/// The status, asking GitHub when the check is on and the last answer is six hours old (or `force`,
/// Settings' "Check now"). A failed request keeps the last answer and says why.
pub fn check(user_zomboid_dir: &str, force: bool) -> UpdateStatus {
    let mut state = load(user_zomboid_dir);
    let enabled = state.enabled.unwrap_or(true);
    if !enabled && !force {
        return status(&state, None);
    }
    if !force && state.checked_at.is_some_and(|at| now() - at < FRESH_FOR_SECS) {
        return status(&state, None);
    }
    let error = match fetch_text(LATEST_URL).and_then(|t| serde_json::from_str::<Release>(&t).map_err(|e| e.to_string())) {
        Ok(r) => {
            state.latest = Some(r.tag_name.trim_start_matches(['v', 'V']).to_string());
            state.url = Some(r.html_url);
            state.source = Some("github".to_string());
            None
        }
        // GitHub did not answer: itch.io knows the version too, and its page has the download.
        Err(github) => match fetch_text(ITCH_LATEST_URL).ok().and_then(|t| parse_wharf(&t)) {
            Some(latest) => {
                state.latest = Some(latest);
                state.url = Some(ITCH_PAGE.to_string());
                state.source = Some("itch".to_string());
                None
            }
            None => Some(github),
        },
    };
    // A failure waits the same six hours: no hammering while offline.
    state.checked_at = Some(now());
    let _ = save(user_zomboid_dir, &state);
    status(&state, error)
}

pub fn set_enabled(user_zomboid_dir: &str, on: bool) -> Result<UpdateStatus, String> {
    let mut state = load(user_zomboid_dir);
    state.enabled = Some(on);
    save(user_zomboid_dir, &state)?;
    Ok(status(&state, None))
}

/// "Not now": no notice for this version; the next release shows one again.
pub fn dismiss(user_zomboid_dir: &str, version: &str) -> Result<(), String> {
    let mut state = load(user_zomboid_dir);
    state.dismissed = Some(version.to_string());
    save(user_zomboid_dir, &state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_numbers() {
        assert!(is_newer("v1.3.0", "1.2.0"));
        assert!(is_newer("1.10.0", "1.9.0"));
        assert!(is_newer("2.0", "1.9.9"));
        assert!(!is_newer("v1.2.0", "1.2.0"));
        assert!(!is_newer("1.1.9", "1.2.0"));
        assert!(!is_newer("1.2.0-beta", "1.2.0"));
        assert!(!is_newer("nightly", "1.2.0"));
    }

    #[test]
    fn itchs_answer_is_a_version_or_nothing() {
        assert_eq!(parse_wharf("{\"latest\":\"1.7.11\"}"), Some("1.7.11".to_string()));
        assert_eq!(parse_wharf("{\"errors\":[\"invalid channel\"]}"), None);
        assert_eq!(parse_wharf("{\"latest\":\"head\"}"), None);
        let s = State { latest: Some("9.9.9".into()), source: Some("itch".into()), ..Default::default() };
        assert!(status(&s, None).newer && !status(&s, None).self_update);
    }

    #[test]
    fn dismissed_only_hides_that_version() {
        let mut s = State { latest: Some("99.0.0".into()), dismissed: Some("99.0.0".into()), ..Default::default() };
        assert!(status(&s, None).dismissed);
        s.latest = Some("99.1.0".into());
        assert!(!status(&s, None).dismissed);
        assert!(status(&s, None).newer);
    }

    /// GitHub's answer, read the way `check` reads it. cargo test --lib latest_release_reads -- --ignored --nocapture
    #[test]
    #[ignore]
    fn latest_release_reads() {
        let r: Release = serde_json::from_str(&fetch_text(LATEST_URL).unwrap()).unwrap();
        println!("latest {} at {}; newer than this build: {}", r.tag_name, r.html_url, is_newer(&r.tag_name, env!("CARGO_PKG_VERSION")));
        assert!(parse_version(&r.tag_name).is_some());
    }
}
