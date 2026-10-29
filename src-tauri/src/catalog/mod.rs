//! The mod catalog: a read-only picture of every installed mod as the game sees it, and a
//! health report of what will go wrong with an active mod list. Nothing here writes to disk.
//!
//! Game rules reproduced (decompiled 42.20.4):
//! - mod folders are searched Workshop uploads (`Zomboid/Workshop/*/Contents/mods`), then
//!   subscribed Workshop items, then `Zomboid/mods` (or `Zomboid/mods` first when the Studio
//!   launches with `-modfolders mods,workshop,steam` for compatible copies); the first copy of an id wins;
//! - each mod's mod.info comes from its version folder for the installed build, else `common/`
//!   (`select_mod_info_for_build`); Build 41 reads the root;
//! - a mod is unavailable outside versionMin..versionMax or when a required mod (at any depth)
//!   is missing or unavailable.

pub mod modinfo;

use crate::load_order::mod_info::{
    get_existing_user_zomboid_dirs, get_subscribed_workshop_ids, read_installed_game_version,
    select_mod_info_for_build, version_dir_int,
};
use modinfo::{format_game_version, parse_game_mod_info, GameModInfo};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModSource {
    /// A Workshop item the user uploads (Zomboid/Workshop/<item>/Contents/mods).
    WorkshopUpload { item_folder: String },
    /// A subscribed Steam Workshop item.
    Workshop { workshop_id: String },
    /// Zomboid/mods.
    Local,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Loadability {
    Loadable,
    /// No mod.info the installed build reads (B41-only, or only folders for newer builds).
    NoFolderForBuild { available_folders: Vec<String> },
    /// mod.info found, but the build is outside versionMin..versionMax.
    OutsideVersionRange { version_min: Option<String>, version_max: Option<String> },
    /// The game rejects the mod.info (no id, invalid version string).
    InvalidModInfo { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMod {
    pub id: String,
    pub name: String,
    pub source: ModSource,
    /// The mod folder (direct child of a `mods` folder).
    pub root_dir: String,
    /// The mod.info the game reads on the installed build (or the one used for listing).
    pub mod_info_path: Option<String>,
    /// Version folders present (`42`, `42.13`, ...) and whether `common/` / a root mod.info exist.
    pub version_folders: Vec<String>,
    pub has_common: bool,
    pub has_root_mod_info: bool,
    pub loadability: Loadability,
    /// When another copy with the same id is found first, the root of that copy: this one is ignored.
    pub shadowed_by: Option<String>,
    /// For mods in Zomboid/mods installed by the Studio: where they were downloaded from.
    #[serde(default)]
    pub origin: Option<crate::origins::ModOrigin>,
    /// For subscribed Workshop items: what Steam has installed vs the latest version.
    #[serde(default)]
    pub workshop_state: Option<crate::workshop::InstalledItemState>,
    pub info: GameModInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModCatalog {
    /// "42.21" when Zomboid/version.txt was found.
    pub game_version: Option<String>,
    /// In game search order; shadowed duplicates included (marked with `shadowed_by`).
    pub mods: Vec<CatalogMod>,
}

impl ModCatalog {
    /// The copy of `id` the game uses (the first one found), if installed.
    pub fn get(&self, id: &str) -> Option<&CatalogMod> {
        self.mods.iter().find(|m| m.id == id && m.shadowed_by.is_none())
    }

    /// The mod a default.txt entry refers to (`ModId` or `<workshopId>/<ModId>`).
    pub fn resolve(&self, entry: &str) -> Option<&CatalogMod> {
        resolve_entry(self, entry)
    }

    /// The installed build as major*1000+minor, when known.
    pub fn game_version_int(&self) -> Option<u32> {
        self.game_version.as_deref().and_then(crate::load_order::mod_info::version_dir_int)
    }
}

fn child_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect())
        .unwrap_or_default();
    out.sort();
    out
}

/// Every `mods` folder the game searches, in its order, with the source each one represents.
fn mod_folders_in_game_order(workshop_dir: &str, user_zomboid_dir: &str) -> Vec<(PathBuf, ModSource)> {
    let mut folders = Vec::new();
    let user_dir = get_existing_user_zomboid_dirs(user_zomboid_dir).into_iter().next();

    if let Some(user_dir) = &user_dir {
        for item in child_dirs(&user_dir.join("Workshop")) {
            let mods = item.join("Contents").join("mods");
            if mods.is_dir() {
                let item_folder = item.file_name().unwrap_or_default().to_string_lossy().to_string();
                folders.push((mods, ModSource::WorkshopUpload { item_folder }));
            }
        }
    }

    let workshop = Path::new(workshop_dir);
    if !workshop_dir.is_empty() && workshop.is_dir() {
        let subscribed = get_subscribed_workshop_ids(workshop);
        let mut items: Vec<(u64, PathBuf, String)> = child_dirs(workshop)
            .into_iter()
            .filter_map(|p| {
                let id = p.file_name()?.to_string_lossy().to_string();
                Some((id.parse::<u64>().ok()?, p, id))
            })
            .filter(|(_, _, id)| subscribed.as_ref().is_none_or(|s| s.contains(id)))
            .collect();
        items.sort();
        for (_, item, workshop_id) in items {
            let mods = item.join("mods");
            if mods.is_dir() {
                folders.push((mods, ModSource::Workshop { workshop_id }));
            }
        }
    }

    if let Some(user_dir) = &user_dir {
        let local = (user_dir.join("mods"), ModSource::Local);
        // With compatible copies the Studio launches the game with "-modfolders mods,workshop,steam":
        // Zomboid/mods is searched first, so the copies win over the Workshop originals.
        if crate::compat::local_first(user_dir) {
            folders.insert(0, local);
        } else {
            folders.push(local);
        }
    }
    folders
}

fn read_game_mod_info(path: &Path) -> Option<GameModInfo> {
    let bytes = fs::read(path).ok()?;
    Some(parse_game_mod_info(&String::from_utf8_lossy(&bytes)))
}

/// Builds the catalog for the installed game build (read from `Zomboid/version.txt` unless
/// `game_version_override` is given, e.g. 42_015).
pub fn build_catalog(workshop_dir: &str, user_zomboid_dir: &str, game_version_override: Option<u32>) -> ModCatalog {
    let game_version = game_version_override.or_else(|| read_installed_game_version(user_zomboid_dir));
    let mut mods: Vec<CatalogMod> = Vec::new();
    let mut first_copy: HashMap<String, String> = HashMap::new();
    let steam_state = crate::workshop::local_state(workshop_dir).unwrap_or_default();
    let recorded_origins = get_existing_user_zomboid_dirs(user_zomboid_dir)
        .first()
        .map(|dir| crate::origins::load(dir))
        .unwrap_or_default();

    for (mods_dir, source) in mod_folders_in_game_order(workshop_dir, user_zomboid_dir) {
        for root in child_dirs(&mods_dir) {
            let folder_name = root.file_name().unwrap_or_default().to_string_lossy().to_string();
            if folder_name.eq_ignore_ascii_case("examplemod") {
                continue; // the game refuses to list it
            }
            let mut version_folders: Vec<String> = child_dirs(&root)
                .iter()
                .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .filter(|n| version_dir_int(n).is_some())
                .collect();
            version_folders.sort_by_key(|n| version_dir_int(n));
            let has_common = root.join("common").is_dir();
            let has_root_mod_info = root.join("mod.info").is_file();

            let selected = select_mod_info_for_build(&root, game_version);
            // Not loadable on this build: still list it from its newest mod.info, else the root one.
            let listed = selected.clone().or_else(|| select_mod_info_for_build(&root, None)).or_else(|| {
                let r = root.join("mod.info");
                r.is_file().then_some(r)
            });
            let Some(mod_info_path) = listed else {
                continue; // not a mod folder
            };
            let Some(info) = read_game_mod_info(&mod_info_path) else { continue };

            let loadability = if let Some(reason) = &info.invalid_reason {
                Loadability::InvalidModInfo { reason: reason.clone() }
            } else if info.id.is_none() {
                Loadability::InvalidModInfo {
                    reason: "mod.info has no line starting with \"id=\" (check spelling, case and a UTF-8 BOM)".into(),
                }
            } else if selected.is_none() {
                let mut available = version_folders.clone();
                if has_common {
                    available.push("common".into());
                }
                if has_root_mod_info {
                    available.push("(root, Build 41 layout)".into());
                }
                Loadability::NoFolderForBuild { available_folders: available }
            } else if game_version.is_some_and(|v| !info.fits_version(v)) {
                Loadability::OutsideVersionRange {
                    version_min: info.version_min.map(format_game_version),
                    version_max: info.version_max.map(format_game_version),
                }
            } else {
                Loadability::Loadable
            };

            let id = info.id.clone().unwrap_or_else(|| folder_name.clone());
            let root_str = root.to_string_lossy().to_string();
            let shadowed_by = match first_copy.get(&id) {
                Some(first) => Some(first.clone()),
                None => {
                    first_copy.insert(id.clone(), root_str.clone());
                    None
                }
            };
            mods.push(CatalogMod {
                name: info.name.clone().map(|n| n.trim().to_string()).unwrap_or_else(|| id.clone()),
                id,
                source: source.clone(),
                root_dir: root_str,
                mod_info_path: Some(mod_info_path.to_string_lossy().to_string()),
                version_folders,
                has_common,
                has_root_mod_info,
                loadability,
                shadowed_by,
                origin: if source == ModSource::Local { recorded_origins.get(&folder_name).map(|r| r.origin.clone()) } else { None },
                workshop_state: match &source {
                    ModSource::Workshop { workshop_id } => steam_state.items.get(workshop_id).cloned(),
                    _ => None,
                },
                info,
            });
        }
    }

    ModCatalog { game_version: game_version.map(format_game_version), mods }
}

/// The catalog and the health report of an active list, as returned to the UI and MCP clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModHealth {
    pub catalog: ModCatalog,
    pub report: HealthReport,
}

pub fn mod_health(workshop_dir: &str, user_zomboid_dir: &str, active: &[String]) -> ModHealth {
    let catalog = build_catalog(workshop_dir, user_zomboid_dir, None);
    let report = health_report(&catalog, active);
    ModHealth { catalog, report }
}

/// `mod_health` plus shared-file conflicts (needs the game folder for the vanilla files).
pub fn mod_health_full(paths: &crate::vfs::StudioPaths, active: &[String]) -> ModHealth {
    let catalog = build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let mut report = health_report(&catalog, active);
    report.install = read_game_install(&paths.pz_install_dir);
    add_user_rule_issues(&mut report, &catalog, active, &crate::rules::load_rules(&paths.user_zomboid_dir));
    if let Some(community) = crate::rules::community::current(&paths.user_zomboid_dir) {
        add_community_rule_issues(&mut report, &catalog, active, &community);
        report.community = Some(community.summary());
    }
    if let Some(index) = crate::rules::index::current(&paths.user_zomboid_dir) {
        let switches = crate::patch_generator::read_patch_switches(&paths.user_zomboid_dir);
        let shim_on = |name: &str| switches.get(name).copied().unwrap_or(true);
        let package_root = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir)
            .into_iter()
            .next()
            .map(|d| d.join("mods").join("Z_PZModStudio_MergedPatch"))
            .unwrap_or_default();
        add_index_issues(&mut report, &catalog, active, &index, &shim_on, &package_root);
        report.community = Some(report.community.take().unwrap_or_else(|| crate::rules::community::summary(&paths.user_zomboid_dir)).with_index(Some(&index)));
    }
    let conflicts = crate::vfs::file_conflict_states(paths, &catalog, active);
    add_file_conflict_issues(&mut report, &conflicts);
    add_package_file_issues(&mut report, &catalog, active);
    if let Some(z) = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next() {
        add_security_issues(&mut report, &catalog, active, &z);
    }
    add_console_flood_issues(&mut report, &catalog, active, &crate::sandbox::error_stats_from_console(&paths.user_zomboid_dir));
    // A package is a mod of its own, in every profile's list: it was built for one list, and this
    // profile's may not be it.
    if let Some((package_id, root)) = crate::vfs::active_package_dir(&catalog, active) {
        if let Some(meta) = std::fs::read_to_string(root.join("patch_metadata.json")).ok().and_then(|t| serde_json::from_str::<crate::patch_generator::MasterPatchMetadata>(&t).ok()) {
            let active_ids: Vec<String> = active
                .iter()
                .map(|e| resolve_entry(&catalog, e.trim()).map(|m| m.id.clone()).unwrap_or_else(|| e.trim().to_string()))
                .collect();
            if let Some(issue) = package_list_issue(&package_id, &meta.packaged_mod_ids, &active_ids) {
                report.issues.push(issue);
            }
        }
    }
    ModHealth { catalog, report }
}

/// From this many console lines of trouble in one session, a mod "floods the console". Measured
/// on real sessions: Fancy Handwork's key handler threw 359 times for 9,330 lines; a mod that threw
/// 51 times made 407, which is a broken mod (the Monitor's business) more than a flood.
pub const FLOOD_LINES: usize = 1000;

