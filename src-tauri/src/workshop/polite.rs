//! Every request to Steam goes through here, so the Studio is one polite client however many
//! screens ask at once.
//!
//! - Each Steam host (the Web API, the Community site) has a minimum spacing between requests,
//!   shared by every thread.
//! - When a host answers 429 or 503 it is left alone for the time its `Retry-After` says, or
//!   for 10, 20, 40... seconds up to five minutes. A request that would wait ten seconds or less
//!   waits; a longer one is refused at once with how long it is, instead of piling on. The first
//!   answer that goes through puts the host back to normal.
//! - The same request asked twice at the same time is sent once; the second asker gets the first
//!   one's answer.
//! - Answers are kept on disk (`<Zomboid>/PZModStudio_Cache/steam/`) for as long as each kind
//!   of request stays true, so a restart does not ask Steam everything again.
//!
//! Seen on 2026-10-20: Steam's Workshop was refusing queries for hours, and after a few dozen
//! quick page reads it answered every request from this PC with 429.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Why a request did not get its answer.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    /// Steam asked to slow down; it can be asked again in this many seconds.
    Busy(u64),
    /// Steam answered with this status (not 2xx, not a slow-down).
    Status(u16),
    /// No connection, a timeout, or an answer that could not be read.
    Network(String),
}

impl Refusal {
    /// In plain words, for the window.
    pub fn say(&self) -> String {
        match self {
            Refusal::Busy(secs) => format!("Steam is limiting requests; try again in {} (429).", plain_wait(*secs)),
            Refusal::Status(code) => format!("Steam answered with an error ({}).", code),
            Refusal::Network(e) => format!("Steam could not be reached: {}", e),
        }
    }
}

fn plain_wait(secs: u64) -> String {
    if secs >= 90 { format!("{} minutes", secs.div_ceil(60)) } else { format!("{} seconds", secs.max(1)) }
}

/// The two Steam hosts the Studio talks to, each with its own pace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    /// api.steampowered.com: the keyless Web API.
    Api,
    /// steamcommunity.com: Workshop pages.
    Community,
}

impl Lane {
    pub fn of(url: &str) -> Lane {
        if url.contains("steamcommunity.com") { Lane::Community } else { Lane::Api }
    }
    fn spacing(self) -> Duration {
        match self {
            Lane::Api => Duration::from_millis(250),
            Lane::Community => Duration::from_millis(700),
        }
    }
}

/// How long a refused host is left alone, and how much of a wait a request sits through.
const BACKOFF_FIRST: u64 = 10;
const BACKOFF_MAX: u64 = 300;
const WAIT_AT_MOST: u64 = 10;

#[derive(Debug, Default, Clone)]
struct LaneState {
    last_sent: Option<Instant>,
    cool_until: Option<Instant>,
    /// The next backoff when Steam gives no Retry-After (doubles each refusal).
    next_backoff: u64,
}

/// The pace of the lanes. Separate from the HTTP calls so the rules can be tested on their own.
#[derive(Debug, Default)]
pub struct Pace {
    lanes: HashMap<Lane, LaneState>,
}

impl Pace {
    /// How long to wait before sending on `lane` at `now`, or a refusal when it is cooling for
    /// longer than a request should wait.
    pub fn before_send(&mut self, lane: Lane, now: Instant) -> Result<Duration, Refusal> {
        let state = self.lanes.entry(lane).or_default();
        let mut wait = Duration::ZERO;
        if let Some(until) = state.cool_until {
            if until > now {
                let left = until - now;
                if left.as_secs() > WAIT_AT_MOST {
                    return Err(Refusal::Busy(left.as_secs()));
                }
                wait = left;
            }
        }
        if let Some(last) = state.last_sent {
            let next = last + lane.spacing();
            if next > now + wait {
                wait = next - now;
            }
        }
        state.last_sent = Some(now + wait);
        Ok(wait)
    }

    /// Steam asked to slow down: leave the lane alone for `retry_after`, or the next backoff.
    pub fn refused(&mut self, lane: Lane, now: Instant, retry_after: Option<u64>) -> u64 {
        let state = self.lanes.entry(lane).or_default();
        let backoff = if state.next_backoff == 0 { BACKOFF_FIRST } else { state.next_backoff };
        let secs = retry_after.unwrap_or(backoff).clamp(1, BACKOFF_MAX);
        state.next_backoff = (backoff * 2).min(BACKOFF_MAX);
        state.cool_until = Some(now + Duration::from_secs(secs));
        secs
    }

