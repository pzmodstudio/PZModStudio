//! What a mod's Java uses of the game that the installed build no longer has.
//!
//! A Java mod is compiled against one build's `projectzomboid.jar`. Its class files name every
//! class, method and field of the game they use, in their constant pool (JVMS 4.4: `Class`,
//! `Fieldref`, `Methodref`, `InterfaceMethodref`, `NameAndType`). When an update renames or
//! removes one, the mod still loads, and fails with `NoClassDefFoundError` / `NoSuchMethodError`
//! / `NoSuchFieldError` the moment that code runs: often while the game starts. Read here without
//! running anything: every reference to a class of a package the game's jar has is looked up in
//! the installed jar, a member through the class, its superclasses and interfaces; one whose
//! lookup leaves the game's jar (a JDK class, a library elsewhere on the classpath) is taken as
//! there. ZombieBuddy's `@Patch` targets are looked up too: a patch on a class or method the
//! game no longer has is skipped by ZombieBuddy, so what the mod meant to change does not change.
//!
//! The game's classes are read once per jar (size and time); each mod's result is kept in
//! `PZModStudio_Cache/java_links/` until its jar or the game's jar changes.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const ENGINE: &str = "java-links-v1";
const MAX_JAR_BYTES: u64 = 256 * 1024 * 1024;
const MAX_FINDINGS: usize = 60;

/// One thing of the game a mod's Java uses that the installed build does not have.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Broken {
    /// `class`, `method`, `field`, or `patch` (a ZombieBuddy patch's target).
    pub kind: String,
    /// The game's class (`zombie.characters.IsoPlayer`).
    pub class: String,
    /// The method or field name; empty for a missing class.
    pub member: String,
    /// The method's argument and return types as the jar names them; empty for fields.
    pub descriptor: String,
    /// The mod's class that uses it.
    pub used_by: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModLinks {
    pub mod_id: String,
    pub name: String,
    pub jar: Option<String>,
    /// What the build lacks, each once (at most MAX_FINDINGS).
    pub broken: Vec<Broken>,
    /// More than MAX_FINDINGS: how many in all.
    pub total: usize,
    /// How many references to the game were checked.
    pub checked: usize,
    /// Why nothing could be checked: `jar_missing`, `no_package`, `unreadable: ...`, `no_game_jar`.
    pub unknown: Option<String>,
    #[serde(default)]
    key: String,
}

// ------------------------------------------------------------------ reading a class file

#[derive(Debug, Default)]
struct ClassFile {
    name: String,
    super_name: Option<String>,
    interfaces: Vec<String>,
    /// name + descriptor
    methods: Vec<(String, String)>,
    fields: Vec<String>,
    /// Classes it names (CONSTANT_Class).
    class_refs: Vec<String>,
    /// (kind, owner, name, descriptor) of every Fieldref / Methodref / InterfaceMethodref.
    member_refs: Vec<(u8, String, String, String)>,
}

fn u16_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?]) as usize)
}

fn u32_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_be_bytes([*b.get(i)?, *b.get(i + 1)?, *b.get(i + 2)?, *b.get(i + 3)?]) as usize)
}

