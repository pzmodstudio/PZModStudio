//! "Propose" with a GitHub token: the pull request the person otherwise opens by hand. With a
//! personal token in the secrets store (Windows Credential Manager, like the Steam key), the
//! Studio forks the repository once, syncs the fork, puts the file on a branch of it and opens
//! the pull request against main — the GitHub REST API, nothing registered anywhere. Without
//! a token nothing here runs: the buttons open GitHub's page with the JSON filled in instead.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const GITHUB_TOKEN: &str = "github_token";
/// The repository the community files live in.
pub const OWNER: &str = "pzmodstudio";
pub const REPO: &str = "PZModStudio";
const BASE_BRANCH: &str = "main";
const API: &str = "https://api.github.com";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProposeResult {
    pub pr_url: String,
    pub fork: String,
    pub branch: String,
}

/// Standard base64, for the contents API; small enough not to want a crate.
pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().fold(0u32, |acc, b| (acc << 8) | *b as u32) << (8 * (3 - chunk.len()));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// `propose/<file stem>-<seconds>`: one branch per proposal, readable on GitHub.
pub fn branch_name(path: &str, now: i64) -> String {
    let stem = path.rsplit('/').next().unwrap_or(path).trim_end_matches(".json");
    let slug: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    format!("propose/{}-{}", slug.trim_matches('-'), now)
}

/// A rule merged into the rules file as it is on GitHub: under `load_after`, once.
pub fn merge_rule_into(rules_text: &str, rule: &Value) -> Result<String, String> {
    let mut file: Value = serde_json::from_str(rules_text).map_err(|e| format!("community/rules.json on GitHub is not readable: {}", e))?;
    let list = file
        .get_mut("load_after")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "community/rules.json has no load_after list".to_string())?;
    let same = |a: &Value| a.get("mod_id") == rule.get("mod_id") && a.get("after") == rule.get("after");
    if list.iter().any(same) {
        return Err("That rule is already in community/rules.json.".to_string());
    }
    list.push(rule.clone());
    if let Some(updated) = file.get_mut("updated") {
        *updated = Value::String(chrono::Utc::now().format("%Y-%m-%d").to_string());
    }
    Ok(serde_json::to_string_pretty(&file).map_err(|e| e.to_string())? + "\n")
}

struct Client {
    agent: ureq::Agent,
    token: String,
}

impl Client {
    fn new(token: &str) -> Self {
        let tls = ureq::tls::TlsConfig::builder().provider(ureq::tls::TlsProvider::NativeTls).build();
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .tls_config(tls)
            .timeout_global(Some(std::time::Duration::from_secs(20)))
            .user_agent(format!("PZModStudio/{}", env!("CARGO_PKG_VERSION")))
            .http_status_as_error(false)
            .build()
            .into();
        Client { agent, token: token.to_string() }
    }

    /// One call; the status and the JSON body (or Null), with GitHub's message when it refuses.
    fn call(&self, method: &str, path: &str, body: Option<Value>) -> Result<(u16, Value), String> {
        let url = format!("{}{}", API, path);
        let req = |r: ureq::RequestBuilder<ureq::typestate::WithBody>| {
            r.header("Authorization", &format!("Bearer {}", self.token))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
        };
        let mut response = match (method, body) {
            ("GET", _) => self
                .agent
                .get(&url)
                .header("Authorization", &format!("Bearer {}", self.token))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .call(),
            ("POST", b) => req(self.agent.post(&url)).send_json(b.unwrap_or(Value::Null)),
            ("PUT", b) => req(self.agent.put(&url)).send_json(b.unwrap_or(Value::Null)),
            _ => unreachable!(),
        }
        .map_err(|e| format!("GitHub could not be reached: {}", e))?;
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        Ok((status, value))
    }

    fn refuse(status: u16, value: &Value, what: &str) -> String {
        let msg = value.get("message").and_then(Value::as_str).unwrap_or("");
        match status {
            401 => "GitHub refuses the token: it is wrong or expired. Settings > GitHub token.".to_string(),
            403 if msg.contains("rate limit") => "GitHub's rate limit is hit for this token; try again in an hour.".to_string(),
            403 | 404 => format!("The token cannot {}: it needs \"Contents\" read and write (and \"Pull requests\" write) on your fork. GitHub says: {}", what, msg),
            _ => format!("GitHub refused to {} ({}): {}", what, status, msg),
        }
    }

