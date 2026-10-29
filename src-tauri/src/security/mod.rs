//! What a mod could do beyond modding: programs it ships, Java it carries (Build 42 runs it with
//! the player's rights, through ZombieBuddy), and Lua that reaches outside the game. Heuristics,
//! said as such: a warning names the file and the thing found, never a verdict. The rules follow
//! pzmm's Lua scanner and PZ-ModGuard's bytecode checks (both MIT), adapted to what the game's
//! Lua can and cannot call.
//!
//! Each mod's scan is kept under `PZModStudio_Cache/security/` with a fingerprint of its files,
//! so Mod Health reads it again only when the mod changed or the rules did.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Bumped when a rule changes: older scans are made again.
const ENGINE: &str = "security-v3";
const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_JAR_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Info,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    /// A stable rule name (`lua_os_execute`, `java_exec`, `executable`...).
    pub rule: String,
    pub risk: Risk,
    /// The file, from the mod's folder on.
    pub file: String,
    pub line: Option<u32>,
    /// What was found, verbatim (an API name, a string, an extension).
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModScan {
    pub mod_id: String,
    pub risk: Risk,
    /// No file matched any rule.
    pub clean: bool,
    pub findings: Vec<Finding>,
    pub files_scanned: u32,
    pub fingerprint: String,
    pub engine: String,
    pub scanned_at: u64,
}

// ---------------------------------------------------------------- Lua

/// `(needle, rule, risk)`: the needle must start at a word boundary.
const LUA_RULES: &[(&str, &str, Risk)] = &[
    ("os.execute(", "lua_os_execute", Risk::High),
    ("io.popen(", "lua_io_popen", Risk::High),
    ("package.loadlib", "lua_loadlib", Risk::High),
    ("loadstring(", "lua_loadstring", Risk::Medium),
    ("loadfile(", "lua_loadfile", Risk::Medium),
    ("dofile(", "lua_dofile", Risk::Medium),
    ("socket.", "lua_socket", Risk::Medium),
    ("http.get(", "lua_http", Risk::Medium),
    ("http.post(", "lua_http", Risk::Medium),
    ("getUrlInputStream(", "lua_url", Risk::Medium),
];

fn at_word_start(text: &str, at: usize) -> bool {
    at == 0 || !text[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_')
}

fn line_of(text: &str, at: usize) -> u32 {
    text[..at].matches('\n').count() as u32 + 1
}

/// The rules a Lua file trips, one finding per rule and line.
pub fn scan_lua(rel: &str, text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let lower = text.to_ascii_lowercase();
    for (needle, rule, risk) in LUA_RULES {
        let needle_l = needle.to_ascii_lowercase();
        let mut from = 0;
        while let Some(i) = lower[from..].find(&needle_l) {
            let at = from + i;
            if at_word_start(&lower, at) && !in_comment(&lower, at) {
                out.push(Finding { rule: rule.to_string(), risk: *risk, file: rel.to_string(), line: Some(line_of(text, at)), detail: needle.trim_end_matches('(').to_string() });
            }
            from = at + needle_l.len();
        }
    }
    // require("http://..."): code fetched from the net
    let mut from = 0;
    while let Some(i) = lower[from..].find("require") {
        let at = from + i;
        let rest = &lower[at + 7..];
        let rest = rest.trim_start().trim_start_matches('(').trim_start();
        if (rest.starts_with("\"http://") || rest.starts_with("\"https://") || rest.starts_with("'http://") || rest.starts_with("'https://")) && at_word_start(&lower, at) {
            out.push(Finding { rule: "lua_remote_require".to_string(), risk: Risk::High, file: rel.to_string(), line: Some(line_of(text, at)), detail: "require of a URL".to_string() });
        }
        from = at + 7;
    }
    // reads or writes with a literal path that climbs out of the mod or Lua folder (`..` alone
    // is Lua's concatenation; only what is inside quotes on the call's line counts)
    for api in ["getfilewriter(", "getmodfilewriter(", "getfilereader(", "getmodfilereader("] {
        let mut from = 0;
        while let Some(i) = lower[from..].find(api) {
            let at = from + i;
            let args = &lower[at + api.len()..];
            let line_end = args.find('\n').unwrap_or(args.len());
            if quoted_literals(&args[..line_end]).iter().any(|lit| escapes_folder(lit)) {
                out.push(Finding { rule: "lua_path_escape".to_string(), risk: Risk::High, file: rel.to_string(), line: Some(line_of(text, at)), detail: format!("{} with a path outside the mod", api.trim_end_matches('('))});
            }
            from = at + api.len();
        }
    }
    // obfuscation: a long run of \ddd or \x escapes, or string.char beside loadstring
    let escapes = lower.matches("\\x").count() + lower.matches("\\1").count() + lower.matches("\\9").count();
    if escapes >= 200 {
        out.push(Finding { rule: "lua_obfuscated".to_string(), risk: Risk::Medium, file: rel.to_string(), line: None, detail: format!("{} escaped characters in strings", escapes) });
    }
    if lower.contains("string.char(") && lower.contains("loadstring(") {
        out.push(Finding { rule: "lua_obfuscated".to_string(), risk: Risk::Medium, file: rel.to_string(), line: None, detail: "string.char and loadstring in the same file".to_string() });
    }
    out
}

/// The string literals ("..." and '...') in a piece of Lua.
fn quoted_literals(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' || c == '\'' {
            let mut lit = String::new();
            let mut escaped = false;
            for d in chars.by_ref() {
                if escaped {
                    lit.push(d);
                    escaped = false;
                } else if d == '\\' {
                    escaped = true;
                } else if d == c {
                    break;
                } else {
                    lit.push(d);
                }
            }
            out.push(lit);
        }
    }
    out
}