    /// An answer went through: the lane is back to normal.
    pub fn answered(&mut self, lane: Lane) {
        let state = self.lanes.entry(lane).or_default();
        state.next_backoff = 0;
        state.cool_until = None;
    }
}

impl Pace {
    /// Each lane cooling now, and for how long, in words.
    pub fn describe(&self, now: Instant) -> String {
        let mut parts: Vec<String> = self
            .lanes
            .iter()
            .filter_map(|(lane, s)| s.cool_until.filter(|u| *u > now).map(|u| format!("{:?} limiting us for {} s more", lane, (u - now).as_secs())))
            .collect();
        parts.sort();
        if parts.is_empty() { "answering normally".to_string() } else { parts.join("; ") }
    }
}

/// Whether Steam is limiting the Studio right now, for the diagnostics.
pub fn state() -> String {
    pace().lock().unwrap_or_else(|e| e.into_inner()).describe(Instant::now())
}

fn pace() -> &'static Mutex<Pace> {
    static PACE: OnceLock<Mutex<Pace>> = OnceLock::new();
    PACE.get_or_init(|| Mutex::new(Pace::default()))
}

/// `Retry-After` in seconds (Steam sends seconds; a date is not worth parsing here).
pub fn parse_retry_after(value: Option<&str>) -> Option<u64> {
    value.and_then(|v| v.trim().parse::<u64>().ok())
}

// ------------------------------------------------------------------ one at a time per request

type Answer = Result<String, Refusal>;

#[derive(Default)]
struct Flight {
    done: Mutex<Option<Answer>>,
    ready: Condvar,
}

fn flights() -> &'static Mutex<HashMap<String, Arc<Flight>>> {
    static FLIGHTS: OnceLock<Mutex<HashMap<String, Arc<Flight>>>> = OnceLock::new();
    FLIGHTS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Runs `send` once for `key` however many threads ask at the same time; the others wait for
/// that answer.
pub fn single_flight(key: &str, send: impl FnOnce() -> Answer) -> Answer {
    let (flight, leader) = {
        let mut map = flights().lock().unwrap_or_else(|e| e.into_inner());
        match map.get(key) {
            Some(f) => (f.clone(), false),
            None => {
                let f = Arc::new(Flight::default());
                map.insert(key.to_string(), f.clone());
                (f, true)
            }
        }
    };
    if !leader {
        let mut done = flight.done.lock().unwrap_or_else(|e| e.into_inner());
        while done.is_none() {
            done = flight.ready.wait(done).unwrap_or_else(|e| e.into_inner());
        }
        return done.clone().unwrap();
    }
    let answer = send();
    *flight.done.lock().unwrap_or_else(|e| e.into_inner()) = Some(answer.clone());
    flight.ready.notify_all();
    flights().lock().unwrap_or_else(|e| e.into_inner()).remove(key);
    answer
}

// ------------------------------------------------------------------ the disk cache

/// Where answers are kept; set by tests, else `<Zomboid>/PZModStudio_Cache/steam`.
static CACHE_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
const CACHE_FILES_KEPT: usize = 3000;

fn cache_dir() -> Option<PathBuf> {
    if let Some(dir) = CACHE_DIR.lock().ok().and_then(|g| g.clone()) {
        return Some(dir);
    }
    // The Zomboid folder the Studio saved, or the one it detects.
    let chosen = crate::vfs::studio_paths().user_zomboid_dir;
    let z = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&chosen).into_iter().next()?;
    Some(z.join("PZModStudio_Cache").join("steam"))
}

#[cfg(test)]
pub fn set_cache_dir(dir: Option<PathBuf>) {
    *CACHE_DIR.lock().unwrap() = dir;
}

/// FNV-1a: a stable file name for a request (not for security).
fn key_hash(key: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", h)
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Cached {
    key: String,
    at: u64,
    body: String,
}

fn cache_read(key: &str, ttl: Duration) -> Option<String> {
    let dir = cache_dir()?;
    let text = std::fs::read_to_string(dir.join(format!("{}.json", key_hash(key)))).ok()?;
    let cached: Cached = serde_json::from_str(&text).ok()?;
    (cached.key == key && now_secs().saturating_sub(cached.at) <= ttl.as_secs()).then_some(cached.body)
}

fn cache_write(key: &str, body: &str) {
    let Some(dir) = cache_dir() else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let Ok(text) = serde_json::to_string(&Cached { key: key.to_string(), at: now_secs(), body: body.to_string() }) else { return };
    let _ = crate::load_order::ini_parser::atomic_write(dir.join(format!("{}.json", key_hash(key))), text);
    // Kept small: past the limit, the oldest third goes.
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut files: Vec<(SystemTime, PathBuf)> = entries.flatten().filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path()))).collect();
        if files.len() > CACHE_FILES_KEPT {
            files.sort();
            for (_, p) in files.into_iter().take(CACHE_FILES_KEPT / 3) {
                let _ = std::fs::remove_file(p);
            }
        }
    }
}

