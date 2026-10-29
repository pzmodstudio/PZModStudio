//! Installing a mod from a link: a direct `.zip`, or a GitHub repository or release.
//!
//! Most Project Zomboid mods live on the Workshop, where subscribing is one click. The rest are
//! posted as a zip on a forum, a Discord, itch.io or GitHub, and until now getting one in meant
//! downloading it in a browser, finding it, and picking the file by hand.
//!
//! What arrives from a link is untrusted, so it is treated exactly like a file the user picked:
//! downloaded into its own temporary folder, capped in size, and unpacked by the installer that
//! already refuses entries pointing outside the mod folder. Only http and https are followed,
//! and only a zip is unpacked. Where the link came from is recorded with the mod, so months later
//! the Mod List can still say where a hand-installed mod is from.

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;

use crate::origins::ModOrigin;

/// 512 MB: larger than any Project Zomboid mod, and small enough that a wrong link cannot fill
/// a disk before anyone notices.
const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;

/// What a link turns out to offer, worked out before anything is downloaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlInstallPlan {
    /// The file that would be downloaded.
    pub download_url: String,
    pub file_name: String,
    /// Size in bytes when the source says it, which GitHub does for a release asset.
    pub size: Option<u64>,
    /// In words: "the latest release of owner/repo (v1.2.0)", "a direct link".
    pub source: String,
    /// A page a person can open to see what this is.
    pub page_url: Option<String>,
}