/// The mods on in this list whose errors filled the last session's console (`console.txt`): the
/// game slows down writing them, and every other mod's errors are lost among them.
pub fn add_console_flood_issues(report: &mut HealthReport, catalog: &ModCatalog, active: &[String], stats: &[crate::sandbox::ModErrorStat]) {
    let total: usize = stats.iter().map(|s| s.lines).sum();
    let on: Vec<&CatalogMod> = active.iter().filter_map(|e| resolve_entry(catalog, e.trim())).collect();
    let mut list = IssueList(Vec::new());
    for stat in stats.iter().filter(|s| s.lines >= FLOOD_LINES) {
        // The game writes the name from mod.info; a mod with none is written by its id.
        let Some(m) = on.iter().find(|m| m.name == stat.mod_name).or_else(|| on.iter().find(|m| m.id == stat.mod_name)) else { continue };
        let share = if total > 0 { stat.lines * 100 / total } else { 0 };
        let (lines, share_s, exceptions, warnings) = (stat.lines.to_string(), share.to_string(), stat.exceptions.to_string(), stat.warnings.to_string());
        list.push(Severity::Warning, "floods_console", Some(&m.id),
            format!("{} filled the console in the last session: {} lines of errors ({}% of all of them), from {} exceptions and {} warnings. Writing them slows the game, and other mods' errors are lost among them. Look at them in the Monitor, or turn it off in this profile.",
                describe(m), lines, share_s, exceptions, warnings),
            vec![SuggestedFix::DisableMod { mod_id: m.id.clone() }]);
        let name = describe(m);
        list.with(&[("mod", &name), ("lines", &lines), ("share", &share_s), ("exceptions", &exceptions), ("warnings", &warnings)]);
    }
    if list.0.is_empty() {
        return;
    }
    report.issues.extend(list.0);
    report.issues.sort_by(|a, b| a.severity.cmp(&b.severity));
    report.warnings = report.issues.iter().filter(|i| i.severity == Severity::Warning).count();
}

/// The package was built with one list and this profile has another: the mods it merged that
/// are off here still have their merged files in it, and the mods on here that it never saw
/// are not covered. Info: nothing breaks by itself, but the Merger's rescan and publish
/// settle it. None when the lists agree.
pub fn package_list_issue(package_id: &str, packaged: &[String], active: &[String]) -> Option<HealthIssue> {
    let is_mod = |id: &String| !id.starts_with("Z_PZModStudio_") && id != "PZModStudioCarrier";
    let missing: Vec<&String> = active.iter().filter(|id| is_mod(id) && !packaged.contains(id)).collect();
    let extra: Vec<&String> = packaged.iter().filter(|id| is_mod(id) && !active.contains(id)).collect();
    if missing.is_empty() && extra.is_empty() {
        return None;
    }
    let few = |list: &[&String]| {
        let shown: Vec<&str> = list.iter().take(4).map(|s| s.as_str()).collect();
        format!("{}{}", shown.join(", "), if list.len() > 4 { format!(", and {} more", list.len() - 4) } else { String::new() })
    };
    let mut parts = Vec::new();
    if !missing.is_empty() {
        parts.push(format!("{} mod{} on in this profile that it never saw ({})", missing.len(), if missing.len() == 1 { " is" } else { "s are" }, few(&missing)));
    }
    if !extra.is_empty() {
        parts.push(format!("{} mod{} it merged {} off here, and its merged files stay in the package ({})", extra.len(), if extra.len() == 1 { "" } else { "s" }, if extra.len() == 1 { "is" } else { "are" }, few(&extra)));
    }
    Some(HealthIssue {
        severity: Severity::Info,
        code: "package_built_for_other_list".to_string(),
        mod_id: Some(package_id.to_string()),
        message: format!(
            "{} was built with a different list: {}. A package is a mod of its own, so it shows in every profile; its merges only settle the mods it was built with. In the Mod Merger, open it, rescan and publish to rebuild it for this list, or switch it off in this profile.",
            package_id,
            parts.join("; ")
        ),
        fixes: vec![SuggestedFix::DisableMod { mod_id: package_id.to_string() }],
        related: Vec::new(),
        details: Vec::new(),
        params: {
            let shown = |list: &[&String]| list.iter().take(4).map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
            let count = |list: &[&String], one: &'static str, other: &'static str| match list.len() {
                0 => "",
                1 => one,
                _ => other,
            };
            let more = |list: &[&String]| list.len().saturating_sub(4);
            issue_params(&[
                ("package", package_id),
                ("missingN", &missing.len().to_string()),
                ("missingList", &shown(&missing)),
                ("missingMoreN", &more(&missing).to_string()),
                ("missingMoreKey", if more(&missing) > 0 { "more_missing" } else { "" }),
                ("missingKey", count(&missing, "pkg_missing_one", "pkg_missing_other")),
                ("extraN", &extra.len().to_string()),
                ("extraList", &shown(&extra)),
                ("extraMoreN", &more(&extra).to_string()),
                ("extraMoreKey", if more(&extra) > 0 { "more_extra" } else { "" }),
                ("extraKey", count(&extra, "pkg_extra_one", "pkg_extra_other")),
                ("sep", if missing.is_empty() || extra.is_empty() { "" } else { "; " }),
            ])
        },
    })
}

/// The quiet conflict: two mods defining the same item, recipe or vehicle in differently named
/// files, which collide nowhere on disk. The game keeps whichever was read last, so the earlier
/// mod's version of that one thing is simply gone, and nothing says so.
pub fn script_definition_issues(paths: &crate::vfs::StudioPaths, active: &[String]) -> Vec<HealthIssue> {
    let catalog = build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let conflicts = crate::scripts::script_definition_conflicts(&catalog, active);
    let states = crate::vfs::definition_merge_states(&catalog, active, &conflicts);
    let mut issues = issues_for_script_conflicts_with_state(&conflicts, &states);
    let language = crate::translations::game_language(&paths.user_zomboid_dir);
    let translations: Vec<_> = crate::translations::translation_conflicts(&catalog, active).into_iter().filter(|c| c.lang == language).collect();
    let package = crate::vfs::active_package_dir(&catalog, active).map(|(_, root)| root);
    issues.extend(issues_for_translation_conflicts(&translations, package.as_ref()));
    issues
}

/// A thing two mods name differently in the same language: one row per language, category and
/// set of mods, Info, since the game shows one of the two and nothing breaks. "Settled" once the
/// package carries the file the Merger wrote.
pub fn issues_for_translation_conflicts(conflicts: &[crate::translations::TranslationConflict], package_root: Option<&std::path::PathBuf>) -> Vec<HealthIssue> {
    let mut groups: Vec<((String, String, Vec<String>), Vec<&crate::translations::TranslationConflict>)> = Vec::new();
    for c in conflicts {
        let key = (c.lang.clone(), c.category.clone(), c.mod_ids.clone());
        match groups.iter_mut().find(|(k, _)| k == &key) {
            Some((_, list)) => list.push(c),
            None => groups.push((key, vec![c])),
        }
    }
    let mut list = IssueList(Vec::new());
    for ((lang, category, mod_ids), group) in groups {
        let c = group[0];
        let files: Vec<String> = group.iter().flat_map(|x| x.absolute_files.iter().cloned()).collect();
        let state = crate::translations::merge_state(package_root, &lang, &category, &files);
        let winner = c.mods.last().cloned().unwrap_or_default();
        let winner_id = mod_ids.last().cloned().unwrap_or_default();
        let others: Vec<&str> = c.mods[..c.mods.len().saturating_sub(1)].iter().map(String::as_str).collect();
        let named: Vec<String> = group.iter().take(3).map(|x| x.key.clone()).collect();
        let example = format!(" {} is {}.", c.key, crate::translations::distinct_values(c));
        let what = if group.len() == 1 { c.key.clone() } else { format!("{} things ({}{})", group.len(), named.join(", "), if group.len() > 3 { ", ..." } else { "" }) };
        let settled = match state.as_str() {
            "merged" => " Your package already carries the words you chose; it loads last, so they show.",
            "stale" => " The words you chose are in your package, but a mod changed since: look again.",
            _ => "",
        };
        let message = format!(
            "{} and {} name {} differently in {} ({}).{} The game reads every mod's translations and keeps the last loaded, so {}'s words show today.{}",
            others.join(", "), winner, what, lang, category, example, winner, settled
        );
        let rel = crate::translations::merged_file_path(&lang, &category);
        let fixes = vec![SuggestedFix::OpenInMerger { relative_path: rel }];
        let (severity, code) = match state.as_str() {
            "stale" => (Severity::Warning, "translation_stale"),
            "merged" => (Severity::Info, "translation_settled"),
            _ => (Severity::Info, "translation_conflict"),
        };
        list.push(severity, code, Some(&winner_id), message, fixes);
        list.with(&[
            ("template", "translation"),
            ("losers", &others.join(", ")), ("winner", &winner), ("lang", &lang), ("category", &category),
            ("what", if group.len() == 1 { &c.key } else { "" }), ("whatKey", if group.len() == 1 { "" } else { "things" }),
            ("n", &group.len().to_string()), ("named", &named.join(", ")), ("ellipsis", if group.len() > 3 { ", ..." } else { "" }),
            ("exName", &c.key), ("exValuesVs", &crate::translations::distinct_value_list(c).join("\u{1f}")), ("exampleKey", "tr_example"),
            ("settledKey", match state.as_str() { "merged" => "tr_merged", "stale" => "tr_stale", _ => "" }),
        ]);
        if let Some(last) = list.0.last_mut() {
            last.related = mod_ids[..mod_ids.len().saturating_sub(1)].to_vec();
            last.details = group
                .iter()
                .flat_map(|x| x.mod_ids.iter().enumerate().map(move |(i, id)| (id.clone(), x.mods[i].clone(), x.absolute_files[i].clone(), x.key.clone())))
                .fold(Vec::<IssueDetail>::new(), |mut acc, (id, name, file, key)| {
                    let merged_path = crate::translations::merged_file_path(&lang, &category);
                    let thing = DeclaredThing { definition: format!("{} = {}", key, group.iter().find(|x| x.key == key).and_then(|x| x.mod_ids.iter().position(|m| m == &id).map(|i| x.values[i].clone())).unwrap_or_default()), merged_path, state: state.clone() };
                    match acc.iter_mut().find(|d| d.mod_id == id && d.file == file) {
                        Some(d) => d.declares.push(thing),
                        None => acc.push(IssueDetail { mod_id: id, mod_name: name, file, declares: vec![thing] }),
                    }
                    acc
                });
        }
    }
    list.0
}

/// Whether each conflict is already settled in the active package, and whether a mod has changed
/// since it was. Same three words the file conflicts use: pending, merged, stale.
pub fn issues_for_script_conflicts(conflicts: &[crate::scripts::ScriptDefConflict]) -> Vec<HealthIssue> {
    let states = vec!["pending".to_string(); conflicts.len()];
    issues_for_script_conflicts_with_state(conflicts, &states)
}

/// The same group file by file: for each mod and each of its files, the things in it that the
/// group is about, in load order, so a person can see which file to open rather than a list of
/// twenty files joined by slashes.
fn details_by_file(group: &[(&crate::scripts::ScriptDefConflict, &str)]) -> Vec<IssueDetail> {
    let mut details: Vec<IssueDetail> = Vec::new();
    for (conflict, state) in group {
        for (i, mod_id) in conflict.mod_ids.iter().enumerate() {
            let file = conflict.files.get(i).cloned().unwrap_or_default();
            let thing = DeclaredThing { definition: conflict.definition.clone(), merged_path: crate::scripts::merged_file_path(conflict), state: state.to_string() };
            match details.iter_mut().find(|d| &d.mod_id == mod_id && d.file == file) {
                Some(d) => d.declares.push(thing),
                None => details.push(IssueDetail {
                    mod_id: mod_id.clone(),
                    mod_name: conflict.mods.get(i).cloned().unwrap_or_else(|| mod_id.clone()),
                    file,
                    declares: vec![thing],
                }),
            }
        }
    }
    details
}