/// The parts of a class file this needs. `with_refs`: also what it uses (a mod's classes); the
/// game's need only what they declare.
fn parse(bytes: &[u8], with_refs: bool) -> Option<ClassFile> {
    if bytes.len() < 10 || bytes[..4] != [0xCA, 0xFE, 0xBA, 0xBE] {
        return None;
    }
    let count = u16_at(bytes, 8)?;
    let mut utf8: Vec<Option<String>> = vec![None; count];
    let mut classes: Vec<Option<usize>> = vec![None; count];
    let mut name_types: Vec<Option<(usize, usize)>> = vec![None; count];
    let mut refs: Vec<(u8, usize, usize)> = Vec::new();
    let mut i = 10;
    let mut slot = 1;
    while slot < count {
        let tag = *bytes.get(i)?;
        i += 1;
        let size = match tag {
            1 => {
                let len = u16_at(bytes, i)?;
                let s = bytes.get(i + 2..i + 2 + len)?;
                utf8[slot] = Some(String::from_utf8_lossy(s).to_string());
                2 + len
            }
            3 | 4 => 4,
            5 | 6 => {
                slot += 1;
                8
            }
            7 => {
                classes[slot] = Some(u16_at(bytes, i)?);
                2
            }
            8 | 16 | 19 | 20 => 2,
            9 | 10 | 11 => {
                refs.push((tag, u16_at(bytes, i)?, u16_at(bytes, i + 2)?));
                4
            }
            12 => {
                name_types[slot] = Some((u16_at(bytes, i)?, u16_at(bytes, i + 2)?));
                4
            }
            17 | 18 => 4,
            15 => 3,
            _ => return None,
        };
        i += size;
        slot += 1;
    }
    let s = |idx: usize| utf8.get(idx).cloned().flatten();
    let class_name = |idx: usize| classes.get(idx).cloned().flatten().and_then(s);
    let mut out = ClassFile { name: class_name(u16_at(bytes, i + 2)?)?, super_name: class_name(u16_at(bytes, i + 4)?), ..Default::default() };
    let ifaces = u16_at(bytes, i + 6)?;
    for k in 0..ifaces {
        if let Some(n) = class_name(u16_at(bytes, i + 8 + 2 * k)?) {
            out.interfaces.push(n);
        }
    }
    i += 8 + 2 * ifaces;
    // fields, then methods: access, name, descriptor, attributes
    for pass in 0..2 {
        let n = u16_at(bytes, i)?;
        i += 2;
        for _ in 0..n {
            let name = s(u16_at(bytes, i + 2)?).unwrap_or_default();
            let desc = s(u16_at(bytes, i + 4)?).unwrap_or_default();
            let attrs = u16_at(bytes, i + 6)?;
            i += 8;
            for _ in 0..attrs {
                let len = u32_at(bytes, i + 2)?;
                i += 6 + len;
            }
            if pass == 0 {
                out.fields.push(name);
            } else {
                out.methods.push((name, desc));
            }
        }
    }
    if with_refs {
        out.class_refs = classes.iter().flatten().filter_map(|&n| s(n)).collect();
        for (tag, class_idx, nt_idx) in refs {
            let (Some(owner), Some(Some((n, d)))) = (class_name(class_idx), name_types.get(nt_idx).cloned()) else { continue };
            out.member_refs.push((tag, owner, s(n).unwrap_or_default(), s(d).unwrap_or_default()));
        }
    }
    Some(out)
}