// ------------------------------------------------------------------ sending

fn agent() -> ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT
        .get_or_init(|| {
            let tls = ureq::tls::TlsConfig::builder().provider(ureq::tls::TlsProvider::NativeTls).build();
            ureq::Agent::config_builder()
                .tls_config(tls)
                .timeout_global(Some(Duration::from_secs(20)))
                .user_agent(format!("PZModStudio/{}", env!("CARGO_PKG_VERSION")))
                // Every status comes back as an answer, so 429's Retry-After can be read.
                .http_status_as_error(false)
                .build()
                .into()
        })
        .clone()
}

/// One request to Steam.
pub enum Request<'a> {
    Get { url: &'a str, query: &'a [(String, String)] },
    PostForm { url: &'a str, form: &'a [(String, String)] },
}

impl Request<'_> {
    fn url(&self) -> &str {
        match self {
            Request::Get { url, .. } | Request::PostForm { url, .. } => url,
        }
    }
    fn key(&self) -> String {
        let (verb, url, pairs) = match self {
            Request::Get { url, query } => ("GET", url, query),
            Request::PostForm { url, form } => ("POST", url, form),
        };
        let mut pairs: Vec<String> = pairs.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        pairs.sort();
        format!("{} {}?{}", verb, url, pairs.join("&"))
    }
    fn send(&self) -> Result<ureq::http::Response<ureq::Body>, ureq::Error> {
        match self {
            Request::Get { url, query } => {
                let mut req = agent().get(*url);
                for (k, v) in query.iter() {
                    req = req.query(k, v);
                }
                req.call()
            }
            Request::PostForm { url, form } => agent().post(*url).send_form(form.iter().map(|(k, v)| (k.as_str(), v.as_str()))),
        }
    }
}

const BODY_LIMIT: u64 = 16 * 1024 * 1024;