/// Turns what two mods both declare into issues, newest first, capped so one messy list cannot
/// bury the problems that stop the game loading.
pub fn issues_for_script_conflicts_with_state(
    conflicts: &[crate::scripts::ScriptDefConflict],
    states: &[String],
) -> Vec<HealthIssue> {
    const SHOWN: usize = 30;
    let mut list = IssueList(Vec::new());

    // Two mods that disagree about one item usually disagree about twenty: the same pair, the
    // same files, the same decision. Grouped by the mods involved, in the order they were found.
    let mut groups: Vec<(Vec<String>, Vec<(&crate::scripts::ScriptDefConflict, &str)>)> = Vec::new();
    for (i, c) in conflicts.iter().enumerate() {
        let state = states.get(i).map(String::as_str).unwrap_or("pending");
        match groups.iter_mut().find(|(ids, _)| ids == &c.mod_ids) {
            Some((_, list)) => list.push((c, state)),
            None => groups.push((c.mod_ids.clone(), vec![(c, state)])),
        }
    }

    for (_, group) in groups.iter().take(SHOWN) {
        let c = group[0].0;
        // A group is settled only when every definition in it is, and stale if any mod changed
        // after the merge was made.
        let pending: Vec<&crate::scripts::ScriptDefConflict> =
            group.iter().filter(|(_, s)| *s != "merged").map(|(c, _)| *c).collect();
        let stale = group.iter().any(|(_, s)| *s == "stale");
        let settled = pending.is_empty();
        let winner = c.mods.last().cloned().unwrap_or_default();
        let winner_id = c.mod_ids.last().cloned().unwrap_or_default();
        let losers: Vec<&str> = c.mods[..c.mods.len().saturating_sub(1)].iter().map(String::as_str).collect();
        // What they disagree about, with a few named so it is recognisable.
        let named: Vec<&str> = group.iter().take(3).map(|(x, _)| x.definition.as_str()).collect();
        let what = if group.len() == 1 {
            named[0].to_string()
        } else {
            format!(
                "{} things ({}{})",
                group.len(),
                named.join(", "),
                if group.len() > named.len() { ", ..." } else { "" }
            )
        };
        // The game applies every declaration to the same thing in turn, so what is at stake is
        // not the whole definition but the handful of properties two mods set differently.
        let mut keys: Vec<&str> = group
            .iter()
            .flat_map(|(x, _)| x.clashing.iter().map(|p| p.key.as_str()))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        let shown: Vec<&str> = keys.iter().take(4).copied().collect();
        let properties = format!(
            "{}{}",
            shown.join(", "),
            if keys.len() > shown.len() { ", ..." } else { "" }
        );
        // One example, with long values cut: a recipe list can run to three hundred characters.
        let short = |v: &str| {
            if v.chars().count() > 60 {
                format!("{}...", v.chars().take(60).collect::<String>())
            } else {
                v.to_string()
            }
        };
        let example = group
            .iter()
            .find_map(|(x, _)| x.clashing.first().map(|p| (x, p)))
            .map(|(x, p)| {
                let values: Vec<String> = p
                    .values
                    .iter()
                    .enumerate()
                    .filter_map(|(i, v)| v.as_ref().map(|v| format!("{} = {}", x.mods[i], short(v))))
                    .collect();
                format!(" On {}, that is {}.", x.definition, values.join(" against "))
            })
            .unwrap_or_default();

        let each = if c.mod_ids.len() > 2 { "every one of them" } else { "both mods" };
        let message = if keys.is_empty() {
            // Nothing to compare property by property: the block's contents are nested blocks
            // (an entity's skin, a recipe's ingredients), so only that they differ is known.
            format!(
                "{} and {} all declare {}, in different files ({}), and their contents differ. \
                 What differs is inside nested blocks, which the Studio does not compare, so it \
                 cannot say what is at stake here — only that {} loads last.",
                losers.join(", "),
                winner,
                what,
                c.files.join(" / "),
                winner,
            )
        } else {
            format!(
                "{} and {} {} set {} on {}, in different files ({}). The game applies {} to the \
                 same {}, so everything else each of them sets is kept; only {} decided by the \
                 load order, and {} loads last.{}",
                losers.join(", "),
                winner,
                if c.mod_ids.len() > 2 { "all" } else { "both" },
                properties,
                what,
                c.files.join(" / "),
                each,
                if group.len() == 1 { "thing" } else { "things" },
                if keys.len() == 1 { "that property is" } else { "those properties are" },
                winner,
                example,
            )
        };

        // What has already been done about it, which is the difference between a decision to
        // make and one that only needs making again because a mod changed underneath it.
        let settled_note = if settled {
            " Your package already settles this; it loads last, so what is in it wins.".to_string()
        } else if stale {
            format!(
                " {} of these were merged into your package before one of the mods changed, so \
                 the package no longer matches them: merge them again.",
                group.len() - pending.len() + pending.len()
            )
        } else {
            String::new()
        };
        let message = format!("{}{}", message, settled_note);
        // Settling it properly means a small file in the package, which is the Merger's job:
        // it keeps what both mods added where that is possible and leaves one word to choose
        // where it is not. Swapping the load order is still offered, as the blunt way to the
        // same answer, but it comes last.
        let mergeable: Vec<String> = pending
            .iter()
            .filter(|x| !x.clashing.is_empty() && x.clashing.iter().all(|p| crate::scripts::union_of(&p.values).is_some()))
            .map(|x| crate::scripts::merged_file_path(x))
            .collect();
        let mut fixes: Vec<SuggestedFix> = Vec::new();
        if !mergeable.is_empty() {
            fixes.push(SuggestedFix::MergeFiles { relative_paths: mergeable });
        }
        if let Some(first) = pending.first() {
            fixes.push(SuggestedFix::OpenInMerger {
                relative_path: crate::scripts::merged_file_path(first),
            });
        }
        if c.mod_ids.len() == 2 && !settled {
            fixes.push(SuggestedFix::MoveBefore {
                mod_id: winner_id.clone(),
                before: c.mod_ids[0].clone(),
            });
        }
        let severity = if stale { Severity::Warning } else { Severity::Info };
        // A settled group has its own code, so a row can say "settled" instead of "conflict".
        let code = if stale {
            "script_definition_stale"
        } else if settled {
            "script_definition_settled"
        } else {
            "script_definition_conflict"
        };
        let example_params = group.iter().find_map(|(x, _)| x.clashing.first().map(|p| (x, p))).map(|(x, p)| {
            let values: Vec<String> = p.values.iter().enumerate().filter_map(|(i, v)| v.as_ref().map(|v| format!("{} = {}", x.mods[i], short(v)))).collect();
            (x.definition.clone(), values.join("\u{1f}"))
        });
        list.push(severity, code, Some(&winner_id), message, fixes);
        let many = c.mod_ids.len() > 2;
        list.with(&[
            ("template", if keys.is_empty() { "definition_nested" } else { "definition" }),
            ("losers", &losers.join(", ")), ("winner", &winner), ("files", &c.files.join(" / ")), ("properties", &properties),
            ("what", if group.len() == 1 { named[0] } else { "" }), ("whatKey", if group.len() == 1 { "" } else { "things" }),
            ("n", &group.len().to_string()), ("named", &named.join(", ")), ("ellipsis", if group.len() > named.len() { ", ..." } else { "" }),
            ("allKey", if many { "all" } else { "both" }), ("eachKey", if many { "each_all" } else { "each_both" }),
            ("thingsKey", if group.len() == 1 { "thing_one" } else { "thing_other" }), ("propsKey", if keys.len() == 1 { "props_one" } else { "props_other" }),
            ("exDef", example_params.as_ref().map(|e| e.0.as_str()).unwrap_or("")), ("exValuesVs", example_params.as_ref().map(|e| e.1.as_str()).unwrap_or("")),
            ("exampleKey", if example_params.is_some() { "def_example" } else { "" }),
            ("staleN", &group.len().to_string()),
            ("settledKey", if settled { "def_settled" } else if stale { "def_stale" } else { "" }),
        ]);
        if let Some(last) = list.0.last_mut() {
            last.related = c.mod_ids[..c.mod_ids.len().saturating_sub(1)].to_vec();
            last.details = details_by_file(group);
        }
    }
    if groups.len() > SHOWN {
        let left: usize = groups.iter().skip(SHOWN).map(|(_, g)| g.len()).sum();
        list.push(
            Severity::Info,
            "script_definition_conflict_more",
            None,
            format!(
                "{} more things, across {} other pairs of mods, have a property two mods set \
                 differently. Whichever mod loads last decides each one.",
                left,
                groups.len() - SHOWN
            ),
            Vec::new(),
        );
        list.with(&[("left", &left.to_string()), ("pairs", &(groups.len() - SHOWN).to_string())]);
    }
    list.0
}

/// Shared files the game would load from only one mod: "pending" (not merged) and "stale"
/// (merged, but a mod changed since) become warnings; "merged" ones are fine.
pub fn add_file_conflict_issues(report: &mut HealthReport, conflicts: &[crate::vfs::FileConflictState]) {
    let mut list = IssueList(Vec::new());
    for c in conflicts.iter().filter(|c| c.state != "merged") {
        let winner = c.mods.last().cloned().unwrap_or_default();
        let others: Vec<&str> = c.mods[..c.mods.len().saturating_sub(1)].iter().map(String::as_str).collect();
        let message = if c.state == "stale" {
            format!("{} was merged, but {} changed since. Merge it again so the package matches the current versions.", c.relative_path, c.mods.join(" / "))
        } else {
            format!(
                "{} and {} both ship {}. Without a merge the game only uses {}'s copy, so {}'s version is ignored. {}",
                others.join(", "), winner, c.relative_path, winner, others.join(", "),
                if c.auto_mergeable { "It can be merged automatically." } else { "They change the same lines differently: choose in the Mod Merger." }
            )
        };
        let mut fixes = Vec::new();
        if c.auto_mergeable {
            fixes.push(SuggestedFix::MergeFiles { relative_paths: vec![c.relative_path.clone()] });
        }
        fixes.push(SuggestedFix::OpenInMerger { relative_path: c.relative_path.clone() });
        let code = if c.state == "stale" { "file_conflict_stale" } else { "file_conflict" };
        list.push(Severity::Warning, code, c.mod_ids.last().map(String::as_str), message, fixes);
        list.with(&[
            ("path", &c.relative_path), ("mods", &c.mods.join(" / ")), ("others", &others.join(", ")), ("winner", &winner),
            ("howKey", if c.auto_mergeable { "fc_auto" } else { "fc_manual" }),
        ]);
        if let Some(last) = list.0.last_mut() {
            last.related = c.mod_ids[..c.mod_ids.len().saturating_sub(1)].to_vec();
        }
    }
    report.issues.extend(list.0);
    report.issues.sort_by(|a, b| a.severity.cmp(&b.severity));
    report.warnings = report.issues.iter().filter(|i| i.severity == Severity::Warning).count();
}

/// The active package's copies of mod files that the mod has changed since (the game reads the
/// copy, so the mod's change never runs), and the copies no mod in the list ships.
pub fn add_package_file_issues(report: &mut HealthReport, catalog: &ModCatalog, active: &[String]) {
    let mut list = IssueList(Vec::new());
    for f in crate::patch_generator::stale::package_file_states(catalog, active) {
        if f.conflict {
            continue; // two mods ship it: a merge, reported with the file conflicts
        }
        if f.stale {
            let mod_name = f.mod_name.clone().unwrap_or_default();
            let mod_date = f.mod_date.clone().unwrap_or_default();
            let message = format!(
                "{} carries a copy of {} made on {}, and {} changed its own file on {}. The game reads the copy, so that change never runs, and the copy may call things the new version no longer has. Take the file out of the package, or patch the new version in the Merger.",
                f.package, f.relative_path, f.copy_date, mod_name, mod_date
            );
            let fixes = vec![
                SuggestedFix::RemoveFromPackage { relative_path: f.relative_path.clone(), package: f.package_folder.clone() },
                SuggestedFix::OpenInMerger { relative_path: f.relative_path.clone() },
            ];
            list.push(Severity::Warning, "package_file_stale", f.mod_id.as_deref(), message, fixes);
            list.with(&[("package", &f.package), ("path", &f.relative_path), ("copyDate", &f.copy_date), ("mod", &mod_name), ("modDate", &mod_date)]);
        } else if f.differs {
            let mod_name = f.mod_name.clone().unwrap_or_default();
            let message = format!(
                "{} carries a copy of {} that is not what {} ships, and {} is the only mod in this list with that file. Either a merge made when another mod shipped it too (that mod is off now), or a patch of yours. If the merge is no longer needed, take it out: the game then reads {}'s own file.",
                f.package, f.relative_path, mod_name, mod_name, mod_name
            );
            let fixes = vec![
                SuggestedFix::RemoveFromPackage { relative_path: f.relative_path.clone(), package: f.package_folder.clone() },
                SuggestedFix::OpenInMerger { relative_path: f.relative_path.clone() },
            ];
            list.push(Severity::Info, "package_file_differs", f.mod_id.as_deref(), message, fixes);
            list.with(&[("package", &f.package), ("path", &f.relative_path), ("mod", &mod_name)]);
        } else if f.orphan {
            let message = format!(
                "{} carries {} (copied on {}), and no mod in this list ships that file. It loads anyway. If it is a patch of yours it is fine there; if it came from a mod that is off or gone, take it out.",
                f.package, f.relative_path, f.copy_date
            );
            let fixes = vec![SuggestedFix::RemoveFromPackage { relative_path: f.relative_path.clone(), package: f.package_folder.clone() }];
            list.push(Severity::Info, "package_file_orphan", None, message, fixes);
            list.with(&[("package", &f.package), ("path", &f.relative_path), ("copyDate", &f.copy_date)]);
        }
    }
    if list.0.is_empty() {
        return;
    }
    report.issues.extend(list.0);
    report.issues.sort_by(|a, b| a.severity.cmp(&b.severity));
    report.warnings = report.issues.iter().filter(|i| i.severity == Severity::Warning).count();
}