    /// The account the token belongs to.
    fn login(&self) -> Result<String, String> {
        let (status, v) = self.call("GET", "/user", None)?;
        if status != 200 {
            return Err(Self::refuse(status, &v, "read the account"));
        }
        v.get("login").and_then(Value::as_str).map(str::to_string).ok_or_else(|| "GitHub gave no login for the token".to_string())
    }

    /// The fork of the repository on the token's account, made and waited for when there is none;
    /// the account's own repository when the token is the owner's.
    fn fork(&self, login: &str) -> Result<String, String> {
        if login == OWNER {
            return Ok(format!("{}/{}", OWNER, REPO));
        }
        let (status, _) = self.call("GET", &format!("/repos/{}/{}", login, REPO), None)?;
        if status != 200 {
            let (status, v) = self.call("POST", &format!("/repos/{}/{}/forks", OWNER, REPO), Some(json!({ "default_branch_only": true })))?;
            if status != 202 && status != 200 {
                return Err(Self::refuse(status, &v, "fork the repository"));
            }
            for _ in 0..15 {
                std::thread::sleep(std::time::Duration::from_secs(2));
                if self.call("GET", &format!("/repos/{}/{}/git/ref/heads/{}", login, REPO, BASE_BRANCH), None)?.0 == 200 {
                    break;
                }
            }
        } else {
            // A fork from an earlier proposal: bring its main up to date, so the branch starts from today's files.
            let _ = self.call("POST", &format!("/repos/{}/{}/merge-upstream", login, REPO), Some(json!({ "branch": BASE_BRANCH })));
        }
        Ok(format!("{}/{}", login, REPO))
    }

    fn main_sha(&self, repo: &str) -> Result<String, String> {
        let (status, v) = self.call("GET", &format!("/repos/{}/git/ref/heads/{}", repo, BASE_BRANCH), None)?;
        if status != 200 {
            return Err(Self::refuse(status, &v, "read the fork's main branch"));
        }
        v.pointer("/object/sha").and_then(Value::as_str).map(str::to_string).ok_or_else(|| "GitHub gave no commit for main".to_string())
    }

    /// The file as it is on a branch: its text and the blob sha the update needs; None when absent.
    fn file(&self, repo: &str, path: &str, branch: &str) -> Result<Option<(String, String)>, String> {
        let (status, v) = self.call("GET", &format!("/repos/{}/contents/{}?ref={}", repo, encode_path(path), branch), None)?;
        match status {
            404 => Ok(None),
            200 => {
                let sha = v.get("sha").and_then(Value::as_str).unwrap_or("").to_string();
                let content = v.get("content").and_then(Value::as_str).unwrap_or("").replace('\n', "");
                let bytes = base64_decode(&content).ok_or_else(|| "GitHub's copy of the file is not readable".to_string())?;
                Ok(Some((String::from_utf8_lossy(&bytes).to_string(), sha)))
            }
            _ => Err(Self::refuse(status, &v, "read the file")),
        }
    }
}

fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let bytes: Vec<u8> = text.bytes().filter(|b| *b != b'=' && !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut n = 0u32;
        for (i, b) in chunk.iter().enumerate() {
            n |= (val(*b)? as u32) << (18 - 6 * i);
        }
        for i in 0..chunk.len() - 1 {
            out.push(((n >> (16 - 8 * i)) & 255) as u8);
        }
    }
    Some(out)
}

/// Checks a token with GitHub and keeps it; returns the login.
pub fn save_token(token: &str) -> Result<String, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("Paste a token first.".to_string());
    }
    let login = Client::new(token).login()?;
    crate::secrets::set(GITHUB_TOKEN, token)?;
    Ok(login)
}

fn token() -> Result<String, String> {
    crate::secrets::get(GITHUB_TOKEN)?.ok_or_else(|| "No GitHub token is saved (Settings > GitHub token).".to_string())
}

