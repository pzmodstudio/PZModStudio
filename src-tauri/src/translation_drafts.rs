//! Translation drafts of a mod's own texts, for its author.
//!
//! A mod's `media/lua/shared/Translate/<LANG>/<Category>.json` files (Build 42; the Build 41
//! `<Category>_<LANG>.txt` tables are read too) are the texts the game shows for it. The author
//! writes one language, usually EN; this writes the other languages' files with the keys that
//! are missing there, translated by the same service that translates mod descriptions
//! (Settings > Translation: Google, or DeepL with a key), keeping every key the author already
//! wrote. The game's own placeholders (`%1`, `<LINE>`, `<SIZE:medium>`, `<RGB:1,0,0>`) are kept
//! out of the translator's hands, and a draft whose placeholders came back changed is listed
//! for review.
//!
//! What was written by machine is remembered in `Translate/PZModStudio_Drafts.json` (the text
//! as written), so a key the author edits since counts as theirs, and the status can say how
//! much of each language is still a draft.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The game's language folders and the code the translation services know them by. `STREW`
/// (a test language of the game) is not one.
const LANGUAGES: &[(&str, &str)] = &[
    ("AR", "ar"), ("CA", "ca"), ("CH", "zh-TW"), ("CN", "zh-CN"), ("CS", "cs"), ("DA", "da"), ("DE", "de"), ("EN", "en"), ("ES", "es"),
    ("ES_CL", "es"), ("ES_MX", "es"), ("FI", "fi"), ("FR", "fr"), ("HU", "hu"), ("ID", "id"), ("IT", "it"), ("JP", "ja"), ("KO", "ko"),
    ("NL", "nl"), ("NO", "no"), ("PL", "pl"), ("PT", "pt-PT"), ("PTBR", "pt-BR"), ("RO", "ro"), ("RU", "ru"), ("TH", "th"), ("TR", "tr"), ("UA", "uk"),
];

const SIDECAR: &str = "PZModStudio_Drafts.json";