/// The same native-TLS agent the Workshop calls use, with longer to run: a release can be tens
/// of megabytes.
fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::NativeTls)
        .build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        // A large release on a slow line takes minutes: the limit is on a stalled connection and
        // on a whole download that goes on for too long, not on two minutes in total.
        .timeout_connect(Some(std::time::Duration::from_secs(30)))
        .timeout_recv_response(Some(std::time::Duration::from_secs(60)))
        .timeout_recv_body(Some(std::time::Duration::from_secs(30 * 60)))
        .timeout_global(Some(std::time::Duration::from_secs(30 * 60)))
        .user_agent(format!("PZModStudio/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// owner/repo from any github.com link that names one.
fn github_repo(url: &str) -> Option<(String, String)> {
    let rest = url
        .trim()
        .strip_prefix("https://")
        .or_else(|| url.trim().strip_prefix("http://"))?
        .strip_prefix("www.")
        .or_else(|| {
            url.trim()
                .strip_prefix("https://")
                .or_else(|| url.trim().strip_prefix("http://"))
        })?;
    let rest = rest.strip_prefix("github.com/")?;
    let mut parts = rest.split('/').filter(|p| !p.is_empty());
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.trim_end_matches(".git").to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

/// Works out what a link gives you. Nothing is downloaded here but, for GitHub, the release the
/// link points at is looked up so the user can see the tag and size before agreeing.
pub fn resolve(url: &str) -> Result<UrlInstallPlan, String> {
    let url = url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Only http and https links can be installed from.".to_string());
    }
    let lower = url.to_lowercase();

    if lower.contains("steamcommunity.com") {
        return Err(
            "That is a Steam Workshop link. Open it in Browse Mods and subscribe: Steam downloads \
             and updates Workshop mods itself."
                .to_string(),
        );
    }
    if lower.contains("nexusmods.com") {
        return Err(
            "Nexus Mods does not allow downloading without their own login, so paste the file you \
             downloaded from them instead (Install from a .zip). The Studio records that it came \
             from Nexus."
                .to_string(),
        );
    }

    // A GitHub link: the latest release's zip, or the repository's own zipball.
    if let Some((owner, repo)) = github_repo(url) {
        if !lower.ends_with(".zip") {
            return github_plan(&owner, &repo);
        }
    }

    let file_name = url
        .rsplit('/')
        .next()
        .and_then(|n| n.split(['?', '#']).next())
        .filter(|n| !n.is_empty())
        .unwrap_or("mod.zip")
        .to_string();
    let ext = file_name.rsplit('.').next().unwrap_or("").to_lowercase();
    if ext == "7z" || ext == "rar" {
        return Err(format!(
            "\".{}\" archives are not supported yet. Download it, extract it (for example with \
             7-Zip) and install the extracted folder.",
            ext
        ));
    }
    if ext != "zip" {
        return Err(
            "That link does not point at a .zip. Paste a direct link to the mod's zip, or a \
             GitHub repository or release page."
                .to_string(),
        );
    }

    Ok(UrlInstallPlan {
        download_url: url.to_string(),
        file_name,
        size: None,
        source: "a direct link".to_string(),
        page_url: None,
    })
}

/// Asks GitHub for a JSON answer, turning its refusals into something a player can act on.
fn github_json(url: &str, owner: &str, repo: &str) -> Result<Option<serde_json::Value>, String> {
    let response = agent()
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .call();
    let body = match response {
        Ok(r) => r
            .into_body()
            .read_to_string()
            .map_err(|e| format!("Unexpected answer from GitHub: {}", e))?,
        // Not found is an answer, not a failure: a repository can simply have no releases.
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(ureq::Error::StatusCode(403)) | Err(ureq::Error::StatusCode(429)) => {
            return Err(
                "GitHub is rate-limiting this machine (it allows 60 requests an hour without an \
                 account). Try again later, or paste a direct link to the .zip."
                    .to_string(),
            )
        }
        Err(other) => {
            return Err(format!("GitHub could not be reached about {}/{}: {}", owner, repo, other))
        }
    };
    serde_json::from_str(&body)
        .map(Some)
        .map_err(|e| format!("Unexpected answer from GitHub: {}", e))
}

/// What to download for a GitHub repository.
///
/// A release with a zip attached is the best thing to take: the author chose what goes in it.
/// Most Project Zomboid mods on GitHub have no releases at all, though — checked against the
/// repositories people actually share — so the normal case is the repository's current files,
/// which GitHub will zip on request. The installer then finds the mod folder inside whatever
/// nesting the zip has.
fn github_plan(owner: &str, repo: &str) -> Result<UrlInstallPlan, String> {
    let latest = github_json(
        &format!("https://api.github.com/repos/{}/{}/releases/latest", owner, repo),
        owner,
        repo,
    )?;

    if let Some(release) = latest {
        let tag = release.get("tag_name").and_then(|v| v.as_str()).unwrap_or("latest").to_string();
        let page_url = release.get("html_url").and_then(|v| v.as_str()).map(str::to_string);
        let asset = release.get("assets").and_then(|a| a.as_array()).and_then(|assets| {
            assets.iter().find(|a| {
                a.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.to_lowercase().ends_with(".zip"))
                    .unwrap_or(false)
            })
        });
        if let Some(asset) = asset {
            if let Some(url) = asset.get("browser_download_url").and_then(|v| v.as_str()) {
                return Ok(UrlInstallPlan {
                    download_url: url.to_string(),
                    file_name: asset
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("mod.zip")
                        .to_string(),
                    size: asset.get("size").and_then(|v| v.as_u64()),
                    source: format!("the latest release of {}/{} ({})", owner, repo, tag),
                    page_url,
                });
            }
        }
    }

    // No release, or one with nothing attached: take the repository as it stands.
    let info = github_json(&format!("https://api.github.com/repos/{}/{}", owner, repo), owner, repo)?
        .ok_or_else(|| {
            format!(
                "GitHub has no repository {}/{}. Check the link, or paste a direct link to a .zip.",
                owner, repo
            )
        })?;
    let branch = info
        .get("default_branch")
        .and_then(|v| v.as_str())
        .unwrap_or("main")
        .to_string();
    Ok(UrlInstallPlan {
        download_url: format!("https://api.github.com/repos/{}/{}/zipball/{}", owner, repo, branch),
        file_name: format!("{}-{}.zip", repo, branch),
        size: None,
        source: format!("the current files of {}/{} (branch {})", owner, repo, branch),
        page_url: info.get("html_url").and_then(|v| v.as_str()).map(str::to_string),
    })
}

/// Downloads the plan's file into its own temporary folder and returns the path.
fn download(plan: &UrlInstallPlan) -> Result<(PathBuf, PathBuf), String> {
    let dir = std::env::temp_dir().join(format!(
        "pzms-url-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create a download folder: {}", e))?;
    // The name is ours, not the server's: a file name from a link is not to be trusted with a path.
    let file = dir.join("download.zip");

    let response = agent()
        .get(&plan.download_url)
        // Anything: GitHub's zipball endpoint refuses application/octet-stream outright (415).
        .header("Accept", "*/*")
        .call()
        .map_err(|e| {
            let _ = std::fs::remove_dir_all(&dir);
            format!("The download failed: {}", e)
        })?;

    let mut reader = response.into_body().into_reader().take(MAX_DOWNLOAD_BYTES + 1);
    let mut bytes = Vec::new();
    if let Err(e) = reader.read_to_end(&mut bytes) {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(format!("The download failed: {}", e));
    }
    if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(format!(
            "That file is larger than {} MB, which is bigger than any Project Zomboid mod. \
             Download it yourself and install the file if you are sure.",
            MAX_DOWNLOAD_BYTES / (1024 * 1024)
        ));
    }
    if !bytes.starts_with(b"PK") {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(
            "What came back is not a zip file. The link may need a login, or point at a page \
             rather than a file."
                .to_string(),
        );
    }
    std::fs::write(&file, &bytes).map_err(|e| {
        let _ = std::fs::remove_dir_all(&dir);
        format!("Cannot save the download: {}", e)
    })?;
    Ok((dir, file))
}