/// A literal path that leaves the folder the game allows: `../x`, `..\x`, `C:\x`, `/etc/x`.
fn escapes_folder(lit: &str) -> bool {
    let l = lit.replace('\\', "/");
    l.starts_with("../") || l.contains("/../") || l.contains(":/") || l.starts_with('/')
}

/// Whether `at` sits after `--` on its line (a comment).
fn in_comment(lower: &str, at: usize) -> bool {
    let line_start = lower[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
    lower[line_start..at].contains("--")
}

// ---------------------------------------------------------------- files

/// Compiled programs: loaded or run as they are.
const BINARY_EXT: &[&str] = &["exe", "dll", "com", "scr", "msi", "so", "dylib"];
/// Scripts: inert in a mod folder until someone runs them by hand (build helpers, mostly).
const SCRIPT_EXT: &[&str] = &["ps1", "bat", "cmd", "vbs", "sh"];
const CODE_EXT: &[&str] = &["exe", "dll", "com", "scr", "msi", "so", "dylib", "ps1", "bat", "cmd", "vbs", "sh", "jar", "class", "lua"];

/// Native files known to belong to a framework: named, not alarmed about.
const KNOWN_NATIVE: &[(&str, &str)] = &[
    ("zbnative.dll", "ZombieBuddy's native agent"),
    ("libzbnative.so", "ZombieBuddy's native agent"),
    ("pz3dloader.dll", "PZ3D's native hook"),
    ("libpz3dloader.so", "PZ3D's native hook"),
];

const SUSPECT_NAME_TOKENS: &[&str] = &["inject", "backdoor", "payload", "keygen", "miner", "keylogger", "rootkit", "stealer"];

fn name_has_token(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    for token in SUSPECT_NAME_TOKENS {
        if let Some(i) = lower.find(token) {
            let before = lower[..i].chars().next_back().is_none_or(|c| !c.is_ascii_alphabetic());
            let after = lower[i + token.len()..].chars().next().is_none_or(|c| !c.is_ascii_alphabetic());
            if before && after {
                return Some(token);
            }
        }
    }
    None
}

// ---------------------------------------------------------------- Java

/// Libraries bundled whole by known frameworks: their classes are the library's, not the mod's.
const LIBRARY_PACKAGES: &[&str] = &[
    "net/bytebuddy/", "io/github/classgraph/", "nonapi/io/github/classgraph/", "org/bouncycastle/", "zb/org/bouncycastle/",
    "kotlin/", "kotlinx/", "org/javacord/", "com/google/", "org/apache/", "org/slf4j/", "org/objectweb/asm/", "org/lwjgl/",
    "com/sun/jna/", "okhttp3/", "okio/", "org/joml/", "org/json/", "com/fasterxml/", "org/yaml/", "io/netty/",
];

/// `(what the constant pool holds, rule, risk)`.
const JAVA_RULES: &[(&str, &str, Risk)] = &[
    ("java/lang/ProcessBuilder", "java_exec", Risk::High),
    ("java/lang/Runtime", "java_runtime", Risk::Medium),
    ("powershell", "java_shell", Risk::High),
    ("cmd.exe", "java_shell", Risk::High),
    ("/bin/sh", "java_shell", Risk::High),
    ("java/net/HttpURLConnection", "java_network", Risk::Medium),
    ("java/net/http/HttpClient", "java_network", Risk::Medium),
    ("java/net/Socket", "java_network", Risk::Medium),
    ("openConnection", "java_network", Risk::Medium),
    ("openStream", "java_network", Risk::Medium),
    ("java/net/URLClassLoader", "java_classloader", Risk::High),
    ("defineClass", "java_classloader", Risk::High),
    ("sun/misc/Unsafe", "java_unsafe", Risk::Medium),
    ("discord.com/api/webhooks", "java_webhook", Risk::High),
    ("loginusers.vdf", "java_credentials", Risk::High),
    ("ssfn", "java_credentials", Risk::High),
    ("Login Data", "java_credentials", Risk::High),
    ("Cookies", "java_credentials", Risk::Medium),
    ("APPDATA", "java_user_files", Risk::Low),
    ("user.home", "java_user_files", Risk::Info),
    ("java/awt/Robot", "java_input", Risk::High),
    ("java/io/FileOutputStream", "java_file_write", Risk::Info),
];

/// Classes of known frameworks that do these things on purpose (ZombieBuddy's approval window
/// and author lookups, Viewpoint's GPU probe and upscaler download): `(mod id, class prefix)`.
const KNOWN_FRAMEWORK_CLASSES: &[(&str, &str)] = &[
    ("ZombieBuddy", "me/zed_0xff/zombie_buddy/"),
    ("Viewpoint", "viewpoint/platform/GpuBusy"),
    ("Viewpoint", "viewpoint/platform/Collector"),
    ("Viewpoint", "viewpoint/upscale/NativeDownload"),
    ("Viewpoint", "viewpoint/platform/HotReload"),
];

fn known_framework_class(mod_id: &str, class: &str) -> bool {
    KNOWN_FRAMEWORK_CLASSES.iter().any(|(m, prefix)| *m == mod_id && class.starts_with(prefix))
}

/// The UTF8 strings of a class file's constant pool. None when it is not a class file.
pub fn class_strings(bytes: &[u8]) -> Option<Vec<String>> {
    if bytes.len() < 10 || bytes[..4] != [0xCA, 0xFE, 0xBA, 0xBE] {
        return None;
    }
    let count = u16::from_be_bytes([bytes[8], bytes[9]]) as usize;
    let mut i = 10;
    let mut out = Vec::new();
    let mut slot = 1;
    while slot < count {
        let tag = *bytes.get(i)?;
        i += 1;
        let size = match tag {
            1 => {
                let len = u16::from_be_bytes([*bytes.get(i)?, *bytes.get(i + 1)?]) as usize;
                let s = bytes.get(i + 2..i + 2 + len)?;
                out.push(String::from_utf8_lossy(s).to_string());
                2 + len
            }
            3 | 4 => 4,
            5 | 6 => {
                slot += 1;
                8
            }
            7 | 8 | 16 | 19 | 20 => 2,
            9 | 10 | 11 | 12 | 17 | 18 => 4,
            15 => 3,
            _ => return None,
        };
        i += size;
        slot += 1;
    }
    Some(out)
}

/// The rules one class trips (the class named by `rel`).
pub fn scan_class(rel: &str, strings: &[String]) -> Vec<Finding> {
    let has_runtime = strings.iter().any(|s| s == "java/lang/Runtime");
    let has_exec = strings.iter().any(|s| s == "exec");
    let mut out = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (needle, rule, risk) in JAVA_RULES {
        if strings.iter().any(|s| s.contains(needle)) {
            // Runtime alone is common (memory, processors); Runtime with exec is running programs.
            let (rule, risk) = if *needle == "java/lang/Runtime" && has_runtime && has_exec { ("java_exec", Risk::High) } else { (*rule, *risk) };
            if *needle == "java/lang/Runtime" && !has_exec {
                continue;
            }
            if seen.insert(needle) {
                out.push(Finding { rule: rule.to_string(), risk, file: rel.to_string(), line: None, detail: needle.to_string() });
            }
        }
    }
    out
}

fn is_library_class(name: &str) -> bool {
    LIBRARY_PACKAGES.iter().any(|p| name.starts_with(p))
}

/// Every class in a jar, scanned; bundled libraries and the mod's known framework classes
/// skipped. `rel` names the jar.
pub fn scan_jar(mod_id: &str, rel: &str, bytes: &[u8]) -> Vec<Finding> {
    let mut out = Vec::new();
    let Ok(mut zip) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else {
        return out;
    };
    for i in 0..zip.len() {
        let Ok(mut entry) = zip.by_index(i) else { continue };
        let name = entry.name().to_string();
        if !name.ends_with(".class") || is_library_class(&name) || known_framework_class(mod_id, &name) || entry.size() > 4 * 1024 * 1024 {
            continue;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        if entry.read_to_end(&mut buf).is_err() {
            continue;
        }
        if let Some(strings) = class_strings(&buf) {
            out.extend(scan_class(&format!("{}!{}", rel, name), &strings));
        }
    }
    out
}

// ---------------------------------------------------------------- a mod

pub(crate) fn fingerprint(root: &Path) -> (String, u32) {
    let mut files = 0u32;
    let mut bytes = 0u64;
    let mut newest = 0u64;
    for e in walkdir::WalkDir::new(root).into_iter().flatten().filter(|e| e.file_type().is_file()) {
        files += 1;
        if let Ok(m) = e.metadata() {
            bytes += m.len();
            if let Ok(t) = m.modified().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).map_err(|e| std::io::Error::other(e))) {
                newest = newest.max(t.as_secs());
            }
        }
    }
    (format!("{}-{}-{}", files, bytes, newest), files)
}