/// One language of the mod, as it stands.
#[derive(Debug, Clone, Serialize)]
pub struct DraftLanguage {
    /// The game's folder name (`ES`, `PTBR`).
    pub lang: String,
    /// Keys of the source language this language has.
    pub present: usize,
    /// Keys of the source language this language lacks.
    pub missing: usize,
    /// Keys written by this tool and not edited since.
    pub machine: usize,
    /// Machine keys whose placeholders came back changed: to look at.
    pub review: usize,
    /// The translation services know the language.
    pub supported: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftStatus {
    /// The language the mod is written in: EN when it has it, else the first with files.
    pub source_lang: String,
    /// Keys of the source language, over all categories.
    pub keys: usize,
    pub categories: Vec<String>,
    /// Where the files are (`.../Translate`).
    pub folder: String,
    /// The source files are Build 41 `.txt` tables (the drafts are written the same way).
    pub format_b41: bool,
    pub languages: Vec<DraftLanguage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftFile {
    pub lang: String,
    pub category: String,
    pub path: String,
    /// Keys written now.
    pub keys: usize,
    /// Keys whose placeholders the service changed.
    pub review: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftReport {
    pub files: Vec<DraftFile>,
    /// "google", "deepl", "mymemory" or "cache".
    pub service: String,
    pub keys_written: usize,
    pub keys_kept: usize,
}

/// What was written by machine: language -> category -> key -> the text as written.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Sidecar {
    #[serde(default)]
    machine: BTreeMap<String, BTreeMap<String, BTreeMap<String, MachineKey>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MachineKey {
    text: String,
    #[serde(default)]
    review: bool,
    #[serde(default)]
    service: String,
}

/// One category's file of one language.
#[derive(Debug, Clone)]
struct CategoryFile {
    category: String,
    path: PathBuf,
    b41: bool,
    /// Key as written -> text, in file order.
    entries: Vec<(String, String)>,
}

fn service_code(lang: &str) -> Option<&'static str> {
    LANGUAGES.iter().find(|(l, _)| l.eq_ignore_ascii_case(lang)).map(|(_, c)| *c)
}

/// The mod's `Translate` folder: the newest media layer that has one.
fn translate_dir(root: &Path) -> Option<PathBuf> {
    // The version folder first, then common/, then the mod's root (a Build 41 layout).
    let mut layers = crate::load_order::mod_info::media_layers_for_build(root, None);
    layers.reverse();
    layers.push(root.to_path_buf());
    for layer in layers {
        let shared = layer.join("media").join("lua").join("shared");
        let Ok(dirs) = std::fs::read_dir(&shared) else { continue };
        if let Some(t) = dirs.flatten().map(|e| e.path()).find(|p| p.is_dir() && p.file_name().and_then(|n| n.to_str()).map(|n| n.eq_ignore_ascii_case("translate")).unwrap_or(false)) {
            return Some(t);
        }
    }
    None
}

/// The files of one language folder, by category.
fn files_of(dir: &Path, lang: &str) -> Vec<CategoryFile> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
    paths.sort();
    for path in paths {
        let stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let (category, b41, entries) = match ext.as_str() {
            "json" => (stem.clone(), false, crate::translations::parse_entries(&text)),
            "txt" => {
                // `<Category>_<LANG>.txt`; `language.txt` and the like are not tables.
                let Some(category) = stem.strip_suffix(&format!("_{}", lang)).map(str::to_string) else { continue };
                (category, true, crate::compat::parse_b41_translation(&text))
            }
            _ => continue,
        };
        if entries.is_empty() {
            continue;
        }
        out.push(CategoryFile { category, path, b41, entries });
    }
    out
}

fn read_sidecar(translate: &Path) -> Sidecar {
    std::fs::read_to_string(translate.join(SIDECAR)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn write_sidecar(translate: &Path, s: &Sidecar) -> Result<(), String> {
    let text = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(translate.join(SIDECAR), text).map_err(|e| format!("Could not write {}: {}", SIDECAR, e))
}

/// The language folders the mod has, upper-cased, sorted.
fn language_dirs(translate: &Path) -> Vec<String> {
    let mut langs: Vec<String> = std::fs::read_dir(translate)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_uppercase())
        .collect();
    langs.sort();
    langs.dedup();
    langs
}

/// The language the mod is written in and its files.
fn source(translate: &Path) -> Result<(String, Vec<CategoryFile>), String> {
    let langs = language_dirs(translate);
    let mut candidates: Vec<String> = Vec::new();
    if langs.iter().any(|l| l == "EN") {
        candidates.push("EN".to_string());
    }
    candidates.extend(langs.iter().filter(|l| *l != "EN").cloned());
    for lang in candidates {
        let files = files_of(&translate.join(&lang), &lang);
        if !files.is_empty() {
            return Ok((lang, files));
        }
    }
    Err("No translation files found: the mod has no media/lua/shared/Translate/<LANG>/<Category>.json with texts.".to_string())
}

/// Every language the game ships, with how much of the mod's source each one has.
pub fn status(root: &Path) -> Result<DraftStatus, String> {
    let translate = translate_dir(root).ok_or_else(|| "The mod has no media/lua/shared/Translate folder.".to_string())?;
    let (source_lang, source_files) = source(&translate)?;
    let sidecar = read_sidecar(&translate);
    let keys: usize = source_files.iter().map(|f| f.entries.len()).sum();
    let format_b41 = source_files.iter().all(|f| f.b41);
    let mut languages = Vec::new();
    for (lang, _) in LANGUAGES {
        if *lang == source_lang {
            continue;
        }
        let have = files_of(&translate.join(lang), lang);
        let machine_of = sidecar.machine.get(*lang);
        let (mut present, mut machine, mut review) = (0usize, 0usize, 0usize);
        for src in &source_files {
            let Some(target) = have.iter().find(|f| f.category == src.category) else { continue };
            let made = machine_of.and_then(|m| m.get(&src.category));
            for (key, _) in &src.entries {
                let Some((_, text)) = target.entries.iter().find(|(k, _)| k == key) else { continue };
                present += 1;
                if let Some(mk) = made.and_then(|m| m.get(key)) {
                    if mk.text == *text {
                        machine += 1;
                        if mk.review {
                            review += 1;
                        }
                    }
                }
            }
        }
        languages.push(DraftLanguage { lang: lang.to_string(), present, missing: keys.saturating_sub(present), machine, review, supported: true });
    }
    // Folders of the mod the list above does not know (a custom language): shown, not offered.
    for lang in language_dirs(&translate) {
        if lang != source_lang && service_code(&lang).is_none() {
            let present: usize = files_of(&translate.join(&lang), &lang).iter().map(|f| f.entries.len()).sum();
            languages.push(DraftLanguage { lang, present, missing: keys.saturating_sub(present), machine: 0, review: 0, supported: false });
        }
    }
    Ok(DraftStatus {
        source_lang,
        keys,
        categories: source_files.iter().map(|f| f.category.clone()).collect(),
        folder: translate.to_string_lossy().to_string(),
        format_b41,
        languages,
    })
}

/// `<LINE>`, `<br>`, `<SIZE:medium>`, `<RGB:1,0,0>`: a letter after the `<`, no spaces inside.
fn is_tag(tag: &str) -> bool {
    let inner = &tag[1..tag.len() - 1];
    inner.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && inner.chars().all(|c| c.is_ascii_alphanumeric() || ":,._-/".contains(c))
}

/// The game's placeholders in a text, in order: `%1`..`%9` and `<...>` tags (`<LINE>`, `<br>`,
/// `<SIZE:medium>`, `<RGB:1,0,0>`, `<CENTRE>`...).
fn placeholders(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            out.push(format!("%{}", chars[i + 1]));
            i += 2;
            continue;
        }
        if chars[i] == '<' {
            if let Some(end) = chars[i..].iter().position(|&c| c == '>').filter(|&e| e < 40) {
                let tag: String = chars[i..=i + end].iter().collect();
                if is_tag(&tag) {
                    out.push(tag);
                    i += end + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// `%1` as `{1}` for the service (a percent sign gets moved or dropped; braces are left alone),
/// and back.
fn mask_percent(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            out.push('{');
            out.push(chars[i + 1]);
            out.push('}');
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn unmask_percent(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' && i + 2 < chars.len() && chars[i + 1].is_ascii_digit() && chars[i + 2] == '}' {
            out.push('%');
            out.push(chars[i + 1]);
            i += 3;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// The text cut at its `<...>` tags: the runs between them are what gets translated, each on
/// its own, so a tag never reaches the service; `%N` stays inside its run, masked as `{N}`
/// (see `mask_percent`), and is checked after.
fn split_tags(text: &str) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut run = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            if let Some(end) = chars[i..].iter().position(|&c| c == '>').filter(|&e| e < 40) {
                let tag: String = chars[i..=i + end].iter().collect();
                if is_tag(&tag) {
                    if !run.is_empty() {
                        out.push((std::mem::take(&mut run), true));
                    }
                    out.push((tag, false));
                    i += end + 1;
                    continue;
                }
            }
        }
        run.push(chars[i]);
        i += 1;
    }
    if !run.is_empty() {
        out.push((run, true));
    }
    out
}

fn json_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

fn b41_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Writes `new` into the category file of `lang`: appended to what is there (the author's order
/// and text kept), or a new file.
fn write_category(path: &Path, b41: bool, category: &str, lang: &str, new: &[(String, String)]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Could not make {}: {}", parent.display(), e))?;
    }
    let existing = std::fs::read_to_string(path).ok();
    // The indentation the file already uses (the first indented line), else four spaces.
    let indent: String = existing
        .as_deref()
        .and_then(|t| t.lines().find(|l| l.starts_with(' ') || l.starts_with('\t')).map(|l| l.chars().take_while(|c| c.is_whitespace()).collect()))
        .unwrap_or_else(|| "    ".to_string());
    let text = if b41 {
        let lines: String = new.iter().map(|(k, v)| format!("{}{} = {},\n", indent, k, b41_string(v))).collect();
        match existing {
            Some(old) => {
                let cut = old.trim_end().rfind('}').ok_or_else(|| format!("{} does not end with }}", path.display()))?;
                let head = old[..cut].trim_end();
                let sep = if head.ends_with('{') || head.ends_with(',') { "\n" } else { ",\n" };
                format!("{}{}{}}}\n", head, sep, lines)
            }
            None => format!("{}_{} = {{\n{}}}\n", category, lang, lines),
        }
    } else {
        let lines: Vec<String> = new.iter().map(|(k, v)| format!("{}{}: {}", indent, json_string(k), json_string(v))).collect();
        match existing {
            Some(old) => {
                let cut = old.trim_end().rfind('}').ok_or_else(|| format!("{} does not end with }}", path.display()))?;
                let head = old[..cut].trim_end();
                let sep = if head.ends_with('{') { "\n" } else { ",\n" };
                format!("{}{}{}\n}}\n", head, sep, lines.join(",\n"))
            }
            None => format!("{{\n{}\n}}\n", lines.join(",\n")),
        }
    };
    std::fs::write(path, text).map_err(|e| format!("Could not write {}: {}", path.display(), e))
}

/// Writes the missing keys of each of `langs`, translated from the source language. Keys the
/// language already has are kept as they are.
pub fn write_drafts(user_zomboid_dir: &str, root: &Path, langs: &[String]) -> Result<DraftReport, String> {
    let translate = translate_dir(root).ok_or_else(|| "The mod has no media/lua/shared/Translate folder.".to_string())?;
    let (source_lang, source_files) = source(&translate)?;
    let mut sidecar = read_sidecar(&translate);
    let mut report = DraftReport { files: Vec::new(), service: "cache".to_string(), keys_written: 0, keys_kept: 0 };
    for lang in langs {
        let lang = lang.to_uppercase();
        if lang == source_lang {
            continue;
        }
        let code = service_code(&lang).ok_or_else(|| format!("{} is not a language the translation services know.", lang))?;
        let have = files_of(&translate.join(&lang), &lang);
        for src in &source_files {
            let target = have.iter().find(|f| f.category == src.category);
            let missing: Vec<&(String, String)> = src.entries.iter().filter(|(k, _)| !target.map(|t| t.entries.iter().any(|(tk, _)| tk == k)).unwrap_or(false)).collect();
            report.keys_kept += src.entries.len() - missing.len();
            if missing.is_empty() {
                continue;
            }
            // Every run of every text as one list, so the service is asked once per file.
            let mut layout: Vec<Vec<(String, bool)>> = Vec::new();
            let mut pieces: Vec<String> = Vec::new();
            for (_, text) in &missing {
                let parts = split_tags(text);
                for (p, send) in &parts {
                    if *send && p.chars().any(char::is_alphabetic) {
                        pieces.push(mask_percent(p));
                    }
                }
                layout.push(parts);
            }
            let translated = if pieces.is_empty() {
                crate::translate::Translation { texts: Vec::new(), source: String::new(), service: "cache".to_string() }
            } else {
                crate::translate::translate_texts_from(user_zomboid_dir, &pieces, service_code(&source_lang), code)?
            };
            if translated.service != "cache" {
                report.service = translated.service.clone();
            }
            let mut next = translated.texts.iter();
            let mut new: Vec<(String, String)> = Vec::new();
            let mut review: Vec<String> = Vec::new();
            for ((key, text), parts) in missing.iter().zip(layout) {
                let mut out = String::new();
                for (p, send) in parts {
                    if send && p.chars().any(char::is_alphabetic) {
                        out.push_str(&unmask_percent(next.next().map(String::as_str).unwrap_or(&p)));
                    } else {
                        out.push_str(&p);
                    }
                }
                let ok = placeholders(text) == placeholders(&out);
                if !ok {
                    review.push(key.clone());
                }
                sidecar.machine.entry(lang.clone()).or_default().entry(src.category.clone()).or_default().insert(
                    key.clone(),
                    MachineKey { text: out.clone(), review: !ok, service: translated.service.clone() },
                );
                new.push((key.clone(), out));
            }
            let b41 = src.b41;
            let path = match target {
                Some(t) => t.path.clone(),
                None if b41 => translate.join(&lang).join(format!("{}_{}.txt", src.category, lang)),
                None => translate.join(&lang).join(format!("{}.json", src.category)),
            };
            write_category(&path, b41, &src.category, &lang, &new)?;
            if b41 {
                // Build 41 reads the folder's charset from language.txt; without one it guesses.
                let lt = translate.join(&lang).join("language.txt");
                if !lt.exists() {
                    let _ = std::fs::write(&lt, format!("text = {},\ncharset = UTF-8,\n", lang));
                }
            }
            report.keys_written += new.len();
            report.files.push(DraftFile { lang: lang.clone(), category: src.category.clone(), path: path.to_string_lossy().to_string(), keys: new.len(), review });
        }
    }
    write_sidecar(&translate, &sidecar)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_and_tags_are_found_and_kept_apart_from_the_text() {
        assert_eq!(placeholders("Give %1 to %2 <LINE> now <RGB:1,0,0>!"), ["%1", "%2", "<LINE>", "<RGB:1,0,0>"].map(String::from).to_vec());
        assert!(placeholders("a < b and c > d").is_empty());
        let parts = split_tags("Hello <LINE> world <br>");
        assert_eq!(parts, vec![("Hello ".to_string(), true), ("<LINE>".to_string(), false), (" world ".to_string(), true), ("<br>".to_string(), false)]);
        assert_eq!(mask_percent("Give %1 to %2 (50%)"), "Give {1} to {2} (50%)");
        assert_eq!(unmask_percent("Dar {1} a {2} (50%)"), "Dar %1 a %2 (50%)");
    }

    #[test]
    fn a_category_file_is_appended_to_or_made() {
        let dir = std::env::temp_dir().join(format!("pzms-drafts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let json = dir.join("ES").join("IG_UI.json");
        write_category(&json, false, "IG_UI", "ES", &[("IG_UI_a".into(), "Hola \"x\"".into())]).unwrap();
        write_category(&json, false, "IG_UI", "ES", &[("IG_UI_b".into(), "Chau".into())]).unwrap();
        let entries = crate::translations::parse_entries(&std::fs::read_to_string(&json).unwrap());
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|(k, v)| k == "IG_UI_a" && v == "Hola \"x\""));
        let txt = dir.join("FR").join("IG_UI_FR.txt");
        write_category(&txt, true, "IG_UI", "FR", &[("IG_UI_a".into(), "Salut".into())]).unwrap();
        write_category(&txt, true, "IG_UI", "FR", &[("IG_UI_b".into(), "Au revoir".into())]).unwrap();
        let text = std::fs::read_to_string(&txt).unwrap();
        assert!(text.starts_with("IG_UI_FR = {"));
        assert_eq!(crate::compat::parse_b41_translation(&text).len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Live, with the network: `cargo test --lib -- --ignored live_write_drafts --nocapture`.
    /// Makes a small mod in the temp folder, drafts ES and FR, prints what was written.
    #[test]
    #[ignore]
    fn live_write_drafts() {
        let dir = std::env::temp_dir().join(format!("pzms-drafts-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let translate = dir.join("42").join("media").join("lua").join("shared").join("Translate");
        std::fs::create_dir_all(translate.join("EN")).unwrap();
        std::fs::create_dir_all(translate.join("ES")).unwrap();
        std::fs::write(
            translate.join("EN").join("UI.json"),
            "{\n  \"UI_studio_hello\": \"Hello, survivor\",\n  \"UI_studio_give\": \"Give %1 to %2\",\n  \"UI_studio_lines\": \"First line <LINE> Second line <RGB:1,0,0> in red\",\n  \"UI_studio_url\": \"See https://example.com/x\"\n}",
        )
        .unwrap();
        std::fs::write(translate.join("ES").join("UI.json"), "{\n  \"UI_studio_hello\": \"Hola, superviviente\"\n}").unwrap();
        let z = crate::vfs::studio_paths().user_zomboid_dir;
        let report = write_drafts(&z, &dir, &["ES".to_string(), "FR".to_string()]).unwrap();
        println!("{:#?}", report);
        for lang in ["ES", "FR"] {
            println!("--- {}\n{}", lang, std::fs::read_to_string(translate.join(lang).join("UI.json")).unwrap());
        }
        println!("--- sidecar\n{}", std::fs::read_to_string(translate.join(SIDECAR)).unwrap());
        let s = status(&dir).unwrap();
        println!("{:#?}", s.languages.iter().filter(|l| l.lang == "ES" || l.lang == "FR").collect::<Vec<_>>());
        assert_eq!(report.keys_written, 7);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_reads_the_source_language_and_counts_what_each_language_has() {
        let dir = std::env::temp_dir().join(format!("pzms-drafts-status-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let translate = dir.join("media").join("lua").join("shared").join("Translate");
        std::fs::create_dir_all(translate.join("EN")).unwrap();
        std::fs::create_dir_all(translate.join("ES")).unwrap();
        std::fs::write(translate.join("EN").join("IG_UI.json"), "{\n  \"IG_UI_a\": \"A\",\n  \"IG_UI_b\": \"B\"\n}").unwrap();
        std::fs::write(translate.join("ES").join("IG_UI.json"), "{\n  \"IG_UI_a\": \"A es\"\n}").unwrap();
        let s = status(&dir).unwrap();
        assert_eq!(s.source_lang, "EN");
        assert_eq!(s.keys, 2);
        let es = s.languages.iter().find(|l| l.lang == "ES").unwrap();
        assert_eq!((es.present, es.missing, es.machine), (1, 1, 0));
        let fr = s.languages.iter().find(|l| l.lang == "FR").unwrap();
        assert_eq!((fr.present, fr.missing), (0, 2));
        assert!(s.languages.iter().all(|l| l.lang != "EN"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