/// What the active mods could do beyond modding, one issue per mod and kind: programs shipped
/// (`security_executable`), Java that runs programs or reaches the net (`security_java`), Lua
/// that does what the game's never does (`security_lua`). A capability is information ("Good
/// to know"); only a pattern with no good reason in a mod (credential files, a Discord webhook,
/// input control, code fetched from the net, a telling file name) is a Warning. Never a verdict.
pub fn add_security_issues(report: &mut HealthReport, catalog: &ModCatalog, active: &[String], user_zomboid_dir: &Path) {
    use crate::security::{capability, Risk};
    /// A pattern no mod has a good reason for, as opposed to a capability many mods use.
    fn is_red_flag(rule: &str) -> bool {
        matches!(rule, "java_credentials" | "java_webhook" | "java_input" | "lua_remote_require" | "suspect_name")
    }
    let mut list = IssueList(Vec::new());
    let active_ids: Vec<String> = active.iter().map(|e| e.trim().to_string()).collect();
    for scan in crate::security::scan_mods(user_zomboid_dir, catalog, &active_ids) {
        let Some(m) = catalog.get(&scan.mod_id) else { continue };
        let fixes = match &m.source {
            ModSource::Workshop { workshop_id } => vec![SuggestedFix::OpenWorkshopPage { workshop_id: workshop_id.clone() }],
            _ => Vec::new(),
        };
        let flagged: Vec<&crate::security::Finding> = scan.findings.iter().filter(|f| f.risk >= Risk::Medium).collect();
        if flagged.is_empty() {
            continue;
        }
        // programs
        let programs: Vec<&&crate::security::Finding> = flagged.iter().filter(|f| f.rule == "executable" || f.rule == "suspect_name" || f.rule == "script").collect();
        if !programs.is_empty() {
            let files: Vec<String> = programs.iter().take(4).map(|f| f.file.clone()).collect();
            let more = programs.len().saturating_sub(4);
            let binaries = programs.iter().any(|f| f.rule != "script");
            let red_flag = programs.iter().any(|f| is_red_flag(&f.rule));
            let message = if binaries {
                format!(
                    "{} ships {} program file(s): {}{}. A program in a mod runs with your rights, outside the game's Lua; keep the mod only if you trust its author. Nothing here says the file is harmful.",
                    m.name, programs.len(), files.join(", "), if more > 0 { format!(" and {} more", more) } else { String::new() }
                )
            } else {
                format!(
                    "{} ships {} script(s): {}{}. A script in a mod folder runs only if you run it yourself (these are usually the author's build helpers); nothing to do unless you open them.",
                    m.name, programs.len(), files.join(", "), if more > 0 { format!(" and {} more", more) } else { String::new() }
                )
            };
            list.push(if red_flag { Severity::Warning } else { Severity::Info }, if binaries { "security_executable" } else { "security_script" }, Some(&m.id), message, fixes.clone());
            list.with(&[("mod", &m.name), ("count", &programs.len().to_string()), ("files", &files.join(", ")), ("more", &more.to_string()), ("moreKey", if more > 0 { "and_more" } else { "" })]);
        }
        // Java
        let java: Vec<&&crate::security::Finding> = flagged.iter().filter(|f| f.rule.starts_with("java_")).collect();
        if !java.is_empty() {
            let mut rules: Vec<&str> = java.iter().map(|f| f.rule.as_str()).collect();
            rules.sort();
            rules.dedup();
            let what: Vec<&str> = rules.iter().map(|r| capability(r)).collect::<BTreeSet<_>>().into_iter().collect();
            let mut classes: Vec<String> = java.iter().map(|f| f.file.rsplit('!').next().unwrap_or(&f.file).trim_end_matches(".class").to_string()).collect();
            classes.sort();
            classes.dedup();
            let shown: Vec<String> = classes.iter().take(3).cloned().collect();
            let high = java.iter().any(|f| is_red_flag(&f.rule));
            let message = format!(
                "{} carries Java code that can {} ({}{}). Build 42 runs a mod's Java with your rights, through ZombieBuddy, which asks before loading a new jar. Many mods do this for good reasons; keep the mod only if you trust its author.",
                m.name, what.join(", "), shown.join(", "), if classes.len() > 3 { format!(" and {} more classes", classes.len() - 3) } else { String::new() }
            );
            list.push(if high { Severity::Warning } else { Severity::Info }, "security_java", Some(&m.id), message, fixes.clone());
            list.with(&[("mod", &m.name), ("rulesList", &rules.join(",")), ("where", &shown.join(", ")), ("more", &classes.len().saturating_sub(3).to_string()), ("moreKey", if classes.len() > 3 { "and_more_classes" } else { "" })]);
        }
        // Lua
        let lua: Vec<&&crate::security::Finding> = flagged.iter().filter(|f| f.rule.starts_with("lua_")).collect();
        if !lua.is_empty() {
            let mut rules: Vec<&str> = lua.iter().map(|f| f.rule.as_str()).collect();
            rules.sort();
            rules.dedup();
            let what: Vec<&str> = rules.iter().map(|r| capability(r)).collect::<BTreeSet<_>>().into_iter().collect();
            let mut places: Vec<String> = lua.iter().map(|f| match f.line { Some(l) => format!("{}:{}", f.file, l), None => f.file.clone() }).collect();
            places.sort();
            places.dedup();
            let shown: Vec<String> = places.iter().take(3).cloned().collect();
            let high = lua.iter().any(|f| is_red_flag(&f.rule));
            let message = format!(
                "{}'s Lua does things the game's own scripts never do: {} ({}{}). The game's Lua cannot run programs or reach the internet by itself, so this is odd rather than dangerous; the file says what it is for.",
                m.name, what.join(", "), shown.join(", "), if places.len() > 3 { format!(" and {} more places", places.len() - 3) } else { String::new() }
            );
            list.push(if high { Severity::Warning } else { Severity::Info }, "security_lua", Some(&m.id), message, fixes.clone());
            list.with(&[("mod", &m.name), ("rulesList", &rules.join(",")), ("where", &shown.join(", ")), ("more", &places.len().saturating_sub(3).to_string()), ("moreKey", if places.len() > 3 { "and_more_places" } else { "" })]);
        }
    }
    if list.0.is_empty() {
        return;
    }
    report.issues.extend(list.0);
    report.issues.sort_by(|a, b| a.severity.cmp(&b.severity));
    report.warnings = report.issues.iter().filter(|i| i.severity == Severity::Warning).count();
}

