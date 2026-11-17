//! Workshop items a dedicated server with Steam cannot download, left out before it starts.
//!
//! A server logs into Steam anonymously and downloads every id of `WorkshopItems=`. An item that
//! is no longer public (removed, made private or hidden) is refused (`onItemNotDownloaded
//! itemID=<id> result=15`, access denied) and the game does not survive it: `GameServerWorkshopItems
//! .Install` hits a NullPointerException in `ZomboidFileSystem.deleteDirectory` and the server exits
//! before it starts (seen on 2026-11-16, 42.22, with a Viewpoint "Nightly" item the player's own
//! Steam still had). The player's game still has the files, so the mod can run from a copy in
//! `Zomboid/mods`, which a server reads too: the item leaves `WorkshopItems=`, the mod stays in
//! `Mods=`.
//!
//! An item counts as unreachable when Steam's public API does not find it (or it is banned), or
//! when the last server log says the server was refused it. Without an answer from Steam only the
//! log counts: nothing is left out on a guess.

use serde::Serialize;
use std::path::Path;

use super::java_mods::{self, CopyNeed, WorkshopCopy};
use super::mod_sync::ini_list;
use crate::catalog::{HealthIssue, ModCatalog, ModSource, Severity};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UnreachableItem {
    pub item: String,
    /// "not_public": Steam's API has no public item with this id (removed, private or hidden);
    /// "banned"; "refused": the last server log says Steam denied it (`result` code).
    pub reason: String,
    /// The names of the mods of `Mods=` that come from it.
    pub mods: Vec<String>,
    /// Their ids, for copying.
    pub mod_ids: Vec<String>,
    /// Every one of them has a folder in `Zomboid/mods` (a copy or a link), which the server reads.
    pub has_copy: bool,
    /// The item is in this PC's Workshop folder, so a copy can be made from it.
    pub installed: bool,
}

/// `itemID -> result` of the items the server was refused, from a server log.
pub fn refused_in_log(log: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in log.lines() {
        let Some(rest) = line.split("onItemNotDownloaded itemID=").nth(1) else { continue };
        let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let result = rest.split("result=").nth(1).map(|r| r.chars().take_while(|c| c.is_ascii_digit()).collect::<String>()).unwrap_or_default();
        if !id.is_empty() && !out.iter().any(|(i, _)| *i == id) {
            out.push((id, result));
        }
    }
    out
}

/// The items of an `.ini`'s `WorkshopItems=` a server with Steam cannot download.
/// `public`: for each id, Some((found, banned)) from Steam's API, None when Steam did not answer.
pub fn find(catalog: &ModCatalog, content: &str, server_log: &str, public: &dyn Fn(&[String]) -> Option<Vec<(String, bool, bool)>>) -> Vec<UnreachableItem> {
    let items = ini_list(content, "WorkshopItems");
    if items.is_empty() {
        return Vec::new();
    }
    let mods_line = ini_list(content, "Mods");
    let refused = refused_in_log(server_log);
    let answers = public(&items).unwrap_or_default();
    let mut out = Vec::new();
    for item in &items {
        let reason = if let Some((_, found, banned)) = answers.iter().find(|(id, _, _)| id == item) {
            if *banned {
                "banned".to_string()
            } else if !found {
                "not_public".to_string()
            } else {
                continue; // public: whatever an older log said, the server can download it now
            }
        } else if let Some((_, result)) = refused.iter().find(|(id, _)| id == item) {
            format!("refused (result {})", result)
        } else {
            continue;
        };
        let from_item: Vec<&crate::catalog::CatalogMod> = catalog
            .mods
            .iter()
            .filter(|m| matches!(&m.source, ModSource::Workshop { workshop_id } if workshop_id == item))
            .filter(|m| mods_line.iter().any(|e| e == &m.id))
            .collect();
        let has_copy = !from_item.is_empty() && from_item.iter().all(|m| catalog.mods.iter().any(|c| c.id == m.id && matches!(c.source, ModSource::Local)));
        let mut mods: Vec<String> = Vec::new();
        let mut mod_ids: Vec<String> = Vec::new();
        for m in &from_item {
            if !mod_ids.contains(&m.id) {
                mod_ids.push(m.id.clone());
                mods.push(m.name.clone());
            }
        }
        out.push(UnreachableItem {
            item: item.clone(),
            reason,
            installed: catalog.mods.iter().any(|m| matches!(&m.source, ModSource::Workshop { workshop_id } if workshop_id == item)),
            mods,
            mod_ids,
            has_copy,
        });
    }
    out
}

/// Steam's public answer for these ids: (id, found, banned), None when Steam could not be asked.
pub fn ask_steam(ids: &[String]) -> Option<Vec<(String, bool, bool)>> {
    crate::workshop::get_details(ids).ok().map(|items| items.into_iter().map(|i| (i.workshop_id, i.found, i.banned)).collect())
}