/// Downloads a link and installs what is in it, recording where it came from.
pub fn install_from_url(
    url: &str,
    user_zomboid_dir: &str,
    workshop_dir: &str,
    replace_existing: bool,
) -> Result<crate::mod_install::InstallResult, String> {
    let plan = resolve(url)?;
    let (dir, file) = download(&plan)?;
    let origin = ModOrigin::Url {
        url: plan.download_url.clone(),
        page_url: plan.page_url.clone(),
        source: plan.source.clone(),
    };
    let result = crate::mod_install::install_mods(
        &file.to_string_lossy(),
        user_zomboid_dir,
        workshop_dir,
        replace_existing,
        Some(origin),
    );
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repository_is_read_out_of_any_github_link() {
        assert_eq!(
            github_repo("https://github.com/Owner/Repo"),
            Some(("Owner".into(), "Repo".into()))
        );
        assert_eq!(
            github_repo("https://github.com/Owner/Repo/releases/tag/v1.0"),
            Some(("Owner".into(), "Repo".into()))
        );
        assert_eq!(
            github_repo("https://github.com/Owner/Repo.git"),
            Some(("Owner".into(), "Repo".into()))
        );
        assert_eq!(github_repo("https://example.com/Owner/Repo"), None);
        assert_eq!(github_repo("https://github.com/Owner"), None);
    }

    #[test]
    fn links_that_cannot_work_say_why_before_anything_is_downloaded() {
        let err = resolve("ftp://example.com/mod.zip").unwrap_err();
        assert!(err.contains("http"), "{}", err);

        let err = resolve("https://steamcommunity.com/sharedfiles/filedetails/?id=123").unwrap_err();
        assert!(err.contains("subscribe"), "{}", err);

        let err = resolve("https://www.nexusmods.com/projectzomboid/mods/445").unwrap_err();
        assert!(err.contains("Nexus"), "{}", err);

        let err = resolve("https://example.com/mods/pack.7z").unwrap_err();
        assert!(err.contains("7z"), "{}", err);

        let err = resolve("https://example.com/mods/readme.pdf").unwrap_err();
        assert!(err.contains(".zip"), "{}", err);
    }

    #[test]
    fn a_direct_zip_link_is_taken_as_it_is() {
        let plan = resolve("https://example.com/files/MyMod_v2.zip?token=abc").unwrap();
        assert_eq!(plan.file_name, "MyMod_v2.zip");
        assert_eq!(plan.download_url, "https://example.com/files/MyMod_v2.zip?token=abc");
        assert_eq!(plan.source, "a direct link");
    }

    /// Asks GitHub about a real repository that ships Project Zomboid mod zips.
    /// PZMS_GITHUB_REPO=owner/repo cargo test a_real_github_release -- --ignored --nocapture
    #[test]
    #[ignore]
    fn a_real_github_release() {
        let repo = std::env::var("PZMS_GITHUB_REPO").unwrap_or_else(|_| "BurntBreakfast/cheatMenu".into());
        let plan = resolve(&format!("https://github.com/{}", repo));
        match plan {
            Ok(p) => println!(
                "{}\n  file: {} ({:?} bytes)\n  from: {}\n  page: {:?}",
                p.source, p.file_name, p.size, p.download_url, p.page_url
            ),
            Err(e) => println!("{}: {}", repo, e),
        }
    }
}

#[cfg(test)]
mod live_install_tests {
    /// Downloads a real link into a throwaway Zomboid folder and installs what is in it.
    /// PZMS_INSTALL_URL=https://github.com/owner/repo
    /// cargo test installs_from_a_real_link -- --ignored --nocapture
    #[test]
    #[ignore]
    fn installs_from_a_real_link() {
        let Ok(url) = std::env::var("PZMS_INSTALL_URL") else { return };
        let zomboid = std::env::temp_dir().join(format!("pzms-urlinstall-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&zomboid);
        std::fs::create_dir_all(zomboid.join("mods")).unwrap();

        let plan = super::resolve(&url).expect("the link resolves");
        println!("{} -> {} ({})", url, plan.file_name, plan.source);

        match super::install_from_url(&url, &zomboid.to_string_lossy(), "", false) {
            Ok(result) => {
                println!("{} mod folder(s) installed:", result.installed.len());
                for m in &result.installed {
                    println!("  {:?} in {}", m.mod_id, m.folder);
                }
                for w in &result.warnings {
                    println!("  warning: {}", w);
                }
                // What was recorded about where it came from.
                let origins = crate::origins::load(&zomboid);
                for (folder, record) in origins {
                    println!("  origin of {}: {:?}", folder, record.origin);
                }
            }
            Err(e) => println!("install refused: {}", e),
        }
        let _ = std::fs::remove_dir_all(&zomboid);
    }
}