/// Sends `request` politely and gives back the body. `ttl` > 0 keeps the answer on disk that
/// long (and answers from there). `fallback`, for a page, is asked once when Steam refuses this
/// client: Windows' curl was answered when this one was not.
pub fn fetch(request: Request, ttl: Duration, fallback: Option<&dyn Fn() -> Option<String>>) -> Result<String, Refusal> {
    let key = request.key();
    if !ttl.is_zero() {
        if let Some(body) = cache_read(&key, ttl) {
            return Ok(body);
        }
    }
    let lane = Lane::of(request.url());
    single_flight(&key, || {
        let mut attempt = 0u64;
        loop {
            let wait = pace().lock().unwrap_or_else(|e| e.into_inner()).before_send(lane, Instant::now())?;
            if !wait.is_zero() {
                std::thread::sleep(wait);
            }
            let refused_by = |retry_after: Option<u64>| {
                let secs = pace().lock().unwrap_or_else(|e| e.into_inner()).refused(lane, Instant::now(), retry_after);
                Refusal::Busy(secs)
            };
            match request.send() {
                Ok(response) => {
                    let status = response.status().as_u16();
                    if status == 429 || status == 503 {
                        let retry_after = parse_retry_after(response.headers().get("retry-after").and_then(|v| v.to_str().ok()));
                        if let Some(body) = fallback.and_then(|f| f()) {
                            pace().lock().unwrap_or_else(|e| e.into_inner()).answered(lane);
                            if !ttl.is_zero() {
                                cache_write(&key, &body);
                            }
                            return Ok(body);
                        }
                        return Err(refused_by(retry_after));
                    }
                    if status >= 500 && attempt < 2 {
                        attempt += 1;
                        std::thread::sleep(Duration::from_millis(400 * attempt));
                        continue;
                    }
                    if !(200..300).contains(&status) {
                        return Err(Refusal::Status(status));
                    }
                    let body = response
                        .into_body()
                        .with_config()
                        .limit(BODY_LIMIT)
                        .read_to_string()
                        .map_err(|e| Refusal::Network(e.to_string()))?;
                    pace().lock().unwrap_or_else(|e| e.into_inner()).answered(lane);
                    if !ttl.is_zero() {
                        cache_write(&key, &body);
                    }
                    return Ok(body);
                }
                // A pooled connection Steam closed while idle, or a passing drop: two more tries.
                Err(e) if attempt < 2 && matches!(e, ureq::Error::Io(_) | ureq::Error::Timeout(_) | ureq::Error::ConnectionFailed) => {
                    attempt += 1;
                    std::thread::sleep(Duration::from_millis(400 * attempt));
                }
                Err(e) => return Err(Refusal::Network(e.to_string())),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_host_waits_the_time_it_asked_or_doubles_and_comes_back_on_an_answer() {
        let mut p = Pace::default();
        let t0 = Instant::now();
        assert_eq!(p.before_send(Lane::Api, t0).unwrap(), Duration::ZERO);
        // The next one on the same lane waits its spacing; the other lane does not.
        assert_eq!(p.before_send(Lane::Api, t0).unwrap(), Lane::Api.spacing());
        assert_eq!(p.before_send(Lane::Community, t0).unwrap(), Duration::ZERO);

        // Retry-After: 4 s is waited out; 60 s is refused at once.
        assert_eq!(p.refused(Lane::Community, t0, Some(4)), 4);
        let w = p.before_send(Lane::Community, t0).unwrap();
        assert!(w >= Duration::from_secs(3) && w <= Duration::from_secs(4));
        assert_eq!(p.refused(Lane::Community, t0, Some(60)), 60);
        assert!(matches!(p.before_send(Lane::Community, t0), Err(Refusal::Busy(s)) if s >= 59));
        // The Web API is not held up by the Community site.
        assert!(p.before_send(Lane::Api, t0 + Duration::from_secs(1)).is_ok());

        // No Retry-After: 10, 20, 40... up to five minutes; an answer resets it.
        let mut q = Pace::default();
        assert_eq!(q.refused(Lane::Api, t0, None), 10);
        assert_eq!(q.refused(Lane::Api, t0, None), 20);
        assert_eq!(q.refused(Lane::Api, t0, None), 40);
        for _ in 0..10 {
            q.refused(Lane::Api, t0, None);
        }
        assert_eq!(q.refused(Lane::Api, t0, None), BACKOFF_MAX);
        q.answered(Lane::Api);
        assert_eq!(q.before_send(Lane::Api, t0 + Duration::from_secs(400)).unwrap(), Duration::ZERO);
        assert_eq!(q.refused(Lane::Api, t0, None), 10);
    }

    #[test]
    fn retry_after_is_read_in_seconds() {
        assert_eq!(parse_retry_after(Some(" 120 ")), Some(120));
        assert_eq!(parse_retry_after(Some("Wed, 21 Oct 2026 07:28:00 GMT")), None);
        assert_eq!(parse_retry_after(None), None);
        assert_eq!(Refusal::Busy(150).say(), "Steam is limiting requests; try again in 3 minutes (429).");
    }

    #[test]
    fn the_same_request_at_the_same_time_is_sent_once() {
        let sent = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let threads: Vec<_> = (0..6)
            .map(|_| {
                let sent = sent.clone();
                std::thread::spawn(move || {
                    single_flight("same-request", || {
                        sent.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(300));
                        Ok("answer".to_string())
                    })
                })
            })
            .collect();
        let answers: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert!(answers.iter().all(|a| a.as_deref() == Ok("answer")));
        assert_eq!(sent.load(std::sync::atomic::Ordering::SeqCst), 1);
        // Once answered, a later ask is a new request.
        single_flight("same-request", || {
            sent.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(String::new())
        })
        .unwrap();
        assert_eq!(sent.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn answers_are_kept_on_disk_for_their_time() {
        let dir = std::env::temp_dir().join(format!("pzms-steamcache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        set_cache_dir(Some(dir.clone()));
        let key = Request::Get { url: "https://steamcommunity.com/x", query: &[("b".into(), "2".into()), ("a".into(), "1".into())] }.key();
        assert_eq!(key, Request::Get { url: "https://steamcommunity.com/x", query: &[("a".into(), "1".into()), ("b".into(), "2".into())] }.key(), "order does not matter");
        assert_eq!(cache_read(&key, Duration::from_secs(60)), None);
        cache_write(&key, "<html>page</html>");
        assert_eq!(cache_read(&key, Duration::from_secs(60)).as_deref(), Some("<html>page</html>"));
        assert_eq!(cache_read("another", Duration::from_secs(60)), None);
        // Out of date: written "long ago".
        let file = dir.join(format!("{}.json", key_hash(&key)));
        let old = serde_json::to_string(&Cached { key: key.clone(), at: now_secs() - 120, body: "old".into() }).unwrap();
        std::fs::write(&file, old).unwrap();
        assert_eq!(cache_read(&key, Duration::from_secs(60)), None);
        set_cache_dir(None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