// ---------------------------------------------------------------------------------------------
// Health report
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The game will not load the mod, or it will break.
    Error,
    /// Likely problems (load order, duplicates).
    Warning,
    /// Worth knowing, nothing to fix.
    Info,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SuggestedFix {
    EnableMod { mod_id: String },
    DisableMod { mod_id: String },
    MoveBefore { mod_id: String, before: String },
    RemoveFromList { entry: String },
    SubscribeWorkshop { hint: String },
    /// Open the item's Workshop page in the browser.
    OpenWorkshopPage { workshop_id: String },
    /// Copy the mod to Zomboid/mods and adapt it to the installed build.
    MakeCompatibleCopy { mod_id: String },
    /// Keep a copy of the version installed today, before Steam replaces it.
    KeepVersion { mod_id: String },
    /// Remove a compatible copy (back to the original).
    RemoveCompatibleCopy { folder: String },
    /// Merge these shared files automatically into the Studio package and publish it.
    MergeFiles { relative_paths: Vec<String> },
    /// Open this shared file in the Mod Merger to decide by hand.
    OpenInMerger { relative_path: String },
    /// Switch one of the shim's per-mod patches on (Settings > Build 42 shim).
    EnableShimPatch { name: String },
    /// Take a mod file's copy out of the Studio package (it is published again without it).
    RemoveFromPackage { relative_path: String, package: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIssue {
    pub severity: Severity,
    /// Stable machine code: missing_mod, not_for_build, outside_version_range, invalid_mod_info,
    /// missing_dependency, dependency_not_enabled, dependency_unavailable, dependency_after,
    /// load_after_order, load_before_order, incompatible, duplicate_entry, shadowed_copy.
    pub code: String,
    pub mod_id: Option<String>,
    pub message: String,
    pub fixes: Vec<SuggestedFix>,
    /// The other mod ids involved (the dependency, the incompatible mod, ...), for UIs that
    /// show the issue next to each mod.
    pub related: Vec<String>,
    /// The same thing file by file, when the message lumps several: what each mod declares in
    /// which file, and where the merged block for each would live. Empty for most issues.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<IssueDetail>,
    /// The values the message is made of, so the app can say it in the player's language: the
    /// key is `code` (or `code_variant`, or `template`), and a `fooKey` param names a phrase the
    /// app looks up for `{{foo}}`. The English `message` stays the text for everyone else.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, String>,
}

/// One file of one mod, and the things in it that another mod also declares.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueDetail {
    pub mod_id: String,
    pub mod_name: String,
    pub file: String,
    pub declares: Vec<DeclaredThing>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclaredThing {
    /// `item Base.Axe`, as the game would name it.
    pub definition: String,
    /// The package file a merged block of it is written to.
    pub merged_path: String,
    /// pending, merged or stale, as the file conflicts say it.
    pub state: String,
}

/// A message's values as `HealthIssue::params` holds them.
pub fn issue_params(params: &[(&str, &str)]) -> BTreeMap<String, String> {
    params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Collects issues; `related` is filled from the fixes (every fix names the other mod).
struct IssueList(Vec<HealthIssue>);

impl IssueList {
    fn push(&mut self, severity: Severity, code: &str, mod_id: Option<&str>, message: String, fixes: Vec<SuggestedFix>) {
        let mut related: Vec<String> = Vec::new();
        for fix in &fixes {
            let other = match fix {
                SuggestedFix::EnableMod { mod_id: m } | SuggestedFix::DisableMod { mod_id: m } => Some(m),
                SuggestedFix::MoveBefore { mod_id: m, before } => Some(if Some(m.as_str()) == mod_id { before } else { m }),
                SuggestedFix::SubscribeWorkshop { hint } => Some(hint),
                SuggestedFix::RemoveFromList { .. }
                | SuggestedFix::OpenWorkshopPage { .. }
                | SuggestedFix::MakeCompatibleCopy { .. }
                | SuggestedFix::KeepVersion { .. }
                | SuggestedFix::RemoveCompatibleCopy { .. }
                | SuggestedFix::MergeFiles { .. }
                | SuggestedFix::OpenInMerger { .. }
                | SuggestedFix::EnableShimPatch { .. }
                | SuggestedFix::RemoveFromPackage { .. } => None,
            };
            if let Some(other) = other.filter(|o| Some(o.as_str()) != mod_id && !related.contains(o)) {
                related.push(other.clone());
            }
        }
        self.0.push(HealthIssue { severity, code: code.to_string(), mod_id: mod_id.map(str::to_string), message, fixes, related, details: vec![], params: BTreeMap::new() });
    }

    /// The message's values, on the issue just pushed.
    fn with(&mut self, params: &[(&str, &str)]) {
        if let Some(last) = self.0.last_mut() {
            last.params = issue_params(params);
        }
    }

    fn relate_last(&mut self, other: &str) {
        if let Some(last) = self.0.last_mut() {
            last.related.push(other.to_string());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub game_version: Option<String>,
    /// What Steam says about the installed game: the branch and when it last updated.
    #[serde(default)]
    pub install: Option<GameInstall>,
    /// The community rules in use, when a copy is at hand.
    #[serde(default)]
    pub community: Option<crate::rules::community::CommunitySummary>,
    pub active_count: usize,
    pub errors: usize,
    pub warnings: usize,
    pub issues: Vec<HealthIssue>,
}

/// The player's own load-order rules, checked against the active list. A broken rule is a
/// warning in the player's words, with the move that mends it; a rule about a mod that is off
/// or not installed says nothing, like the game's own rules.
pub fn add_user_rule_issues(report: &mut HealthReport, catalog: &ModCatalog, active: &[String], rules: &[crate::rules::UserRule]) {
    if rules.is_empty() {
        return;
    }
    let active_ids: Vec<String> = active
        .iter()
        .map(|e| resolve_entry(catalog, e.trim()).map(|m| m.id.clone()).unwrap_or_else(|| e.trim().to_string()))
        .collect();
    let position: HashMap<&str, usize> = active_ids.iter().enumerate().rev().map(|(i, id)| (id.as_str(), i)).collect();
    let mut list = IssueList(Vec::new());
    for rule in rules {
        let (Some(&at), Some(&after_at)) = (position.get(rule.mod_id.as_str()), position.get(rule.after.as_str())) else { continue };
        if at < after_at {
            let name = |id: &str| catalog.get(id).map(describe).unwrap_or_else(|| id.to_string());
            list.push(Severity::Warning, "user_rule_order", Some(&rule.mod_id),
                format!("You asked for {} to load after {}{}, and it comes before it in the list",
                    name(&rule.mod_id), name(&rule.after),
                    rule.note.as_deref().map(|n| format!(" ({})", n)).unwrap_or_default()),
                vec![SuggestedFix::MoveBefore { mod_id: rule.after.clone(), before: rule.mod_id.clone() }]);
            list.with(&[("mod", &name(&rule.mod_id)), ("after", &name(&rule.after)), ("note", &rule.note.as_deref().map(|n| format!(" ({})", n)).unwrap_or_default())]);
        }
    }
    for issue in list.0 {
        match issue.severity {
            Severity::Error => report.errors += 1,
            Severity::Warning => report.warnings += 1,
            Severity::Info => {}
        }
        report.issues.push(issue);
    }
}

#[cfg(test)]
mod user_rule_tests {
    use super::*;

    #[test]
    fn a_package_built_for_another_list_is_an_info_and_a_matching_one_is_nothing() {
        let packaged = vec!["A".to_string(), "B".to_string(), "Z_PZModStudio_Bridge".to_string()];
        let active = vec!["A".to_string(), "C".to_string(), "Z_PZModStudio_MergedPatch".to_string()];
        let issue = package_list_issue("Z_PZModStudio_MergedPatch", &packaged, &active).unwrap();
        assert_eq!(issue.code, "package_built_for_other_list");
        assert!(issue.message.contains("1 mod is on in this profile that it never saw (C)"), "{}", issue.message);
        assert!(issue.message.contains("1 mod it merged is off here"), "{}", issue.message);
        assert!(package_list_issue("P", &["A".to_string()], &["A".to_string(), "Z_PZModStudio_MergedPatch".to_string()]).is_none());
    }

    #[test]
    fn a_broken_rule_is_a_warning_with_the_move_that_mends_it() {
        let mut catalog = ModCatalog { game_version: None, mods: Vec::new() };
        for id in ["Overhaul", "Library"] {
            catalog.mods.push(CatalogMod {
                id: id.to_string(), name: id.to_string(), source: ModSource::Local, root_dir: String::new(),
                mod_info_path: None, version_folders: Vec::new(), has_common: false, has_root_mod_info: true,
                loadability: Loadability::Loadable, shadowed_by: None, origin: None, workshop_state: None,
                info: GameModInfo::default(),
            });
        }
        let rules = vec![crate::rules::UserRule { mod_id: "Overhaul".into(), after: "Library".into(), note: Some("recipes vanish".into()) }];
        let active = vec!["Overhaul".to_string(), "Library".to_string()];
        let mut report = HealthReport { game_version: None, install: None, community: None, active_count: 2, errors: 0, warnings: 0, issues: vec![] };
        add_user_rule_issues(&mut report, &catalog, &active, &rules);
        assert_eq!(report.warnings, 1);
        let issue = &report.issues[0];
        assert_eq!(issue.code, "user_rule_order");
        assert!(issue.message.contains("recipes vanish"), "{}", issue.message);
        assert!(matches!(&issue.fixes[0], SuggestedFix::MoveBefore { mod_id, before } if mod_id == "Library" && before == "Overhaul"));

        // In order: nothing to say. A mod that is off: nothing either.
        let mut ok = HealthReport { game_version: None, install: None, community: None, active_count: 2, errors: 0, warnings: 0, issues: vec![] };
        add_user_rule_issues(&mut ok, &catalog, &["Library".to_string(), "Overhaul".to_string()], &rules);
        assert!(ok.issues.is_empty());
        add_user_rule_issues(&mut ok, &catalog, &["Overhaul".to_string()], &rules);
        assert!(ok.issues.is_empty());
    }
}

/// The community's rules against the active list: a broken load-after rule and an incompatible
/// pair both on, each named as the community's with its note and source.
pub fn add_community_rule_issues(report: &mut HealthReport, catalog: &ModCatalog, active: &[String], community: &crate::rules::community::CommunityRules) {
    let active_ids: Vec<String> = active
        .iter()
        .map(|e| resolve_entry(catalog, e.trim()).map(|m| m.id.clone()).unwrap_or_else(|| e.trim().to_string()))
        .collect();
    let position: HashMap<&str, usize> = active_ids.iter().enumerate().rev().map(|(i, id)| (id.as_str(), i)).collect();
    let name = |id: &str| catalog.get(id).map(describe).unwrap_or_else(|| id.to_string());
    // The same, as values: the note as written, and the source behind a phrase of its own.
    let why_params = |note: &Option<String>, source: &Option<String>| {
        (note.as_deref().map(|n| format!(" ({})", n)).unwrap_or_default(), source.clone().unwrap_or_default())
    };
    let why = |note: &Option<String>, source: &Option<String>| {
        let mut s = String::new();
        if let Some(n) = note {
            s.push_str(&format!(" ({})", n));
        }
        if let Some(src) = source {
            s.push_str(&format!(" [source: {}]", src));
        }
        s
    };
    let mut list = IssueList(Vec::new());
    for rule in &community.load_after {
        let (Some(&at), Some(&after_at)) = (position.get(rule.mod_id.as_str()), position.get(rule.after.as_str())) else { continue };
        if at < after_at {
            list.push(Severity::Warning, "community_rule_order", Some(&rule.mod_id),
                format!("The community has it that {} loads after {}{}, and it comes before it in the list",
                    name(&rule.mod_id), name(&rule.after), why(&rule.note, &rule.source)),
                vec![SuggestedFix::MoveBefore { mod_id: rule.after.clone(), before: rule.mod_id.clone() }]);
            let (note, source) = why_params(&rule.note, &rule.source);
            list.with(&[("variant", "rules"), ("mod", &name(&rule.mod_id)), ("after", &name(&rule.after)), ("note", &note), ("source", &source), ("sourceTextKey", if source.is_empty() { "" } else { "source" })]);
        }
    }
    for pair in &community.incompatible {
        if position.contains_key(pair.a.as_str()) && position.contains_key(pair.b.as_str()) {
            list.push(Severity::Warning, "community_incompatible", Some(&pair.a),
                format!("The community has it that {} and {} do not go together{}, and both are on",
                    name(&pair.a), name(&pair.b), why(&pair.note, &pair.source)),
                vec![SuggestedFix::DisableMod { mod_id: pair.b.clone() }, SuggestedFix::DisableMod { mod_id: pair.a.clone() }]);
            let (note, source) = why_params(&pair.note, &pair.source);
            list.with(&[("variant", "rules"), ("mod", &name(&pair.a)), ("other", &name(&pair.b)), ("note", &note), ("source", &source), ("sourceTextKey", if source.is_empty() { "" } else { "source" })]);
        }
    }
    for issue in list.0 {
        match issue.severity {
            Severity::Error => report.errors += 1,
            Severity::Warning => report.warnings += 1,
            Severity::Info => {}
        }
        report.issues.push(issue);
    }
}

/// The community index against the active list: an incompatible pair both on, a load-after
/// relation the list breaks, a known issue not yet dealt with on this machine (with the fix the
/// Studio can apply), and the Build 42 successor of a mod this build cannot load. Each named as
/// the index's, with the build it was seen on and whether it was verified or reported.
pub fn add_index_issues(
    report: &mut HealthReport,
    catalog: &ModCatalog,
    active: &[String],
    index: &crate::rules::index::CommunityIndex,
    shim_on: &dyn Fn(&str) -> bool,
    package_root: &std::path::Path,
) {
    use crate::rules::index::{IssueFix, Status};
    let active_ids: Vec<String> = active
        .iter()
        .map(|e| resolve_entry(catalog, e.trim()).map(|m| m.id.clone()).unwrap_or_else(|| e.trim().to_string()))
        .collect();
    let position: HashMap<&str, usize> = active_ids.iter().enumerate().rev().map(|(i, id)| (id.as_str(), i)).collect();
    let name = |id: &str| catalog.get(id).map(describe).unwrap_or_else(|| id.to_string());
    let seen_on = |build: &Option<String>, status: Status| {
        let who = match status {
            Status::Verified => "verified",
            Status::Reported => "reported",
        };
        match build {
            Some(b) => format!("{} on {}", who, b),
            None => who.to_string(),
        }
    };
    // The same as the phrase the app looks up: verified, reported, verified_on or reported_on.
    let seen_key = |build: &Option<String>, status: Status| {
        match (status, build.is_some()) {
            (Status::Verified, false) => "verified",
            (Status::Reported, false) => "reported",
            (Status::Verified, true) => "verified_on",
            (Status::Reported, true) => "reported_on",
        }
    };
    let note_semi = |note: &Option<String>| note.clone().map(|n| format!("{}; ", n)).unwrap_or_default();
    let mut list = IssueList(Vec::new());
    for id in &active_ids {
        let Some(registry) = index.get(id) else { continue };
        let mine = position[id.as_str()];
        for rel in &registry.incompatible {
            // Each pair once: from the side that comes first in the list, or the only side listed.
            let other_listed = index.get(&rel.id).map(|r| r.incompatible.iter().any(|x| &x.id == id)).unwrap_or(false);
            if let Some(&theirs) = position.get(rel.id.as_str()) {
                if other_listed && theirs < mine {
                    continue;
                }
                list.push(Severity::Warning, "community_incompatible", Some(id),
                    format!("The community index has it that {} and {} do not go together ({}{}), and both are on",
                        name(id), name(&rel.id), rel.note.clone().map(|n| format!("{}; ", n)).unwrap_or_default(), seen_on(&rel.build, rel.status)),
                    vec![SuggestedFix::DisableMod { mod_id: rel.id.clone() }, SuggestedFix::DisableMod { mod_id: id.clone() }]);
                list.with(&[("variant", "index"), ("mod", &name(id)), ("other", &name(&rel.id)), ("note", &note_semi(&rel.note)),
                    ("build", rel.build.as_deref().unwrap_or("")), ("seenKey", seen_key(&rel.build, rel.status))]);
            }
        }
        for rel in &registry.load_after {
            if let Some(&after_at) = position.get(rel.id.as_str()) {
                if mine < after_at {
                    list.push(Severity::Warning, "community_rule_order", Some(id),
                        format!("The community index has it that {} loads after {} ({}{}), and it comes before it in the list",
                            name(id), name(&rel.id), rel.note.clone().map(|n| format!("{}; ", n)).unwrap_or_default(), seen_on(&rel.build, rel.status)),
                        vec![SuggestedFix::MoveBefore { mod_id: rel.id.clone(), before: id.clone() }]);
                    list.with(&[("variant", "index"), ("mod", &name(id)), ("after", &name(&rel.id)), ("note", &note_semi(&rel.note)),
                        ("build", rel.build.as_deref().unwrap_or("")), ("seenKey", seen_key(&rel.build, rel.status))]);
                }
            }
        }
        // The copy the game loads is the Studio's compatible copy: it already rewrote what a
        // compatible copy fixes, so offering to make one again says the opposite of the truth.
        let compat_copy_loads = catalog
            .get(id)
            .map(|m| matches!(m.origin, Some(crate::origins::ModOrigin::Compat { .. })))
            .unwrap_or(false);
        for issue in &registry.issues {
            if crate::rules::index::issue_is_handled(issue, shim_on, package_root) {
                continue;
            }
            if compat_copy_loads && matches!(issue.fix, IssueFix::RequirePath { .. } | IssueFix::CompatibleCopy) {
                continue;
            }
            let evidence = issue.file.as_deref().map(|f| match issue.line { Some(l) => format!(" ({}:{})", f, l), None => format!(" ({})", f) }).unwrap_or_default();
            let note = issue.note.clone().map(|n| format!(": {}", n)).unwrap_or_default();
            let (severity, fixes, how, how_key) = match &issue.fix {
                IssueFix::ShimPatch { name: patch } => (Severity::Warning, vec![SuggestedFix::EnableShimPatch { name: patch.clone() }], "; the shim's patch for it is off".to_string(), "shim_off"),
                IssueFix::RequirePath { from, to } => (
                    Severity::Warning,
                    vec![SuggestedFix::MakeCompatibleCopy { mod_id: id.clone() }],
                    format!("; it requires \"{}\", which Build 42 has at \"{}\" (a compatible copy rewrites it; Fix the path in the Monitor Center publishes the one file)", from, to),
                    "require_path",
                ),
                IssueFix::CompatibleCopy => (Severity::Warning, vec![SuggestedFix::MakeCompatibleCopy { mod_id: id.clone() }], "; a compatible copy fixes it".to_string(), "copy_fixes"),
                IssueFix::None => (Severity::Info, vec![], String::new(), ""),
            };
            let (from, to) = match &issue.fix {
                IssueFix::RequirePath { from, to } => (from.as_str(), to.as_str()),
                _ => ("", ""),
            };
            list.push(severity, "community_known_issue", Some(id),
                format!("{} has a known issue, per the community index: {}{}{}{} [{}]", name(id), issue.title, evidence, note, how, seen_on(&issue.build, issue.status)),
                fixes);
            list.with(&[("mod", &name(id)), ("title", &issue.title), ("evidence", &evidence), ("note", &note), ("howKey", how_key), ("from", from), ("to", to),
                ("build", issue.build.as_deref().unwrap_or("")), ("seenKey", seen_key(&issue.build, issue.status))]);
        }
        if let Some(successor) = &registry.successor {
            let loadable = catalog.get(id).map(|m| matches!(m.loadability, Loadability::Loadable)).unwrap_or(true);
            if !loadable && !position.contains_key(successor.id.as_str()) {
                let fixes = successor.workshop_id.clone().map(|w| vec![SuggestedFix::OpenWorkshopPage { workshop_id: w }]).unwrap_or_default();
                list.push(Severity::Info, "community_successor", Some(id),
                    format!("{} has a version for this build, per the community index: {}{}", name(id), name(&successor.id), successor.note.clone().map(|n| format!(" ({})", n)).unwrap_or_default()),
                    fixes);
                list.with(&[("mod", &name(id)), ("successor", &name(&successor.id)), ("note", &successor.note.clone().map(|n| format!(" ({})", n)).unwrap_or_default())]);
            }
        }
    }
    for issue in list.0 {
        match issue.severity {
            Severity::Error => report.errors += 1,
            Severity::Warning => report.warnings += 1,
            Severity::Info => {}
        }
        report.issues.push(issue);
    }
}

/// The installed game as Steam's app manifest describes it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameInstall {
    /// "stable" when no beta is opted into, else the beta's name ("unstable", "legacy41", "b41multiplayer"...).
    pub branch: String,
    /// Steam's build id for what is on disk.
    pub build_id: Option<String>,
    /// When Steam last updated the game, as a date.
    pub last_updated: Option<String>,
}

/// What `appmanifest_108600.acf` says. Steam writes the beta under UserConfig (chosen) and
/// MountedConfig (what is on disk); either names it, and neither is written on the default
/// branch.
pub fn game_install_from_manifest(text: &str) -> GameInstall {
    let vdf = crate::vdf::parse(text);
    let state = vdf.get("AppState").unwrap_or(&vdf);
    let beta = ["MountedConfig", "UserConfig"]
        .iter()
        .filter_map(|k| state.get(k))
        .filter_map(|cfg| cfg.str("BetaKey"))
        .map(str::trim)
        .find(|b| !b.is_empty() && !b.eq_ignore_ascii_case("public"));
    GameInstall {
        branch: beta.map(str::to_string).unwrap_or_else(|| "stable".to_string()),
        build_id: state.str("buildid").map(|s| s.trim().to_string()),
        last_updated: state
            .u64("LastUpdated")
            .and_then(|t| chrono::DateTime::from_timestamp(t as i64, 0))
            .map(|d| d.format("%Y-%m-%d").to_string()),
    }
}

/// The manifest beside the game: `<library>/steamapps/common/ProjectZomboid` ->
/// `<library>/steamapps/appmanifest_108600.acf`. None when the game is not a Steam install.
pub fn read_game_install(pz_install_dir: &str) -> Option<GameInstall> {
    let manifest = std::path::Path::new(pz_install_dir).parent()?.parent()?.join("appmanifest_108600.acf");
    let text = std::fs::read_to_string(manifest).ok()?;
    Some(game_install_from_manifest(&text))
}

#[cfg(test)]
mod game_install_tests {
    use super::game_install_from_manifest;

    #[test]
    fn the_branch_is_read_from_the_manifest_and_stable_when_none_is_named() {
        let stable = "\"AppState\"\n{\n\t\"appid\"\t\t\"108600\"\n\t\"buildid\"\t\t\"25485521\"\n\t\"LastUpdated\"\t\t\"1790603391\"\n}\n";
        let got = game_install_from_manifest(stable);
        assert_eq!(got.branch, "stable");
        assert_eq!(got.build_id.as_deref(), Some("25485521"));
        assert_eq!(got.last_updated.as_deref(), Some("2026-09-28"));

        let beta = "\"AppState\"\n{\n\t\"buildid\"\t\t\"1\"\n\t\"UserConfig\"\n\t{\n\t\t\"BetaKey\"\t\t\"unstable\"\n\t}\n}\n";
        assert_eq!(game_install_from_manifest(beta).branch, "unstable");
    }
}

/// A default.txt entry may be `ModId` or `<workshopId>/<ModId>` (newer B42 builds use the second
/// form to tell apart mods that share an id).
fn resolve_entry<'a>(catalog: &'a ModCatalog, entry: &str) -> Option<&'a CatalogMod> {
    // Some mods put a slash in their own id (e.g. "2858201402/dailystatisticsandgains").
    if let Some(exact) = catalog.get(entry) {
        return Some(exact);
    }
    match entry.split_once('/') {
        Some((workshop_id, id)) if workshop_id.chars().all(|c| c.is_ascii_digit()) => catalog
            .mods
            .iter()
            .find(|m| m.id == id && m.source == ModSource::Workshop { workshop_id: workshop_id.to_string() })
            .or_else(|| catalog.get(id)),
        _ => catalog.get(entry),
    }
}

fn entry_mod_id(entry: &str) -> &str {
    match entry.split_once('/') {
        Some((ws, id)) if ws.chars().all(|c| c.is_ascii_digit()) => id,
        _ => entry,
    }
}

fn describe(m: &CatalogMod) -> String {
    if m.name == m.id {
        m.id.clone()
    } else {
        format!("{} ({})", m.name, m.id)
    }
}

fn near_match<'a>(catalog: &'a ModCatalog, id: &str) -> Option<&'a CatalogMod> {
    catalog.mods.iter().find(|m| m.shadowed_by.is_none() && m.id.eq_ignore_ascii_case(id))
}