/// Puts one file on a branch of the fork and opens the pull request. `new_content` gets the
/// file as it is on GitHub today (None when it does not exist) and returns what it should be.
pub fn propose_file(path: &str, title: &str, body: &str, new_content: &dyn Fn(Option<&str>) -> Result<String, String>) -> Result<ProposeResult, String> {
    let client = Client::new(&token()?);
    let login = client.login()?;
    let fork = client.fork(&login)?;
    let sha = client.main_sha(&fork)?;
    let branch = branch_name(path, chrono::Utc::now().timestamp());
    let (status, v) = client.call("POST", &format!("/repos/{}/git/refs", fork), Some(json!({ "ref": format!("refs/heads/{}", branch), "sha": sha })))?;
    if status != 201 {
        return Err(Client::refuse(status, &v, "make a branch on the fork"));
    }
    let existing = client.file(&fork, path, &branch)?;
    let content = new_content(existing.as_ref().map(|(text, _)| text.as_str()))?;
    let mut put = json!({ "message": title, "content": base64(content.as_bytes()), "branch": branch });
    if let Some((_, file_sha)) = &existing {
        put["sha"] = Value::String(file_sha.clone());
    }
    let (status, v) = client.call("PUT", &format!("/repos/{}/contents/{}", fork, encode_path(path)), Some(put))?;
    if status != 200 && status != 201 {
        return Err(Client::refuse(status, &v, "write the file on the fork"));
    }
    let head = if login == OWNER { branch.clone() } else { format!("{}:{}", login, branch) };
    let (status, v) = client.call("POST", &format!("/repos/{}/{}/pulls", OWNER, REPO), Some(json!({ "title": title, "head": head, "base": BASE_BRANCH, "body": body, "maintainer_can_modify": true })))?;
    if status != 201 {
        return Err(Client::refuse(status, &v, "open the pull request"));
    }
    let pr_url = v.get("html_url").and_then(Value::as_str).unwrap_or("").to_string();
    Ok(ProposeResult { pr_url, fork, branch })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips_and_pads() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64("{\"id\": \"ñ\"}\n".as_bytes()), "eyJpZCI6ICLDsSJ9Cg==");
        assert_eq!(base64_decode("eyJpZCI6ICLDsSJ9Cg==").unwrap(), "{\"id\": \"ñ\"}\n".as_bytes());
        // GitHub wraps the content in 60-character lines; padding only ever ends it.
        assert_eq!(base64_decode("Zm9v\nZm8=").unwrap(), b"foofo");
    }

    #[test]
    fn a_branch_is_named_after_the_file() {
        assert_eq!(branch_name("community/index/2858201402__dailystatisticsandgains.json", 7), "propose/2858201402--dailystatisticsandgains-7");
        assert_eq!(branch_name("community/rules.json", 7), "propose/rules-7");
    }

    #[test]
    fn a_rule_is_merged_once_into_the_rules_file() {
        let rules = r#"{"version":1,"updated":"2026-10-18","load_after":[],"incompatible":[]}"#;
        let rule = json!({ "mod_id": "A", "after": "B", "note": "n", "source": "tested on 42.21" });
        let merged = merge_rule_into(rules, &rule).unwrap();
        assert!(merged.contains("\"mod_id\": \"A\""));
        assert!(merge_rule_into(&merged, &rule).is_err(), "the same rule twice is refused");
        assert!(merge_rule_into("{}", &rule).is_err());
    }
}

/// A repository path for the contents API: each segment percent-encoded (mod ids carry spaces,
/// brackets and parentheses), the slashes between them kept.
fn encode_path(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            seg.bytes()
                .map(|b| match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
                    _ => format!("%{:02X}", b),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod encode_tests {
    #[test]
    fn a_mod_id_with_spaces_and_brackets_is_a_valid_path() {
        assert_eq!(
            super::encode_path("community/index/Arsenal(26)GunFighter[MAIN MOD 2.0].json"),
            "community/index/Arsenal%2826%29GunFighter%5BMAIN%20MOD%202.0%5D.json"
        );
    }
}