/// Before a server with Steam starts: copies into `Zomboid/mods` the mods of the unreachable items
/// that have no folder there yet (from this PC's Workshop folder), then takes those items out of
/// `WorkshopItems=` (the file backed up first). Returns what was left out.
pub fn leave_out(ini_path: &Path, zomboid: &Path, catalog: &ModCatalog) -> Result<Vec<UnreachableItem>, String> {
    let content = std::fs::read_to_string(ini_path).map_err(|e| format!("Cannot read {}: {}", ini_path.display(), e))?;
    let log = std::fs::read_to_string(zomboid.join("server-console.txt")).unwrap_or_default();
    let found = find(catalog, &content, &log, &ask_steam);
    if found.is_empty() {
        return Ok(found);
    }
    for u in found.iter().filter(|u| !u.has_copy && u.installed) {
        let copies: Vec<WorkshopCopy> = u
            .mod_ids
            .iter()
            .filter(|id| !catalog.mods.iter().any(|c| &c.id == *id && matches!(c.source, ModSource::Local)))
            .filter_map(|id| {
                let m = catalog.mods.iter().find(|c| &c.id == id && matches!(&c.source, ModSource::Workshop { workshop_id } if *workshop_id == u.item))?;
                let folder = Path::new(&m.root_dir).file_name()?.to_string_lossy().to_string();
                Some(WorkshopCopy { id: m.id.clone(), name: m.name.clone(), source: m.root_dir.clone(), folder, workshop_id: u.item.clone(), need: CopyNeed::Missing, required_by: vec![] })
            })
            .collect();
        if let Err(e) = java_mods::copy_into_mods(zomboid, &copies) {
            crate::diagnostics::error(format!("[Servers] Could not copy {} into Zomboid/mods: {}", u.mods.join(", "), e));
        }
    }
    let keep: Vec<String> = ini_list(&content, "WorkshopItems").into_iter().filter(|i| !found.iter().any(|u| &u.item == i)).collect();
    let text = crate::discord::ini_set(&content, "WorkshopItems", &keep.join(";"));
    crate::load_order::ini_parser::write_file_with_backup(ini_path, &text, &zomboid.join("PZModStudio_Backups"))?;
    Ok(found)
}

/// The Servers screen's rows: an item a server with Steam cannot download stops it at boot.
pub fn issues(found: &[UnreachableItem]) -> Vec<HealthIssue> {
    found
        .iter()
        .map(|u| {
            let names = if u.mods.is_empty() { "-".to_string() } else { u.mods.join(", ") };
            let why = match u.reason.as_str() {
                "not_public" => "Steam has no public item with this id (removed, made private or hidden)".to_string(),
                "banned" => "Steam banned it".to_string(),
                r => format!("the last server log says Steam refused it ({})", r),
            };
            let variant = if u.has_copy { "copy" } else if u.installed { "will_copy" } else { "lost" };
            let then = match variant {
                "copy" => format!("When the Studio starts it with Steam, it leaves the item out of WorkshopItems= and the server runs {} from its copy in Zomboid/mods.", names),
                "will_copy" => format!("When the Studio starts it with Steam, it copies {} from this PC's Workshop folder into Zomboid/mods and leaves the item out of WorkshopItems=.", names),
                _ => "When the Studio starts it with Steam, it leaves the item out of WorkshopItems=; this PC does not have it either, so its mods will be missing.".to_string(),
            };
            HealthIssue {
                params: crate::catalog::issue_params(&[("item", u.item.as_str()), ("mods", names.as_str()), ("variant", variant), ("reason", u.reason.as_str())]),
                details: vec![],
                severity: Severity::Warning,
                state: crate::catalog::issue_state(Severity::Warning, "server_item_unreachable"),
                code: "server_item_unreachable".to_string(),
                mod_id: u.mod_ids.first().cloned(),
                message: format!("WorkshopItems= has {} ({}): {}. A server with Steam cannot download it and stops at boot. {}", u.item, names, why, then),
                fixes: vec![],
                related: vec![],
                own_mod: false,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_items_a_server_was_refused() {
        let log = "LOG  : Network      f:0> Workshop: onItemNotDownloaded itemID=3839959932 result=15\nLOG  : Network      f:0> Workshop: item state DownloadPending -> Fail ID=3839959932\n";
        assert_eq!(refused_in_log(log), vec![("3839959932".to_string(), "15".to_string())]);
    }

    #[test]
    fn steam_answer_wins_and_without_it_only_the_log_counts() {
        let catalog = ModCatalog::default();
        let ini = "Mods=A;B\nWorkshopItems=111;222;333\n";
        let log = "Workshop: onItemNotDownloaded itemID=222 result=15\nWorkshop: onItemNotDownloaded itemID=333 result=15\n";
        // Steam: 111 gone, 222 public again, 333 not answered.
        let steam = |_: &[String]| Some(vec![("111".to_string(), false, false), ("222".to_string(), true, false)]);
        let found: Vec<(String, String)> = find(&catalog, ini, log, &steam).into_iter().map(|u| (u.item, u.reason)).collect();
        assert_eq!(found, vec![("111".to_string(), "not_public".to_string()), ("333".to_string(), "refused (result 15)".to_string())]);
        let offline = |_: &[String]| None;
        let found: Vec<String> = find(&catalog, ini, "", &offline).into_iter().map(|u| u.item).collect();
        assert!(found.is_empty(), "nothing left out on a guess");
    }
}

#[cfg(test)]
mod live {
    /// `PZMS_INI=<a server .ini> cargo test --release --lib unreachable_live -- --ignored --nocapture` (reads only)
    #[test]
    #[ignore]
    fn unreachable_live() {
        let Ok(ini) = std::env::var("PZMS_INI") else { return };
        let paths = crate::vfs::studio_paths();
        let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
        let content = std::fs::read_to_string(ini).unwrap();
        let log = std::fs::read_to_string(std::path::Path::new(&paths.user_zomboid_dir).join("server-console.txt")).unwrap_or_default();
        let found = super::find(&catalog, &content, &log, &super::ask_steam);
        println!("{:#?}", found);
        for i in super::issues(&found) {
            println!("{}", i.message);
        }
    }
}