/// Everything that will go wrong when the game loads `active` (the default.txt list, in order).
pub fn health_report(catalog: &ModCatalog, active: &[String]) -> HealthReport {
    let mut list = IssueList(Vec::new());

    let active_ids: Vec<&str> = active
        .iter()
        .map(|e| {
            let e = e.trim();
            resolve_entry(catalog, e).map(|m| m.id.as_str()).unwrap_or_else(|| entry_mod_id(e))
        })
        .collect();
    let position: HashMap<&str, usize> = active_ids.iter().enumerate().rev().map(|(i, id)| (*id, i)).collect();

    // Availability like ChooseGameInfo.isAvailable (self + required mods, at any depth).
    // Why: the mod at fault and whether it is missing (`not_installed`) or not for this build
    // (`cannot_load`).
    let mut unavailable_memo: HashMap<String, Option<(&'static str, String)>> = HashMap::new();
    fn why_unavailable(
        catalog: &ModCatalog,
        id: &str,
        seen: &mut HashSet<String>,
        memo: &mut HashMap<String, Option<(&'static str, String)>>,
    ) -> Option<(&'static str, String)> {
        if let Some(r) = memo.get(id) {
            return r.clone();
        }
        if !seen.insert(id.to_string()) {
            return None; // cycle: the game stops following it as well
        }
        let reason = match catalog.get(id) {
            None => Some(("not_installed", id.to_string())),
            Some(m) if m.loadability != Loadability::Loadable => Some(("cannot_load", describe(m))),
            Some(m) => m.info.require.iter().find_map(|r| why_unavailable(catalog, r, seen, memo)),
        };
        memo.insert(id.to_string(), reason.clone());
        reason
    }

    let mut updated_since_play: Vec<String> = Vec::new();
    let mut seen_entries: HashSet<&str> = HashSet::new();
    for (idx, entry) in active.iter().enumerate() {
        let entry = entry.trim();
        let id = active_ids[idx];
        if !seen_entries.insert(id) {
            list.push(Severity::Warning, "duplicate_entry", Some(id), format!("{} is in the active list more than once", id),
                vec![SuggestedFix::RemoveFromList { entry: entry.to_string() }]);
            list.with(&[("mod", id)]);
            continue;
        }
        let Some(m) = resolve_entry(catalog, entry) else {
            let mut fixes = vec![SuggestedFix::RemoveFromList { entry: entry.to_string() }];
            let (message, params) = match near_match(catalog, id) {
                Some(similar) => {
                    fixes.insert(0, SuggestedFix::EnableMod { mod_id: similar.id.clone() });
                    (format!("{} is not installed. Mod ids are case-sensitive: did you mean {}?", id, similar.id),
                        issue_params(&[("variant", "similar"), ("mod", id), ("similar", &similar.id)]))
                }
                None => {
                    fixes.push(SuggestedFix::SubscribeWorkshop { hint: id.to_string() });
                    (format!("{} is enabled but not installed (unsubscribed or deleted)", id), issue_params(&[("variant", "gone"), ("mod", id)]))
                }
            };
            list.push(Severity::Error, "missing_mod", Some(id), message, fixes);
            list.0.last_mut().unwrap().params = params;
            continue;
        };

        // Mods from Workshop folders can get an adapted copy in Zomboid/mods.
        let compat_fix = (m.source != ModSource::Local).then(|| SuggestedFix::MakeCompatibleCopy { mod_id: m.id.clone() });
        let name = describe(m);
        let build = catalog.game_version.as_deref().unwrap_or("?");
        match &m.loadability {
            Loadability::Loadable => {}
            Loadability::NoFolderForBuild { available_folders } => {
                list.push(
                Severity::Error, "not_for_build", Some(id),
                format!("{} has no files for game build {} (it has: {}). The game will not load it.",
                    describe(m), catalog.game_version.as_deref().unwrap_or("?"),
                    if available_folders.is_empty() { "nothing usable".to_string() } else { available_folders.join(", ") }),
                compat_fix.clone().into_iter().chain([SuggestedFix::DisableMod { mod_id: m.id.clone() }]).collect(),
                );
                let folders = available_folders.join(", ");
                list.with(&[("variant", if available_folders.is_empty() { "none" } else { "some" }), ("mod", &name), ("build", build), ("folders", &folders)]);
            }
            Loadability::OutsideVersionRange { version_min, version_max } => {
                list.push(
                Severity::Error, "outside_version_range", Some(id),
                format!("{} supports game versions {} to {}; you have {}. The game will not load it.",
                    describe(m), version_min.as_deref().unwrap_or("any"), version_max.as_deref().unwrap_or("any"),
                    catalog.game_version.as_deref().unwrap_or("?")),
                compat_fix.clone().into_iter().chain([SuggestedFix::DisableMod { mod_id: m.id.clone() }]).collect(),
                );
                list.with(&[
                    ("mod", &name), ("build", build),
                    ("min", version_min.as_deref().unwrap_or("")), ("minKey", if version_min.is_none() { "any" } else { "" }),
                    ("max", version_max.as_deref().unwrap_or("")), ("maxKey", if version_max.is_none() { "any" } else { "" }),
                ]);
            }
            Loadability::InvalidModInfo { reason } => {
                list.push(
                Severity::Error, "invalid_mod_info", Some(id),
                format!("{}: {}", describe(m), reason),
                compat_fix.into_iter().chain([SuggestedFix::DisableMod { mod_id: m.id.clone() }]).collect(),
                );
                list.with(&[("mod", &name), ("reason", reason)]);
            }
        }

        for req in &m.info.require {
            match catalog.get(req) {
                None => {
                    let similar = near_match(catalog, req);
                    let message = match similar {
                        Some(similar) => format!("{} requires {}, which is not installed (an installed mod is called {}: ids are case-sensitive)", describe(m), req, similar.id),
                        None => format!("{} requires {}, which is not installed", describe(m), req),
                    };
                    list.push(Severity::Error, "missing_dependency", Some(id), message,
                        vec![SuggestedFix::SubscribeWorkshop { hint: req.clone() }, SuggestedFix::DisableMod { mod_id: m.id.clone() }]);
                    match similar {
                        Some(similar) => list.with(&[("variant", "similar"), ("mod", &name), ("dep", req), ("similar", &similar.id)]),
                        None => list.with(&[("variant", "plain"), ("mod", &name), ("dep", req)]),
                    }
                }
                Some(dep) => match position.get(req.as_str()) {
                    None => {
                        list.push(Severity::Error, "dependency_not_enabled", Some(id),
                        format!("{} requires {}, which is installed but not enabled", describe(m), describe(dep)),
                        vec![
                            SuggestedFix::EnableMod { mod_id: dep.id.clone() },
                            // ...and place it before the mod that needs it.
                            SuggestedFix::MoveBefore { mod_id: dep.id.clone(), before: m.id.clone() },
                        ]);
                        list.with(&[("mod", &name), ("dep", &describe(dep))]);
                    }
                    Some(&dep_pos) if dep_pos > idx => {
                        list.push(Severity::Warning, "dependency_after", Some(id),
                        format!("{} requires {}, but {} loads after it", describe(m), dep.id, dep.id),
                        vec![SuggestedFix::MoveBefore { mod_id: dep.id.clone(), before: m.id.clone() }]);
                        list.with(&[("mod", &name), ("dep", &dep.id)]);
                    }
                    Some(_) => {
                        if let Some((why, culprit)) = why_unavailable(catalog, req, &mut HashSet::new(), &mut unavailable_memo) {
                            let reason = if why == "not_installed" { format!("{} is not installed", culprit) } else { format!("{} cannot load on this build", culprit) };
                            list.push(Severity::Error, "dependency_unavailable", Some(id),
                                format!("{} requires {}, which is unavailable: {}", describe(m), dep.id, reason), vec![]);
                            list.with(&[("variant", why), ("mod", &name), ("dep", &dep.id), ("culprit", &culprit)]);
                            list.relate_last(&dep.id);
                        }
                    }
                },
            }
        }

        for after in &m.info.load_after {
            if let Some(&p) = position.get(after.as_str()) {
                if p > idx {
                    list.push(Severity::Warning, "load_after_order", Some(id),
                        format!("{} asks to load after {}, but {} comes later in the list", describe(m), after, after),
                        vec![SuggestedFix::MoveBefore { mod_id: after.clone(), before: m.id.clone() }]);
                    list.with(&[("mod", &name), ("other", after)]);
                }
            }
        }
        for before in &m.info.load_before {
            if let Some(&p) = position.get(before.as_str()) {
                if p < idx {
                    list.push(Severity::Warning, "load_before_order", Some(id),
                        format!("{} asks to load before {}, but {} comes earlier in the list", describe(m), before, before),
                        vec![SuggestedFix::MoveBefore { mod_id: m.id.clone(), before: before.clone() }]);
                    list.with(&[("mod", &name), ("other", before)]);
                }
            }
        }
        // Another mod manager that draws the game's Mods screen, on next to the Studio Link.
        if crate::studio_link::OTHER_MANAGERS.contains(&m.id.as_str()) && position.contains_key(crate::studio_link::LINK_MOD_ID) {
            list.push(Severity::Warning, "two_mod_managers", Some(id),
                format!("{} also draws the game's Mods screen. While it is on, the Studio leaves that screen to it (the save backups and the check before a save loads stay the Studio's). Turn it off in this profile to use the Studio's screen.", describe(m)),
                vec![SuggestedFix::DisableMod { mod_id: m.id.clone() }]);
            list.with(&[("mod", &name)]);
        }
        for other in &m.info.incompatible {
            if let Some(&p) = position.get(other.as_str()) {
                // Report each pair once, on the later mod.
                if p < idx || !catalog.get(other).is_some_and(|o| o.info.incompatible.contains(&m.id)) {
                    list.push(Severity::Error, "incompatible", Some(id),
                        format!("{} is marked incompatible with {}, and both are enabled", describe(m), other),
                        vec![SuggestedFix::DisableMod { mod_id: m.id.clone() }, SuggestedFix::DisableMod { mod_id: other.clone() }]);
                    list.with(&[("mod", &name), ("other", other)]);
                }
            }
        }

        if let (ModSource::Workshop { workshop_id }, Some(state)) = (&m.source, &m.workshop_state) {
            if state.update_pending {
                let when = state.latest_time.and_then(|t| chrono::DateTime::from_timestamp(t as i64, 0)).map(|d| d.format("%Y-%m-%d").to_string());
                list.push(Severity::Warning, "workshop_update_pending", Some(id),
                    format!("Steam has a newer version of {}{} that is not downloaded yet. Let Steam download it (restart Steam if it is stuck): servers reject players whose mod versions differ (\"Workshop item version is different than the server's\").",
                        describe(m), when.as_ref().map(|w| format!(" (published {})", w)).unwrap_or_default()),
                    vec![SuggestedFix::KeepVersion { mod_id: m.id.clone() }, SuggestedFix::OpenWorkshopPage { workshop_id: workshop_id.clone() }]);
                list.with(&[("mod", &name), ("date", when.as_deref().unwrap_or("")), ("publishedKey", if when.is_some() { "published" } else { "" })]);
            }
            if state.updated_since_last_play {
                updated_since_play.push(describe(m));
            }
        }

        let compat = matches!(&m.origin, Some(crate::origins::ModOrigin::Compat { .. }));
        if let Some(crate::origins::ModOrigin::Compat { changes, .. }) = &m.origin {
            let folder = std::path::Path::new(&m.root_dir).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
            list.push(Severity::Info, "compatible_copy", Some(id),
                format!("{} runs from a compatible copy the Studio made in Zomboid/mods ({}); the Workshop original is ignored meanwhile. It is used when the game is started from PZ Mod Studio; starting it from Steam needs the launch option -modfolders mods,workshop,steam. Nothing to do: once the author updates the mod for Build 42, \"Back to the original\" removes the copy.",
                    describe(m), changes.join(" ")),
                vec![SuggestedFix::RemoveCompatibleCopy { folder }]);
            list.with(&[("mod", &name), ("changes", &changes.join(" "))]);
        }

        // A compatible copy shadows the Workshop original on purpose: its own row says so.
        for copy in catalog.mods.iter().filter(|c| !compat && c.id == m.id && c.shadowed_by.is_some()) {
            list.push(Severity::Info, "shadowed_copy", Some(id),
                format!("Another copy of {} at {} is ignored: the game uses the one at {}", m.id, copy.root_dir, m.root_dir), vec![]);
            list.with(&[("mod", &m.id), ("ignored", &copy.root_dir), ("used", &m.root_dir)]);
        }
    }

    if !updated_since_play.is_empty() {
        let shown: Vec<&str> = updated_since_play.iter().take(12).map(String::as_str).collect();
        let more = updated_since_play.len().saturating_sub(shown.len());
        list.push(Severity::Info, "updated_since_last_play", None,
            format!("{} active mods were updated since you last played: {}{}. If a save misbehaves, start with these.",
                updated_since_play.len(), shown.join(", "), if more > 0 { format!(" and {} more", more) } else { String::new() }),
            vec![]);
        list.with(&[("n", &updated_since_play.len().to_string()), ("mods", &shown.join(", ")), ("more", &more.to_string()), ("andMoreKey", if more > 0 { "and_more" } else { "" })]);
    }

    let mut issues = list.0;
    issues.sort_by(|a, b| a.severity.cmp(&b.severity));
    let errors = issues.iter().filter(|i| i.severity == Severity::Error).count();
    let warnings = issues.iter().filter(|i| i.severity == Severity::Warning).count();
    HealthReport { game_version: catalog.game_version.clone(), install: None, community: None, active_count: active.len(), errors, warnings, issues }
}

// ---------------------------------------------------------------------------------------------
// Load-order sort
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortResult {
    /// The new order: every input entry exactly once.
    pub order: Vec<String>,
    /// Entries whose position changed.
    pub moved: Vec<String>,
    /// Entries caught in a require/loadModAfter/loadModBefore cycle (kept in their old order).
    pub cycle: Vec<String>,
}

/// Studio merge packages go last so they override the mods they merge.
fn is_studio_package(id: &str) -> bool {
    id.starts_with("Z_PZModStudio_") && id != "Z_PZModStudio_Bridge"
}

/// Sorts `order` (the full list, enabled and disabled) so every mod comes after what it
/// requires and its loadModAfter mods, and before its loadModBefore mods, using the
/// mod.info data the game reads. Everything else keeps the user's order: among mods that are
/// free to go next, the one that was earliest goes first, so an already valid list is unchanged.
pub fn sort_load_order(catalog: &ModCatalog, order: &[String]) -> SortResult {
    let ids: Vec<String> = order
        .iter()
        .map(|e| resolve_entry(catalog, e.trim()).map(|m| m.id.clone()).unwrap_or_else(|| e.trim().to_string()))
        .collect();
    // The Studio's profiler must load before every mod it measures; the packages go last.
    let (first, rest): (Vec<usize>, Vec<usize>) = (0..ids.len()).partition(|&i| ids[i] == crate::sandbox::profiler::PROFILER_ID);
    let (regular, packages): (Vec<usize>, Vec<usize>) = rest.into_iter().partition(|&i| !is_studio_package(&ids[i]));
    let index_of: HashMap<&str, usize> = regular.iter().rev().map(|&i| (ids[i].as_str(), i)).collect();

    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); ids.len()];
    let mut in_degree = vec![0usize; ids.len()];
    let mut edges: HashSet<(usize, usize)> = HashSet::new();
    let mut add_edge = |before: usize, after: usize| {
        if before != after && edges.insert((before, after)) {
            successors[before].push(after);
            in_degree[after] += 1;
        }
    };
    for &i in &regular {
        if index_of.get(ids[i].as_str()) != Some(&i) {
            continue; // duplicate entry: only the first one carries constraints
        }
        let Some(m) = catalog.get(&ids[i]) else { continue };
        for dep in m.info.require.iter().chain(&m.info.load_after) {
            if let Some(&d) = index_of.get(dep.as_str()) {
                add_edge(d, i);
            }
        }
        for later in &m.info.load_before {
            if let Some(&l) = index_of.get(later.as_str()) {
                add_edge(i, l);
            }
        }
    }

    let mut ready: std::collections::BTreeSet<usize> = regular.iter().copied().filter(|&i| in_degree[i] == 0).collect();
    let mut placed = vec![false; ids.len()];
    let mut sorted: Vec<usize> = first;
    while let Some(i) = ready.pop_first() {
        placed[i] = true;
        sorted.push(i);
        for &n in &successors[i] {
            in_degree[n] -= 1;
            if in_degree[n] == 0 {
                ready.insert(n);
            }
        }
    }
    let cycle: Vec<usize> = regular.iter().copied().filter(|&i| !placed[i]).collect();
    sorted.extend(&cycle);
    sorted.extend(&packages);

    let new_order: Vec<String> = sorted.iter().map(|&i| order[i].clone()).collect();
    let moved = sorted.iter().enumerate().filter(|(pos, &i)| *pos != i).map(|(_, &i)| order[i].clone()).collect();
    SortResult { order: new_order, moved, cycle: cycle.iter().map(|&i| order[i].clone()).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        root: PathBuf,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// A fake Zomboid folder + Workshop folder under the temp dir.
    fn fixture(tag: &str) -> Fixture {
        let root = std::env::temp_dir().join(format!("pzms-catalog-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Zomboid/mods")).unwrap();
        fs::create_dir_all(root.join("workshop")).unwrap();
        Fixture { root }
    }

    fn write_mod(dir: &Path, info: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("mod.info"), info).unwrap();
    }

    fn paths(f: &Fixture) -> (String, String) {
        (f.root.join("workshop").to_string_lossy().into(), f.root.join("Zomboid").to_string_lossy().into())
    }

    fn codes(report: &HealthReport) -> Vec<(String, String)> {
        report.issues.iter().map(|i| (i.code.clone(), i.mod_id.clone().unwrap_or_default())).collect()
    }

    #[test]
    fn catalog_follows_game_search_order_and_version_folders() {
        let f = fixture("order");
        let ws = f.root.join("workshop");
        write_mod(&ws.join("111/mods/Lib/42"), "id=Lib\nname=Library\n");
        write_mod(&ws.join("111/mods/Lib/42.30"), "id=Lib\nname=Too new\n");
        write_mod(&ws.join("222/mods/OldMod"), "id=OldMod\nname=B41 only\n");
        write_mod(&f.root.join("Zomboid/mods/Lib/42"), "id=Lib\nname=Local copy\n");
        let (w, z) = paths(&f);

        let cat = build_catalog(&w, &z, Some(42_021));
        assert_eq!(cat.game_version.as_deref(), Some("42.21"));
        let lib = cat.get("Lib").unwrap();
        assert_eq!(lib.name, "Library", "42.30 is newer than the build; Workshop copy wins over local");
        assert_eq!(lib.source, ModSource::Workshop { workshop_id: "111".into() });
        assert!(cat.mods.iter().any(|m| m.id == "Lib" && m.source == ModSource::Local && m.shadowed_by.is_some()));
        assert!(matches!(cat.get("OldMod").unwrap().loadability, Loadability::NoFolderForBuild { .. }));
        // Build 41 reads only the root.
        let b41 = build_catalog(&w, &z, Some(41_078));
        assert_eq!(b41.get("OldMod").unwrap().loadability, Loadability::Loadable);
    }

    #[test]
    fn health_report_finds_what_the_game_would_trip_on() {
        let f = fixture("health");
        let ws = f.root.join("workshop");
        write_mod(&ws.join("1/mods/Lib/42"), "id=Lib\n");
        write_mod(&ws.join("2/mods/UsesLib/42"), "id=UsesLib\nrequire=\\Lib\n");
        write_mod(&ws.join("3/mods/NeedsMissing/42"), "id=NeedsMissing\nrequire=\\Ghost,\\lib\n");
        write_mod(&ws.join("4/mods/Old/"), "id=Old\n");
        write_mod(&ws.join("5/mods/NeedsOld/42"), "id=NeedsOld\nrequire=Old\n");
        write_mod(&ws.join("6/mods/A/42"), "id=A\nincompatible=\\B\n");
        write_mod(&ws.join("7/mods/B/42"), "id=B\n");
        write_mod(&ws.join("8/mods/Ranged/42"), "id=Ranged\nversionMax=42.15\n");
        write_mod(&ws.join("9/mods/Broken/42"), "id=Broken\nversionMin=42\n");
        write_mod(&ws.join("10/mods/Late/42"), "id=Late\nloadModAfter=B\n");
        write_mod(&ws.join("11/mods/Slashy/42"), "id=11/slashy\n");
        let (w, z) = paths(&f);
        let cat = build_catalog(&w, &z, Some(42_021));

        let active: Vec<String> = ["UsesLib", "Lib", "NeedsMissing", "Old", "NeedsOld", "A", "Late", "B", "Ranged", "Broken", "Gone", "a", "A"]
            .iter().map(|s| s.to_string()).collect();
        let report = health_report(&cat, &active);
        let got = codes(&report);
        let has = |code: &str, id: &str| got.contains(&(code.to_string(), id.to_string()));

        assert!(has("dependency_after", "UsesLib"));
        assert!(has("missing_dependency", "NeedsMissing"), "Ghost and lowercase lib are not installed");
        assert!(has("not_for_build", "Old"));
        assert!(has("dependency_unavailable", "NeedsOld"));
        assert!(has("incompatible", "A"));
        assert!(!report.issues.iter().any(|i| i.code == "two_mod_managers"), "no manager here");
        assert!(has("load_after_order", "Late"));
        assert!(has("outside_version_range", "Ranged"));
        assert!(has("invalid_mod_info", "Broken"));
        assert!(has("missing_mod", "Gone"));
        assert!(has("missing_mod", "a"), "ids are case-sensitive");
        assert!(has("duplicate_entry", "A"));
        assert_eq!(report.issues.iter().filter(|i| i.code == "incompatible").count(), 1);
        let related = |code: &str, id: &str| {
            report.issues.iter().find(|i| i.code == code && i.mod_id.as_deref() == Some(id)).unwrap().related.clone()
        };
        assert_eq!(related("dependency_after", "UsesLib"), vec!["Lib"]);
        assert_eq!(related("dependency_unavailable", "NeedsOld"), vec!["Old"]);
        assert_eq!(related("incompatible", "A"), vec!["B"]);
        assert_eq!(related("load_after_order", "Late"), vec!["B"]);
        assert!(report.issues.windows(2).all(|w| w[0].severity <= w[1].severity), "errors first");

        // A clean list has no issues.
        let clean = health_report(&cat, &["Lib".to_string(), "UsesLib".to_string()]);
        assert!(clean.issues.is_empty(), "{:?}", clean.issues);
        // Workshop-qualified entries resolve to that item.
        let qualified = health_report(&cat, &["1/Lib".to_string()]);
        assert!(qualified.issues.is_empty(), "{:?}", qualified.issues);
        // ...and an id that itself contains a slash (real mod 2858201402) is matched exactly.
        let slashy = health_report(&cat, &["11/slashy".to_string()]);
        assert!(slashy.issues.is_empty(), "{:?}", slashy.issues);
    }

    #[test]
    fn a_mod_that_fills_the_console_is_reported() {
        let f = fixture("flood");
        let ws = f.root.join("workshop");
        write_mod(&ws.join("1/mods/Noisy/42"), "id=Noisy
name=Noisy Keys
");
        write_mod(&ws.join("2/mods/Quiet/42"), "id=Quiet
");
        write_mod(&ws.join("3/mods/OffNoisy/42"), "id=OffNoisy
name=Off Noisy
");
        let (w, z) = paths(&f);
        let cat = build_catalog(&w, &z, Some(42_021));
        let stat = |name: &str, lines: usize| crate::sandbox::ModErrorStat { mod_name: name.into(), exceptions: lines / 4, lines, ..Default::default() };
        let stats = vec![stat("Noisy Keys", 2970), stat("Quiet", 407), stat("Off Noisy", 1200)];
        let mut report = health_report(&cat, &["Noisy".to_string(), "Quiet".to_string()]);
        add_console_flood_issues(&mut report, &cat, &["Noisy".to_string(), "Quiet".to_string()], &stats);
        let floods: Vec<&HealthIssue> = report.issues.iter().filter(|i| i.code == "floods_console").collect();
        assert_eq!(floods.len(), 1, "only a mod that is on, and only past the threshold");
        assert_eq!(floods[0].mod_id.as_deref(), Some("Noisy"));
        assert_eq!(floods[0].params["lines"], "2970");
        assert_eq!(floods[0].params["share"], "64");
        assert!(matches!(floods[0].fixes[0], SuggestedFix::DisableMod { .. }));
    }

    #[test]
    fn a_compatible_copy_is_one_row_not_two() {
        let f = fixture("compat-row");
        write_mod(&f.root.join("workshop/1/mods/Old/42"), "id=Old\n");
        write_mod(&f.root.join("Zomboid/mods/Old/42"), "id=Old\n");
        let (w, z) = paths(&f);
        let mut cat = build_catalog(&w, &z, Some(42_021));
        let active = vec!["Old".to_string()];
        let codes = |cat: &ModCatalog| health_report(cat, &active).issues.iter().map(|i| i.code.clone()).collect::<Vec<_>>();
        // Two copies of a mod: the one ignored is worth a row.
        assert!(codes(&cat).contains(&"shadowed_copy".to_string()));
        // The copy in use is the Studio's compatible copy: its own row says the original is ignored.
        let used = cat.mods.iter_mut().find(|m| m.id == "Old" && m.shadowed_by.is_none()).unwrap();
        used.origin = Some(crate::origins::ModOrigin::Compat { workshop_id: Some("1".into()), source_dir: "x".into(), changes: vec!["fixed".into()] });
        let after = codes(&cat);
        assert!(after.contains(&"compatible_copy".to_string()));
        assert!(!after.contains(&"shadowed_copy".to_string()), "{:?}", after);
    }

    #[test]
    fn sort_moves_only_what_the_rules_require() {
        let f = fixture("sort");
        let ws = f.root.join("workshop");
        write_mod(&ws.join("1/mods/Lib/42"), "id=Lib\n");
        write_mod(&ws.join("2/mods/UsesLib/42"), "id=UsesLib\nrequire=\\Lib\n");
        write_mod(&ws.join("3/mods/Late/42"), "id=Late\nloadModAfter=Zed\n");
        write_mod(&ws.join("4/mods/Early/42"), "id=Early\nloadModBefore=Alpha\n");
        write_mod(&ws.join("5/mods/Alpha/42"), "id=Alpha\n");
        write_mod(&ws.join("6/mods/Zed/42"), "id=Zed\n");
        write_mod(&ws.join("7/mods/CycA/42"), "id=CycA\nrequire=CycB\n");
        write_mod(&ws.join("8/mods/CycB/42"), "id=CycB\nrequire=CycA\n");
        let (w, z) = paths(&f);
        let cat = build_catalog(&w, &z, Some(42_021));
        let list = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        let r = sort_load_order(&cat, &list(&["Z_PZModStudio_Pack", "UsesLib", "Alpha", "Late", "Lib", "Early", "Zed", "NotInstalled"]));
        // Lib moves up to just before UsesLib, Early before Alpha, Zed before Late; the rest keeps its order.
        assert_eq!(r.order, list(&["Lib", "UsesLib", "Early", "Alpha", "Zed", "Late", "NotInstalled", "Z_PZModStudio_Pack"]));
        let pos = |id: &str| r.order.iter().position(|x| x == id).unwrap();
        assert!(pos("Lib") < pos("UsesLib"));
        assert!(pos("Zed") < pos("Late"));
        assert!(pos("Early") < pos("Alpha"));
        assert_eq!(r.order.last().unwrap(), "Z_PZModStudio_Pack", "Studio packages go last");
        assert!(r.cycle.is_empty());

        // An already valid order is left alone.
        let valid = list(&["Lib", "UsesLib", "Zed", "Late", "Early", "Alpha"]);
        let again = sort_load_order(&cat, &valid);
        assert_eq!(again.order, valid);
        assert!(again.moved.is_empty());

        // Cycles are kept (not dropped) and reported.
        let c = sort_load_order(&cat, &list(&["CycA", "Lib", "CycB"]));
        assert_eq!(c.order.len(), 3);
        assert_eq!(c.cycle, list(&["CycA", "CycB"]));
    }

    /// Read-only report over the real install. Run with:
    /// PZMS_WORKSHOP_DIR=... PZMS_ZOMBOID_DIR=... cargo test real_health -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_health_report() {
        let (Ok(w), Ok(z)) = (std::env::var("PZMS_WORKSHOP_DIR"), std::env::var("PZMS_ZOMBOID_DIR")) else { return };
        let cat = build_catalog(&w, &z, None);
        let active = crate::load_order::ini_parser::read_mod_list_ini(&format!("{}/mods/default.txt", z)).unwrap().active_mods;
        let report = health_report(&cat, &active);
        println!("build {:?}: {} mods in catalog, {} active, {} errors, {} warnings",
            cat.game_version, cat.mods.len(), report.active_count, report.errors, report.warnings);
        for i in &report.issues {
            println!("  [{:?}] {}: {}", i.severity, i.code, i.message);
        }
        let mut by_source: std::collections::BTreeMap<String, Vec<String>> = Default::default();
        for m in cat.mods.iter().filter(|m| m.shadowed_by.is_none()) {
            let key = match &m.source { ModSource::Workshop { .. } => "workshop", ModSource::WorkshopUpload { .. } => "upload", ModSource::Local => "local" };
            by_source.entry(key.to_string()).or_default().push(m.id.clone());
        }
        for (k, v) in &by_source {
            println!("source {}: {} {:?}", k, v.len(), if v.len() <= 10 { v.clone() } else { vec![] });
        }
        let sorted = sort_load_order(&cat, &active);
        println!("sort: {} moved, {} in cycles: {:?}", sorted.moved.len(), sorted.cycle.len(), sorted.moved);
    }

    #[test]
    fn file_conflicts_become_health_issues() {
        let state = |rel: &str, state: &str, auto: bool| crate::vfs::FileConflictState {
            relative_path: rel.into(),
            mods: vec!["Mod A".into(), "Mod B".into()],
            mod_ids: vec!["A".into(), "B".into()],
            state: state.into(),
            auto_mergeable: auto,
            strategy: String::new(),
            collisions: if auto { 0 } else { 1 },
        };
        let mut report = HealthReport { game_version: None, install: None, community: None, active_count: 2, errors: 0, warnings: 0, issues: vec![] };
        add_file_conflict_issues(
            &mut report,
            &[state("a.lua", "pending", true), state("b.lua", "pending", false), state("c.lua", "merged", true), state("d.lua", "stale", true)],
        );
        assert_eq!(report.warnings, 3, "merged files are not reported");
        let a = report.issues.iter().find(|i| i.message.contains("a.lua")).unwrap();
        assert_eq!(a.code, "file_conflict");
        assert_eq!(a.mod_id.as_deref(), Some("B"));
        assert_eq!(a.related, vec!["A".to_string()]);
        assert!(matches!(&a.fixes[0], SuggestedFix::MergeFiles { relative_paths } if relative_paths == &vec!["a.lua".to_string()]));
        let b = report.issues.iter().find(|i| i.message.contains("b.lua")).unwrap();
        assert_eq!(b.fixes, vec![SuggestedFix::OpenInMerger { relative_path: "b.lua".into() }]);
        assert!(report.issues.iter().any(|i| i.code == "file_conflict_stale"));
    }
}

#[cfg(test)]
mod script_conflict_issue_tests {
    use super::*;
    use crate::scripts::ScriptDefConflict;

    fn conflict(def: &str, mods: [&str; 2]) -> ScriptDefConflict {
        ScriptDefConflict {
            definition: def.to_string(),
            kind: def.split_whitespace().next().unwrap_or_default().to_string(),
            module: "Base".to_string(),
            name: def.split('.').next_back().unwrap_or(def).to_string(),
            bodies: vec![String::new(), String::new()],
            mod_ids: mods.iter().map(|m| m.to_string()).collect(),
            mods: mods.iter().map(|m| m.to_string()).collect(),
            files: vec!["media/scripts/a.txt".into(), "media/scripts/b.txt".into()],
            absolute_files: vec!["a.txt".into(), "b.txt".into()],
            clashing: vec![crate::scripts::ClashingProperty {
                key: "StaticModel".to_string(),
                values: vec![Some("One".to_string()), Some("Other".to_string())],
            }],
        }
    }

    #[test]
    fn one_pair_of_mods_is_one_row_however_much_they_disagree_about() {
        let conflicts = vec![
            conflict("item Base.ModernTire1", ["Cars", "Carry"]),
            conflict("item Base.ModernTire2", ["Cars", "Carry"]),
            conflict("item Base.ModernTire3", ["Cars", "Carry"]),
            conflict("item Base.NormalTire1", ["Cars", "Carry"]),
            conflict("character_trait_definition Base.base:burglar", ["Details", "Lockpicking"]),
        ];
        let issues = issues_for_script_conflicts(&conflicts);
        assert_eq!(issues.len(), 2, "{:#?}", issues);
        assert!(issues[0].message.contains("4 things"), "{}", issues[0].message);
        assert!(issues[0].message.contains("StaticModel"), "{}", issues[0].message);
        // It no longer claims the loser's whole definition is thrown away.
        assert!(!issues[0].message.contains("is ignored"), "{}", issues[0].message);
        assert!(issues[0].message.contains("Base.ModernTire1"), "{}", issues[0].message);
        assert!(issues[0].message.contains("..."), "{}", issues[0].message);
        // The second pair is named in full, since it is about one thing.
        assert!(issues[1].message.contains("base:burglar"), "{}", issues[1].message);
        // Settling it in the package comes first; swapping the load order is the blunt way and
        // comes last. (These mock values cannot be merged into one, so there is no auto-merge.)
        assert!(matches!(issues[0].fixes[0], SuggestedFix::OpenInMerger { .. }), "{:?}", issues[0].fixes);
        assert!(matches!(issues[0].fixes[1], SuggestedFix::MoveBefore { .. }), "{:?}", issues[0].fixes);
        // File by file: each mod's file once, with the four things it declares, and the second
        // pair's files apart from the first's.
        let d = &issues[0].details;
        assert_eq!(d.len(), 2, "{:#?}", d);
        assert_eq!(d[0].mod_id, "Cars");
        assert_eq!(d[0].declares.len(), 4);
        assert_eq!(d[0].declares[0].definition, "item Base.ModernTire1");
        assert!(d[0].declares[0].merged_path.starts_with("media/scripts/PZModStudio_"));
        assert_eq!(d[0].declares[0].state, "pending");
        assert_eq!(issues[1].details.len(), 2);
    }
}

#[cfg(test)]
mod live_script_issue_tests {
    /// How the real list's script conflicts read in Mod Health.
    /// PZMS_PZ_DIR=... PZMS_WORKSHOP_DIR=... PZMS_ZOMBOID_DIR=...
    /// cargo test --release the_real_script_conflict_rows -- --ignored --nocapture
    #[test]
    #[ignore]
    fn the_real_script_conflict_rows() {
        let (Ok(pz), Ok(workshop), Ok(zomboid)) = (
            std::env::var("PZMS_PZ_DIR"),
            std::env::var("PZMS_WORKSHOP_DIR"),
            std::env::var("PZMS_ZOMBOID_DIR"),
        ) else {
            return;
        };
        let active = crate::load_order::ini_parser::read_mod_list_ini(&format!("{}/mods/default.txt", zomboid))
            .map(|d| d.active_mods)
            .unwrap_or_default();
        let paths = crate::vfs::StudioPaths {
            pz_install_dir: pz,
            workshop_dir: workshop.clone(),
            mod_list_ini_path: format!("{}/mods/default.txt", zomboid),
            user_zomboid_dir: zomboid.clone(),
            carrier_workshop_id: None,
            is_valid: true,
        };
        let issues = super::script_definition_issues(&paths, &active);
        println!("{} row(s) in Mod Health:", issues.len());
        for i in &issues {
            println!("  - {}", i.message);
            println!("    fixes: {}", i.fixes.len());
        }
    }
}

#[cfg(test)]
mod live_health_tests {
    /// The whole Mod Health for this machine, read-only. PZMS_ZOMBOID_DIR.
    #[test]
    #[ignore]
    fn the_real_mod_health() {
        if std::env::var("PZMS_ZOMBOID_DIR").is_err() { return; }
        let paths = crate::vfs::auto_detect_paths();
        let active = crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default();
        let health = super::mod_health_full(&paths, &active);
        for i in health.report.issues.iter().filter(|i| i.severity != super::Severity::Info) {
            println!("[{:?}] {} {:?}: {}", i.severity, i.code, i.mod_id, i.message.chars().take(160).collect::<String>());
        }
        println!("warnings {}, errors {}", health.report.warnings, health.report.errors);
    }
}