/// Each class file of a jar, read by `f` (classes over 16 MB are left out).
fn each_class(bytes: &[u8], mut f: impl FnMut(&[u8])) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("not a jar: {}", e))?;
    for idx in 0..zip.len() {
        let Ok(mut entry) = zip.by_index(idx) else { continue };
        if !entry.name().ends_with(".class") || entry.size() > 16 * 1024 * 1024 {
            continue;
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        if entry.read_to_end(&mut data).is_ok() {
            f(&data);
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ the game's classes

#[derive(Debug, Default)]
pub struct GameClasses {
    classes: HashMap<String, GameClass>,
    /// The packages the jar has classes in (`zombie/characters`).
    packages: HashSet<String>,
}

#[derive(Debug, Default)]
struct GameClass {
    super_name: Option<String>,
    interfaces: Vec<String>,
    methods: HashSet<(String, String)>,
    fields: HashSet<String>,
}

/// What every class has from java.lang.Object.
const OBJECT_METHODS: [&str; 9] = ["equals", "hashCode", "toString", "getClass", "notify", "notifyAll", "wait", "clone", "finalize"];

fn package_of(internal: &str) -> &str {
    internal.rsplit_once('/').map(|(p, _)| p).unwrap_or("")
}

enum Found {
    Yes,
    No,
    /// The lookup left the game's jar (a JDK class, another library): not known here.
    Unknown,
}

impl GameClasses {
    pub fn has_class(&self, internal: &str) -> bool {
        self.classes.contains_key(internal)
    }

    /// Whether this package is the game's own (a reference into it can be checked here).
    fn covers(&self, internal: &str) -> bool {
        self.packages.contains(package_of(internal))
    }

    /// A member through the class, its superclasses and interfaces. Constructors only in the class.
    /// `on_object`: java.lang.Object has it (where nearly every class ends: known, not outside).
    fn find(&self, owner: &str, has: &dyn Fn(&GameClass) -> bool, inherited: bool, on_object: bool) -> Found {
        let mut stack = vec![owner.to_string()];
        let mut seen = HashSet::new();
        let mut left_jar = false;
        while let Some(name) = stack.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            let Some(c) = self.classes.get(&name) else {
                if name == "java/lang/Object" {
                    if on_object {
                        return Found::Yes;
                    }
                } else {
                    left_jar = true;
                }
                continue;
            };
            if has(c) {
                return Found::Yes;
            }
            if inherited {
                stack.extend(c.super_name.iter().cloned());
                stack.extend(c.interfaces.iter().cloned());
            }
        }
        if left_jar {
            Found::Unknown
        } else {
            Found::No
        }
    }

    /// A method by name (any overload), for ZombieBuddy's targets (named without arguments).
    fn has_method_named(&self, owner: &str, name: &str) -> Found {
        self.find(owner, &|c| c.methods.iter().any(|(n, _)| n == name), name != "<init>", OBJECT_METHODS.contains(&name))
    }
}

type JarKey = (PathBuf, u64, u64);
static GAME: Mutex<Option<(JarKey, Arc<GameClasses>)>> = Mutex::new(None);

fn file_stamp(path: &Path) -> (u64, u64) {
    let md = std::fs::metadata(path).ok();
    let len = md.as_ref().map(|m| m.len()).unwrap_or(0);
    let time = md.and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    (len, time)
}

fn game_jar(install_dir: &Path) -> PathBuf {
    install_dir.join("projectzomboid.jar")
}

/// The installed game's classes, read once per jar.
pub fn game_classes(install_dir: &Path) -> Option<Arc<GameClasses>> {
    let jar = game_jar(install_dir);
    let (len, time) = file_stamp(&jar);
    if len == 0 {
        return None;
    }
    let key = (jar.clone(), len, time);
    if let Some((k, g)) = GAME.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
        if *k == key {
            return Some(g.clone());
        }
    }
    let bytes = std::fs::read(&jar).ok()?;
    let mut game = GameClasses::default();
    each_class(&bytes, |data| {
        if let Some(c) = parse(data, false) {
            game.packages.insert(package_of(&c.name).to_string());
            game.classes.insert(
                c.name.clone(),
                GameClass { super_name: c.super_name, interfaces: c.interfaces, methods: c.methods.into_iter().collect(), fields: c.fields.into_iter().collect() },
            );
        }
    })
    .ok()?;
    let game = Arc::new(game);
    *GAME.lock().unwrap_or_else(|p| p.into_inner()) = Some((key, game.clone()));
    Some(game)
}

// ------------------------------------------------------------------ a mod's jar

fn dotted(internal: &str) -> String {
    internal.replace('/', ".")
}

/// The class a descriptor or array name is about (`[Lzombie/Foo;` -> `zombie/Foo`), or None for
/// primitives.
fn element_class(name: &str) -> Option<&str> {
    let n = name.trim_start_matches('[');
    if n.len() < name.len() {
        return n.strip_prefix('L').and_then(|r| r.strip_suffix(';'));
    }
    Some(name)
}

/// What a jar uses of the game that the game does not have, from its bytes.
pub fn check_jar(bytes: &[u8], game: &GameClasses, patches: &[crate::java_patches::JavaPatch]) -> Result<(Vec<Broken>, usize), String> {
    let mut mine: Vec<ClassFile> = Vec::new();
    each_class(bytes, |data| {
        if let Some(c) = parse(data, true) {
            mine.push(c);
        }
    })?;
    let own: HashSet<String> = mine.iter().map(|c| c.name.clone()).collect();
    let checkable = |owner: &str| !own.contains(owner) && game.covers(owner);
    let mut found: BTreeSet<Broken> = BTreeSet::new();
    let mut checked = 0usize;
    for c in &mine {
        for r in &c.class_refs {
            let Some(owner) = element_class(r) else { continue };
            if !checkable(owner) {
                continue;
            }
            checked += 1;
            if !game.has_class(owner) {
                found.insert(Broken { kind: "class".into(), class: dotted(owner), member: String::new(), descriptor: String::new(), used_by: dotted(&c.name) });
            }
        }
        for (tag, owner, name, desc) in &c.member_refs {
            let Some(owner) = element_class(owner) else { continue };
            if !checkable(owner) || !game.has_class(owner) {
                continue; // a missing class is said once, above
            }
            checked += 1;
            let result = if *tag == 9 {
                game.find(owner, &|g| g.fields.contains(name), true, false)
            } else {
                game.find(owner, &|g| g.methods.contains(&(name.clone(), desc.clone())), name != "<init>", OBJECT_METHODS.contains(&name.as_str()))
            };
            if matches!(result, Found::No) {
                let kind = if *tag == 9 { "field" } else { "method" };
                found.insert(Broken { kind: kind.into(), class: dotted(owner), member: name.clone(), descriptor: if *tag == 9 { String::new() } else { desc.clone() }, used_by: dotted(&c.name) });
            }
        }
    }
    // ZombieBuddy's targets: a class and a method name, `*` at either end a wildcard (not checked).
    for p in patches.iter().filter(|p| p.not_applied.is_none()) {
        let Some(class) = p.target_class.as_deref().filter(|c| !c.contains('*')) else { continue };
        let internal = class.replace('.', "/");
        if !game.covers(&internal) {
            continue;
        }
        checked += 1;
        let missing_class = !game.has_class(&internal);
        let missing_method = !missing_class
            && p.target_method.as_deref().filter(|m| !m.contains('*')).is_some_and(|m| matches!(game.has_method_named(&internal, m), Found::No));
        if missing_class || missing_method {
            found.insert(Broken {
                kind: "patch".into(),
                class: class.to_string(),
                member: if missing_class { String::new() } else { p.target_method.clone().unwrap_or_default() },
                descriptor: String::new(),
                used_by: p.patch_class.clone(),
            });
        }
    }
    // One line per thing missing, whichever of the mod's classes uses it.
    let mut seen = HashSet::new();
    let list: Vec<Broken> = found.into_iter().filter(|b| seen.insert((b.kind.clone(), b.class.clone(), b.member.clone(), b.descriptor.clone()))).collect();
    Ok((list, checked))
}

fn cache_file(user_zomboid_dir: &Path, mod_id: &str) -> PathBuf {
    let safe: String = mod_id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
    user_zomboid_dir.join("PZModStudio_Cache").join("java_links").join(format!("{}.json", safe))
}

/// One mod's result, from the cache while its jar and the game's jar are the same. None when the
/// mod has no Java ZombieBuddy loads.
pub fn read_mod(user_zomboid_dir: &Path, install_dir: &Path, m: &crate::catalog::CatalogMod, game_version: Option<u32>) -> Option<ModLinks> {
    let (jar_rel, jar_path, missing) = crate::java_patches::jar_of(m, game_version)?;
    let mut out = ModLinks { mod_id: m.id.clone(), name: m.name.clone(), jar: Some(jar_rel), ..Default::default() };
    if let Some(why) = missing {
        out.unknown = Some(why.to_string());
        return Some(out);
    }
    let (glen, gtime) = file_stamp(&game_jar(install_dir));
    let (mlen, mtime) = file_stamp(&jar_path);
    let key = format!("{}|{}|{}|{}|{}|{}", ENGINE, jar_path.display(), mlen, mtime, glen, gtime);
    let cache = cache_file(user_zomboid_dir, &m.id);
    if let Some(kept) = std::fs::read_to_string(&cache).ok().and_then(|t| serde_json::from_str::<ModLinks>(&t).ok()) {
        if kept.key == key && kept.mod_id == m.id {
            return Some(ModLinks { name: m.name.clone(), ..kept });
        }
    }
    let Some(game) = game_classes(install_dir) else {
        out.unknown = Some("no_game_jar".into());
        return Some(out);
    };
    if mlen > MAX_JAR_BYTES {
        out.unknown = Some("unreadable: the jar is too large to read".into());
        return Some(out);
    }
    let patches = crate::java_patches::read_mod(user_zomboid_dir, m, game_version).map(|p| p.patches).unwrap_or_default();
    match std::fs::read(&jar_path).map_err(|e| e.to_string()).and_then(|b| check_jar(&b, &game, &patches)) {
        Ok((broken, checked)) => {
            out.total = broken.len();
            out.checked = checked;
            out.broken = broken.into_iter().take(MAX_FINDINGS).collect();
        }
        Err(e) => out.unknown = Some(format!("unreadable: {}", e)),
    }
    out.key = key;
    if let Some(dir) = cache.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string(&out) {
        let _ = std::fs::write(&cache, text);
    }
    Some(out)
}

/// The Java mods among these entries, each once, in list order.
pub fn read_mods(user_zomboid_dir: &Path, install_dir: &Path, catalog: &crate::catalog::ModCatalog, ids: &[String]) -> Vec<ModLinks> {
    let version = catalog.game_version_int();
    let mut seen = HashSet::new();
    ids.iter()
        .filter_map(|e| catalog.resolve(e.trim()))
        .filter(|m| seen.insert(m.id.clone()))
        .filter_map(|m| read_mod(user_zomboid_dir, install_dir, m, version))
        .collect()
}

/// `zombie.Foo.bar(...)`, `zombie.Foo.baz`, `zombie.Foo`: one missing thing, short.
pub fn short(b: &Broken) -> String {
    match b.kind.as_str() {
        "class" => b.class.clone(),
        "field" => format!("{}.{}", b.class, b.member),
        _ if b.member.is_empty() => b.class.clone(),
        _ => format!("{}.{}()", b.class, b.member),
    }
}

/// Diagnostics: one finding per mod whose Java uses what the build lacks.
pub fn issues(mods: &[ModLinks], zombiebuddy_in_list: bool) -> Vec<crate::catalog::HealthIssue> {
    use crate::catalog::{issue_params, issue_state, HealthIssue, Severity};
    if !zombiebuddy_in_list {
        return Vec::new(); // no jar loads without ZombieBuddy
    }
    mods.iter()
        .filter(|m| !m.broken.is_empty())
        .map(|m| {
            let examples: Vec<String> = m.broken.iter().take(3).map(short).collect();
            let more = m.total.saturating_sub(examples.len());
            let more_text = if more > 0 { format!(" and {} more", more) } else { String::new() };
            let patches = m.broken.iter().filter(|b| b.kind == "patch").count();
            let patch_text = if patches > 0 { " Some are what its patches change: ZombieBuddy skips a patch whose target is gone, so that change does not happen." } else { "" };
            let message = format!(
                "{}'s Java uses {} thing(s) this build of the game no longer has: {}{}. That code fails when it runs, often as the game starts or a save loads.{} If the Monitor shows it failing, look for an update of {} or turn it off. If the game runs fine with it, that part is not used (or the mod checks before using it) and there is nothing to do.",
                m.name,
                m.total,
                examples.join(", "),
                more_text,
                patch_text,
                m.name
            );
            let severity = Severity::Warning;
            let mut params = issue_params(&[
                ("mod", m.name.as_str()),
                ("examples", examples.join(", ").as_str()),
                ("moreKey", if more > 0 { "jl_more" } else { "" }),
                ("patchKey", if patches > 0 { "jl_patch" } else { "" }),
            ]);
            params.insert("count".to_string(), m.total.to_string());
            // Only when there is more: an empty phrase keeps the value sent, and a 0 was said after the examples.
            if more > 0 {
                params.insert("more".to_string(), more.to_string());
            }
            HealthIssue {
                severity,
                state: issue_state(severity, "java_link_broken"),
                code: "java_link_broken".to_string(),
                mod_id: Some(m.mod_id.clone()),
                message,
                fixes: Vec::new(),
                related: Vec::new(),
                details: vec![],
                params,
                own_mod: false,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(classes: &[(&str, Option<&str>, &[(&str, &str)], &[&str])]) -> GameClasses {
        let mut g = GameClasses::default();
        for (name, sup, methods, fields) in classes {
            g.packages.insert(package_of(name).to_string());
            g.classes.insert(
                name.to_string(),
                GameClass {
                    super_name: sup.map(str::to_string),
                    interfaces: Vec::new(),
                    methods: methods.iter().map(|(n, d)| (n.to_string(), d.to_string())).collect(),
                    fields: fields.iter().map(|f| f.to_string()).collect(),
                },
            );
        }
        g
    }

    #[test]
    fn members_are_found_through_superclasses_and_leaving_the_jar_is_not_missing() {
        let g = game(&[
            ("zombie/characters/IsoGameCharacter", Some("java/lang/Object"), &[("getHealth", "()F")], &["x"]),
            ("zombie/characters/IsoPlayer", Some("zombie/characters/IsoGameCharacter"), &[("<init>", "()V")], &[]),
        ]);
        assert!(matches!(g.find("zombie/characters/IsoPlayer", &|c| c.methods.contains(&("getHealth".into(), "()F".into())), true, false), Found::Yes));
        assert!(matches!(g.find("zombie/characters/IsoPlayer", &|c| c.methods.contains(&("gone".into(), "()V".into())), true, false), Found::No), "Object is known: gone is gone");
        assert!(matches!(g.find("zombie/characters/IsoPlayer", &|_| false, true, true), Found::Yes), "toString comes from Object");
        assert!(matches!(g.find("zombie/characters/IsoPlayer", &|c| c.fields.contains("x"), true, false), Found::Yes));
        let lib = game(&[("zombie/util/MyList", Some("java/util/ArrayList"), &[], &[])]);
        assert!(matches!(lib.find("zombie/util/MyList", &|_| false, true, false), Found::Unknown), "ArrayList is outside the jar");
        assert!(g.covers("zombie/characters/Anything") && !g.covers("org/lwjgl/opengl/GL11"));
        assert_eq!(element_class("[Lzombie/Foo;"), Some("zombie/Foo"));
        assert_eq!(element_class("[I"), None);
    }

    #[test]
    fn a_method_missing_from_a_class_with_no_way_out_is_missing() {
        let g = game(&[("zombie/core/Core", None, &[("getVersion", "()Ljava/lang/String;")], &[])]);
        assert!(matches!(g.find("zombie/core/Core", &|c| c.methods.contains(&("getOldThing".into(), "()V".into())), true, false), Found::No));
        assert!(matches!(g.has_method_named("zombie/core/Core", "getVersion"), Found::Yes));
    }
}