fn risk_of(findings: &[Finding]) -> Risk {
    findings.iter().map(|f| f.risk).max().unwrap_or(Risk::Info)
}

/// Scans every file under `root` now (no cache).
pub fn scan_tree(mod_id: &str, root: &Path) -> ModScan {
    let mut findings = Vec::new();
    let (fp, files) = fingerprint(root);
    for e in walkdir::WalkDir::new(root).into_iter().flatten().filter(|e| e.file_type().is_file()) {
        let path = e.path();
        let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let ext = path.extension().map(|x| x.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let size = e.metadata().map(|m| m.len()).unwrap_or(0);
        if CODE_EXT.contains(&ext.as_str()) {
            if let Some(token) = name_has_token(&name) {
                findings.push(Finding { rule: "suspect_name".to_string(), risk: Risk::Medium, file: rel.clone(), line: None, detail: token.to_string() });
            }
        }
        if BINARY_EXT.contains(&ext.as_str()) {
            let known = KNOWN_NATIVE.iter().find(|(n, _)| n.eq_ignore_ascii_case(&name)).map(|(_, what)| *what);
            findings.push(Finding {
                rule: if known.is_some() { "known_native".to_string() } else { "executable".to_string() },
                risk: if known.is_some() { Risk::Info } else { Risk::High },
                file: rel.clone(),
                line: None,
                detail: known.map(str::to_string).unwrap_or_else(|| format!(".{}", ext)),
            });
            continue;
        }
        if SCRIPT_EXT.contains(&ext.as_str()) {
            findings.push(Finding { rule: "script".to_string(), risk: Risk::Medium, file: rel.clone(), line: None, detail: format!(".{}", ext) });
            continue;
        }
        if ext == "jar" {
            if size <= MAX_JAR_BYTES {
                if let Ok(bytes) = std::fs::read(path) {
                    let found = scan_jar(mod_id, &rel, &bytes);
                    if found.is_empty() {
                        findings.push(Finding { rule: "java_code".to_string(), risk: Risk::Info, file: rel.clone(), line: None, detail: "Java code, nothing flagged".to_string() });
                    }
                    findings.extend(found);
                }
            }
            continue;
        }
        if ext == "class" && size <= 4 * 1024 * 1024 {
            if let Ok(bytes) = std::fs::read(path) {
                if let Some(strings) = class_strings(&bytes) {
                    findings.extend(scan_class(&rel, &strings));
                }
            }
            continue;
        }
        if ext == "lua" && size <= MAX_TEXT_BYTES {
            if let Ok(text) = std::fs::read_to_string(path) {
                findings.extend(scan_lua(&rel, &text));
            }
        }
    }
    findings.sort_by(|a, b| b.risk.cmp(&a.risk).then(a.file.cmp(&b.file)).then(a.line.cmp(&b.line)));
    ModScan {
        mod_id: mod_id.to_string(),
        risk: risk_of(&findings),
        clean: findings.iter().all(|f| f.risk == Risk::Info),
        findings,
        files_scanned: files,
        fingerprint: fp,
        engine: ENGINE.to_string(),
        scanned_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
    }
}

/// A mod id as a file name.
pub(crate) fn safe_name(mod_id: &str) -> String {
    mod_id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

fn cache_file(user_zomboid_dir: &Path, mod_id: &str) -> PathBuf {
    let safe = safe_name(mod_id);
    user_zomboid_dir.join("PZModStudio_Cache").join("security").join(format!("{}.json", safe))
}

/// The mod's scan, from the cache when its files and the rules are unchanged.
pub fn scan_mod(user_zomboid_dir: &Path, mod_id: &str, root: &Path) -> ModScan {
    let file = cache_file(user_zomboid_dir, mod_id);
    let (fp, _) = fingerprint(root);
    if let Some(kept) = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<ModScan>(&t).ok()) {
        if kept.fingerprint == fp && kept.engine == ENGINE {
            return kept;
        }
    }
    let scan = scan_tree(mod_id, root);
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string(&scan) {
        let _ = std::fs::write(&file, text);
    }
    scan
}

/// The Studio's own mods: the bridge, the agent, the Link, the profiler, the packages.
pub fn is_studio_mod(id: &str) -> bool {
    id.starts_with("Z_PZModStudio_") || id.starts_with("PZModStudio_") || id == "Z_ClaudeAgent" || id == "Z_StudioLink"
}

/// The scans of these mods (ids or list entries), the Studio's own mods left out.
pub fn scan_mods(user_zomboid_dir: &Path, catalog: &crate::catalog::ModCatalog, ids: &[String]) -> Vec<ModScan> {
    let mut seen = BTreeSet::new();
    ids.iter()
        .filter_map(|e| catalog.resolve(e.trim()))
        .filter(|m| !is_studio_mod(&m.id) && seen.insert(m.id.clone()))
        .map(|m| scan_mod(user_zomboid_dir, &m.id, Path::new(&m.root_dir)))
        .collect()
}

/// What a Java rule means, for a sentence: "run programs", "reach the internet".
pub fn capability(rule: &str) -> &'static str {
    match rule {
        "java_exec" => "run programs",
        "java_shell" => "run shell commands",
        "java_network" => "reach the internet",
        "java_classloader" => "load code at run time",
        "java_credentials" => "read Steam or browser credential files",
        "java_webhook" => "post to a Discord webhook",
        "java_input" => "control the mouse and keyboard",
        "java_user_files" => "read your user folders",
        "java_unsafe" => "use unsafe memory access",
        "java_file_write" => "write files",
        "lua_os_execute" | "lua_io_popen" => "run programs",
        "lua_loadlib" => "load native libraries",
        "lua_loadstring" | "lua_loadfile" | "lua_dofile" => "run code built at run time",
        "lua_remote_require" => "fetch code from the internet",
        "lua_socket" | "lua_http" | "lua_url" => "reach the internet",
        "lua_path_escape" => "read or write files outside the mod",
        "lua_obfuscated" => "hide what its code says",
        _ => "something unusual",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_rules_find_calls_but_not_comments_or_words_that_contain_them() {
        let text = "local x = os.execute(\"calc\")\n-- loadstring(\"x\") in a comment\nlocal y = myloadstring(1)\nlocal z = loadstring(s)\nrequire(\"https://x.y/z.lua\")\ngetFileWriter(\"../../x.txt\", true, false)";
        let f = scan_lua("a.lua", text);
        let rules: Vec<(&str, Option<u32>)> = f.iter().map(|x| (x.rule.as_str(), x.line)).collect();
        assert!(rules.contains(&("lua_os_execute", Some(1))));
        assert!(rules.contains(&("lua_loadstring", Some(4))), "{:?}", rules);
        assert!(!rules.contains(&("lua_loadstring", Some(2))) && !rules.contains(&("lua_loadstring", Some(3))));
        assert!(rules.contains(&("lua_remote_require", Some(5))));
        assert!(rules.contains(&("lua_path_escape", Some(6))));
        assert!(scan_lua("b.lua", "local t = getText(\"IGUI_x\")\nprint(os.time())").is_empty());
        // Lua's `..` is concatenation: a name built from pieces is not a path that escapes
        assert!(scan_lua("c.lua", "local f = getFileWriter(name .. \".json\", true, false)\nlocal g = getModFileReader(\"MyMod\", \"data/\" .. id, true)").is_empty());
        assert!(!scan_lua("d.lua", "getFileWriter(\"C:\\\\Users\\\\x.txt\", true, false)").is_empty());
    }

    /// A class file by hand: magic, version, a pool of UTF8 strings.
    fn class_with(strings: &[&str]) -> Vec<u8> {
        let mut b = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 61];
        b.extend(((strings.len() + 1) as u16).to_be_bytes());
        for s in strings {
            b.push(1);
            b.extend((s.len() as u16).to_be_bytes());
            b.extend(s.as_bytes());
        }
        b
    }

    #[test]
    fn a_class_that_runs_programs_or_reads_steam_files_is_flagged_and_a_plain_one_is_not() {
        let strings = class_strings(&class_with(&["java/lang/Runtime", "exec", "loginusers.vdf", "zombie/Foo"])).expect("a class");
        let f = scan_class("Foo.class", &strings);
        let rules: Vec<&str> = f.iter().map(|x| x.rule.as_str()).collect();
        assert!(rules.contains(&"java_exec") && rules.contains(&"java_credentials"), "{:?}", rules);
        let plain = class_strings(&class_with(&["java/lang/Runtime", "availableProcessors", "zombie/Bar"])).unwrap();
        assert!(scan_class("Bar.class", &plain).is_empty(), "Runtime without exec is memory and cores");
        assert!(class_strings(b"not a class").is_none());
        assert!(is_library_class("net/bytebuddy/agent/Foo.class") && !is_library_class("me/author/Foo.class"));
    }

    #[test]
    fn a_tree_is_scanned_and_the_worst_finding_sets_the_risk() {
        let dir = std::env::temp_dir().join(format!("pzms_security_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let lua = dir.join("42").join("media").join("lua").join("client");
        std::fs::create_dir_all(&lua).unwrap();
        std::fs::write(lua.join("ok.lua"), "print('hi')").unwrap();
        std::fs::write(lua.join("dyn.lua"), "local f = loadstring(data)").unwrap();
        std::fs::write(dir.join("zbNative.dll"), b"x").unwrap();
        std::fs::write(dir.join("helper.exe"), b"x").unwrap();
        std::fs::write(dir.join("build.ps1"), b"x").unwrap();
        std::fs::write(dir.join("Hat_Miner.xml"), b"x").unwrap();
        let scan = scan_tree("Test", &dir);
        assert_eq!(scan.risk, Risk::High);
        assert!(!scan.clean);
        let rules: Vec<&str> = scan.findings.iter().map(|f| f.rule.as_str()).collect();
        assert!(rules.contains(&"executable") && rules.contains(&"known_native") && rules.contains(&"lua_loadstring") && rules.contains(&"script"));
        assert!(!rules.contains(&"suspect_name"), "a miner's hat is not a miner: names count on code files only");
        assert_eq!(scan.files_scanned, 6);
        assert!(is_studio_mod("Z_ClaudeAgent") && !is_studio_mod("ZombieBuddy"));
        // the cache answers the second time
        let z = dir.join("z");
        std::fs::create_dir_all(&z).unwrap();
        let first = scan_mod(&z, "Test", &dir);
        let again = scan_mod(&z, "Test", &dir);
        assert_eq!(first.scanned_at, again.scanned_at);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
