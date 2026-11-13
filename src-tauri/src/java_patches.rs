//! Which classes a mod's Java changes, and where two active mods change the same one.
//!
//! The facts are ZombieBuddy's own (its source ships with the mod, `java/src/.../Patch.java`,
//! `PatchEngine.java`, `JavaModInfo.java`, 2.3.4):
//!
//! - A mod's `mod.info` names one jar (`javaJarFile=`, the first such line) and one package
//!   (`javaPkgName=`, the first). Without the package ZombieBuddy does not load the jar. On
//!   Build 42 it tries, in this order, the version folder's mod.info with the jar inside the
//!   version folder, `common/mod.info` with the jar inside `common`, and `common/mod.info` with
//!   the jar inside the version folder; the first with a package and a jar that exists wins.
//! - ZombieBuddy scans the classes of exactly that package (not its sub-packages) for the
//!   class-level, runtime-visible annotation `me.zed_0xff.zombie_buddy.Patch`:
//!   `@Patch(className = "zombie.X", methodName = "update", isAdvice = true, warmUp = false, ...)`.
//!   One patch class names one target class and one method name. `*` at either end of a name is
//!   a wildcard, `<init>` is the constructors. `isAdvice = false` replaces the method (ByteBuddy
//!   MethodDelegation); the default adds code before and/or after it (ByteBuddy Advice), and
//!   advices of several mods on one method stack.
//! - What an advice does is on its methods: `@Patch.OnEnter` (with `skipOn = true` it can skip
//!   the original method), `@Patch.OnExit`, and on their parameters `@Patch.Return(readOnly =
//!   false)` (it can change what the method returns) and `@Patch.Argument(readOnly = false)` /
//!   `@Patch.AllArguments(readOnly = false)` (it can change the arguments). ByteBuddy's own
//!   annotations (`Advice$OnMethodEnter` ...) mean the same.
//! - Patches whose target starts with `me.zed_0xff.` are refused, and so is
//!   `LuaManager$Exposer.exposeAll` without `IKnowWhatIAmDoing`.
//! - ZombieBuddy's own jar applies its `.patches` package always and `.patches.experimental`
//!   only when its experimental switch is on.
//! - A mod's `Main` may ask ZombieBuddy for more packages itself
//!   (`PatchEngine.applyPatches("other.pkg", loader)`), whenever and under whatever condition its
//!   code decides. Seen on this PC: Shift Your Gear (`sygmptest.Main` applies `gsa`, logged as
//!   "Scanned 4 classes in package gsa"). A `@Patch` class outside the scanned package is
//!   therefore counted, and marked `by_code`, only when one of the jar's own classes both refers
//!   to `PatchEngine` and holds that package's name as a string; otherwise `outside_package`.
//!
//! All of that is declared in the jar, so it is read from the class files' annotation attributes
//! (JVMS 4.7.16, 4.7.18) without running anything. What a jar does in code instead (its `Main`
//! calling ByteBuddy or `Instrumentation` itself) is not declared anywhere: such a jar is marked
//! `patches_in_code` and nothing is claimed about what it changes that way.
//!
//! Kept per mod in `PZModStudio_Cache/java_patches/` until the jar or the `mod.info` changes.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::catalog::{CatalogMod, ModCatalog};

/// Bumped when what is read or kept changes, so older caches are read again.
const ENGINE: &str = "java-patches-v1";
const MAX_JAR_BYTES: u64 = 256 * 1024 * 1024;
const ZOMBIEBUDDY_ID: &str = "ZombieBuddy";
const ZB_PACKAGE: &str = "me.zed_0xff.zombie_buddy";
const PATCH_ANNOTATION: &str = "Lme/zed_0xff/zombie_buddy/Patch;";

// ------------------------------------------------------------------ the class file

/// One annotation element's value.
#[derive(Debug, Clone, PartialEq)]
enum Val {
    Str(String),
    Int(i32),
    /// A class literal, as its descriptor (`V` is `void.class`).
    Class(String),
    Other,
}

/// One annotation: its type descriptor (`Lme/zed_0xff/zombie_buddy/Patch;`) and its elements.
#[derive(Debug, Clone, PartialEq)]
struct Ann {
    type_desc: String,
    values: Vec<(String, Val)>,
}

impl Ann {
    fn get(&self, name: &str) -> Option<&Val> {
        self.values.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }
    fn str(&self, name: &str) -> Option<&str> {
        match self.get(name) {
            Some(Val::Str(s)) => Some(s),
            _ => None,
        }
    }
    fn bool(&self, name: &str) -> Option<bool> {
        match self.get(name) {
            Some(Val::Int(v)) => Some(*v != 0),
            _ => None,
        }
    }
}

struct MethodAnns {
    #[allow(dead_code)]
    name: String,
    annotations: Vec<Ann>,
    /// Every annotation on every parameter.
    param_annotations: Vec<Ann>,
}

/// One class, as far as patches need it.
struct ClassAnns {
    /// `pkg/Outer$Inner`.
    name: String,
    annotations: Vec<Ann>,
    methods: Vec<MethodAnns>,
    /// The classes its constant pool refers to (`net/bytebuddy/agent/builder/AgentBuilder`).
    class_refs: Vec<String>,
    /// Its string literals (CONSTANT_String).
    strings: Vec<String>,
}

struct Pool {
    utf8: Vec<Option<String>>,
    ints: Vec<Option<i32>>,
    /// Class entries: slot -> name index.
    classes: Vec<Option<usize>>,
    /// String entries: slot -> text index.
    strings: Vec<Option<usize>>,
}

impl Pool {
    fn str(&self, idx: usize) -> Option<String> {
        self.utf8.get(idx).cloned().flatten()
    }
}

fn u16_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?]) as usize)
}

fn u32_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_be_bytes([*b.get(i)?, *b.get(i + 1)?, *b.get(i + 2)?, *b.get(i + 3)?]) as usize)
}

/// Reads one `element_value` (JVMS 4.7.16.1).
fn element_value(b: &[u8], i: &mut usize, pool: &Pool, depth: u32) -> Option<Val> {
    if depth > 16 {
        return None;
    }
    let tag = *b.get(*i)?;
    *i += 1;
    match tag {
        b's' => {
            let idx = u16_at(b, *i)?;
            *i += 2;
            Some(pool.str(idx).map(Val::Str).unwrap_or(Val::Other))
        }
        b'B' | b'C' | b'I' | b'S' | b'Z' => {
            let idx = u16_at(b, *i)?;
            *i += 2;
            Some(pool.ints.get(idx).cloned().flatten().map(Val::Int).unwrap_or(Val::Other))
        }
        b'D' | b'F' | b'J' => {
            u16_at(b, *i)?;
            *i += 2;
            Some(Val::Other)
        }
        b'c' => {
            let idx = u16_at(b, *i)?;
            *i += 2;
            Some(pool.str(idx).map(Val::Class).unwrap_or(Val::Other))
        }
        b'e' => {
            u16_at(b, *i + 2)?;
            *i += 4;
            Some(Val::Other)
        }
        b'@' => {
            annotation(b, i, pool, depth + 1)?;
            Some(Val::Other)
        }
        b'[' => {
            let n = u16_at(b, *i)?;
            *i += 2;
            for _ in 0..n {
                element_value(b, i, pool, depth + 1)?;
            }
            Some(Val::Other)
        }
        _ => None,
    }
}

/// Reads one `annotation` structure.
fn annotation(b: &[u8], i: &mut usize, pool: &Pool, depth: u32) -> Option<Ann> {
    let type_idx = u16_at(b, *i)?;
    let pairs = u16_at(b, *i + 2)?;
    *i += 4;
    let mut values = Vec::with_capacity(pairs);
    for _ in 0..pairs {
        let name_idx = u16_at(b, *i)?;
        *i += 2;
        let value = element_value(b, i, pool, depth)?;
        values.push((pool.str(name_idx).unwrap_or_default(), value));
    }
    Some(Ann { type_desc: pool.str(type_idx).unwrap_or_default(), values })
}

/// The annotations of a `Runtime(In)VisibleAnnotations` attribute; what could be read of a
/// damaged one.
fn annotations_attr(attr: &[u8], pool: &Pool) -> Vec<Ann> {
    let mut out = Vec::new();
    let Some(n) = u16_at(attr, 0) else { return out };
    let mut i = 2;
    for _ in 0..n {
        match annotation(attr, &mut i, pool, 0) {
            Some(a) => out.push(a),
            None => break,
        }
    }
    out
}

/// The annotations of a `Runtime(In)VisibleParameterAnnotations` attribute, of every parameter.
fn parameter_annotations_attr(attr: &[u8], pool: &Pool) -> Vec<Ann> {
    let mut out = Vec::new();
    let Some(&params) = attr.first() else { return out };
    let mut i = 1;
    for _ in 0..params {
        let Some(n) = u16_at(attr, i) else { return out };
        i += 2;
        for _ in 0..n {
            match annotation(attr, &mut i, pool, 0) {
                Some(a) => out.push(a),
                None => return out,
            }
        }
    }
    out
}

/// A class file's name, its annotations and its methods' (JVMS 4.1). None when it is not one.
fn parse_class(bytes: &[u8]) -> Option<ClassAnns> {
    if bytes.len() < 10 || bytes[..4] != [0xCA, 0xFE, 0xBA, 0xBE] {
        return None;
    }
    let count = u16_at(bytes, 8)?;
    let mut pool = Pool { utf8: vec![None; count], ints: vec![None; count], classes: vec![None; count], strings: vec![None; count] };
    let mut i = 10;
    let mut slot = 1;
    while slot < count {
        let tag = *bytes.get(i)?;
        i += 1;
        let size = match tag {
            1 => {
                let len = u16_at(bytes, i)?;
                let s = bytes.get(i + 2..i + 2 + len)?;
                pool.utf8[slot] = Some(String::from_utf8_lossy(s).to_string());
                2 + len
            }
            3 => {
                pool.ints[slot] = Some(u32_at(bytes, i)? as u32 as i32);
                4
            }
            4 => 4,
            5 | 6 => {
                slot += 1;
                8
            }
            7 => {
                pool.classes[slot] = Some(u16_at(bytes, i)?);
                2
            }
            8 => {
                pool.strings[slot] = Some(u16_at(bytes, i)?);
                2
            }
            16 | 19 | 20 => 2,
            9 | 10 | 11 | 12 | 17 | 18 => 4,
            15 => 3,
            _ => return None,
        };
        i += size;
        slot += 1;
    }
    let this = u16_at(bytes, i + 2)?;
    let name = pool.classes.get(this).cloned().flatten().and_then(|n| pool.str(n))?;
    let ifaces = u16_at(bytes, i + 6)?;
    i += 8 + 2 * ifaces;

    let is_annotations = |n: Option<String>| matches!(n.as_deref(), Some("RuntimeVisibleAnnotations") | Some("RuntimeInvisibleAnnotations"));
    let is_param_annotations =
        |n: Option<String>| matches!(n.as_deref(), Some("RuntimeVisibleParameterAnnotations") | Some("RuntimeInvisibleParameterAnnotations"));

    // fields, then methods: access, name, descriptor, attributes
    let mut methods = Vec::new();
    for pass in 0..2 {
        let n = u16_at(bytes, i)?;
        i += 2;
        for _ in 0..n {
            let name_idx = u16_at(bytes, i + 2)?;
            let attrs = u16_at(bytes, i + 6)?;
            i += 8;
            let mut m = MethodAnns { name: pool.str(name_idx).unwrap_or_default(), annotations: Vec::new(), param_annotations: Vec::new() };
            for _ in 0..attrs {
                let attr_name = u16_at(bytes, i)?;
                let len = u32_at(bytes, i + 2)?;
                let body = bytes.get(i + 6..i + 6 + len)?;
                if pass == 1 {
                    if is_annotations(pool.str(attr_name)) {
                        m.annotations.extend(annotations_attr(body, &pool));
                    } else if is_param_annotations(pool.str(attr_name)) {
                        m.param_annotations.extend(parameter_annotations_attr(body, &pool));
                    }
                }
                i += 6 + len;
            }
            if pass == 1 {
                methods.push(m);
            }
        }
    }
    // the class's own attributes
    let mut annotations = Vec::new();
    let attrs = u16_at(bytes, i)?;
    i += 2;
    for _ in 0..attrs {
        let attr_name = u16_at(bytes, i)?;
        let len = u32_at(bytes, i + 2)?;
        let body = bytes.get(i + 6..i + 6 + len)?;
        if is_annotations(pool.str(attr_name)) {
            annotations.extend(annotations_attr(body, &pool));
        }
        i += 6 + len;
    }
    let class_refs = pool.classes.iter().flatten().filter_map(|n| pool.str(*n)).collect();
    let strings = pool.strings.iter().flatten().filter_map(|n| pool.str(*n)).collect();
    Some(ClassAnns { name, annotations, methods, class_refs, strings })
}

// ------------------------------------------------------------------ a jar's patches

/// How a patch changes its method, as the jar declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchKind {
    /// Code added before and/or after the method (ByteBuddy Advice); several mods' advices stack.
    Advice,
    /// The method is replaced (`isAdvice = false`, ByteBuddy MethodDelegation); ZombieBuddy keeps
    /// one delegation per method.
    Delegation,
}

/// One `@Patch` class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JavaPatch {
    /// The class it names (`zombie.characters.IsoPlayer`; `*` at either end is a wildcard).
    /// None when the annotation carries no readable `className`.
    pub target_class: Option<String>,
    /// The method name it names (`update`, `<init>` for constructors, `*` wildcards). None when
    /// the annotation carries no readable `methodName`. By name only: every overload matches
    /// unless the patch narrows it by its arguments, which is not read here.
    pub target_method: Option<String>,
    /// The mod's class carrying the annotation (`pz.mymod.Patch_IsoPlayer`).
    pub patch_class: String,
    pub kind: PatchKind,
    /// What its methods declare, of: `enter` (runs before), `exit` (runs after), `skip` (can
    /// skip the original method), `return` (can change what it returns), `args` (can change its
    /// arguments). Empty when nothing of that is declared.
    pub effects: Vec<String>,
    /// The patch is not in the package ZombieBuddy scans on its own: the mod's code asks
    /// ZombieBuddy for its package (`PatchEngine.applyPatches`), when and if that code runs.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub by_code: bool,
    /// Why ZombieBuddy does not apply it though the jar has it: `outside_package` (not in the
    /// package of `javaPkgName`, the only one scanned, and no code of the jar asks for its
    /// package), `zombiebuddy_class` (targets
    /// ZombieBuddy's own classes), `exposer` (`LuaManager$Exposer.exposeAll` unconfirmed),
    /// `experimental` (ZombieBuddy's own experimental patches, off unless switched on).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_applied: Option<String>,
}

/// What one jar's bytes declare.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JarPatches {
    pub patches: Vec<JavaPatch>,
    pub classes: usize,
    /// Libraries the jar's own classes use that change classes in code, outside `@Patch`:
    /// `bytebuddy`, `instrumentation`, `asm`, `javassist`.
    pub patches_in_code: Vec<String>,
    /// The packages of the jar's own classes (bundled libraries left out).
    pub own_packages: Vec<String>,
}

fn dotted(internal: &str) -> String {
    internal.replace('/', ".")
}

fn package_of(dotted_class: &str) -> &str {
    dotted_class.rsplit_once('.').map(|(p, _)| p).unwrap_or("")
}

/// Libraries bundled into a jar: their classes are not the mod's own code.
fn is_bundled_library(internal: &str) -> bool {
    ["net/bytebuddy/", "org/objectweb/asm/", "javassist/", "io/github/classgraph/", "nonapi/io/github/classgraph/", "kotlin/", "com/google/", "org/apache/", "me/zed_0xff/zombie_buddy/"]
        .iter()
        .any(|p| internal.starts_with(p))
}

fn code_patching_of(class_ref: &str) -> Option<&'static str> {
    if class_ref.starts_with("net/bytebuddy/agent/") || class_ref.starts_with("net/bytebuddy/dynamic/") || class_ref == "net/bytebuddy/ByteBuddy" {
        Some("bytebuddy")
    } else if class_ref.starts_with("java/lang/instrument/") {
        Some("instrumentation")
    } else if class_ref.starts_with("org/objectweb/asm/") {
        Some("asm")
    } else if class_ref.starts_with("javassist/") {
        Some("javassist")
    } else {
        None
    }
}

/// The `@Patch` of one class, when it has one. `scanned` are the packages ZombieBuddy scans in
/// this jar; `experimental` the one it scans only when asked.
fn patch_of(class: &ClassAnns, scanned: &[String], experimental: Option<&str>, asked_in_code: &BTreeSet<String>) -> Option<JavaPatch> {
    let ann = class.annotations.iter().find(|a| a.type_desc == PATCH_ANNOTATION)?;
    let patch_class = dotted(&class.name);
    let target_class = ann.str("className").map(str::to_string).filter(|s| !s.is_empty());
    let target_method = ann.str("methodName").map(str::to_string).filter(|s| !s.is_empty());
    let kind = if ann.bool("isAdvice") == Some(false) { PatchKind::Delegation } else { PatchKind::Advice };

    let mut effects: BTreeSet<&str> = BTreeSet::new();
    for m in &class.methods {
        for a in &m.annotations {
            match a.type_desc.as_str() {
                "Lme/zed_0xff/zombie_buddy/Patch$OnEnter;" => {
                    effects.insert("enter");
                    if a.bool("skipOn") == Some(true) {
                        effects.insert("skip");
                    }
                }
                "Lnet/bytebuddy/asm/Advice$OnMethodEnter;" => {
                    effects.insert("enter");
                    // ByteBuddy's default is void.class: never skip.
                    if matches!(a.get("skipOn"), Some(Val::Class(c)) if c != "V") {
                        effects.insert("skip");
                    }
                }
                "Lme/zed_0xff/zombie_buddy/Patch$OnExit;" | "Lnet/bytebuddy/asm/Advice$OnMethodExit;" => {
                    effects.insert("exit");
                }
                _ => {}
            }
        }
        for a in &m.param_annotations {
            let writes = a.bool("readOnly") == Some(false);
            match a.type_desc.as_str() {
                "Lme/zed_0xff/zombie_buddy/Patch$Return;" | "Lnet/bytebuddy/asm/Advice$Return;" if writes => {
                    effects.insert("return");
                }
                "Lme/zed_0xff/zombie_buddy/Patch$Argument;"
                | "Lme/zed_0xff/zombie_buddy/Patch$AllArguments;"
                | "Lnet/bytebuddy/asm/Advice$Argument;"
                | "Lnet/bytebuddy/asm/Advice$AllArguments;"
                    if writes =>
                {
                    effects.insert("args");
                }
                _ => {}
            }
        }
    }

    let package = package_of(&patch_class);
    let in_scanned = scanned.iter().any(|p| p == package);
    let by_code = !in_scanned && experimental != Some(package) && asked_in_code.contains(package);
    let not_applied = if experimental == Some(package) {
        Some("experimental")
    } else if !in_scanned && !by_code {
        Some("outside_package")
    } else if target_class.as_deref().is_some_and(|c| c.starts_with("me.zed_0xff.")) {
        Some("zombiebuddy_class")
    } else if target_class.as_deref() == Some("zombie.Lua.LuaManager$Exposer")
        && target_method.as_deref() == Some("exposeAll")
        && ann.bool("IKnowWhatIAmDoing") != Some(true)
    {
        Some("exposer")
    } else {
        None
    };
    // The order of effects is fixed, so two reads of one jar say the same.
    let order = ["enter", "exit", "skip", "return", "args"];
    Some(JavaPatch {
        target_class,
        target_method,
        patch_class,
        kind,
        effects: order.iter().filter(|e| effects.contains(*e)).map(|e| e.to_string()).collect(),
        by_code,
        not_applied: not_applied.map(str::to_string),
    })
}

/// The packages ZombieBuddy scans for `javaPkgName = package`, and the one it scans only with
/// its experimental switch (its own jar: `.patches` and `.patches.experimental`).
fn scanned_packages(package: &str) -> (Vec<String>, Option<String>) {
    if package == ZB_PACKAGE {
        (vec![format!("{}.patches", ZB_PACKAGE)], Some(format!("{}.patches.experimental", ZB_PACKAGE)))
    } else {
        (vec![package.to_string()], None)
    }
}

/// Reads a jar's bytes (a zip of class files) for its `@Patch` classes.
pub fn read_jar(bytes: &[u8], package: &str) -> Result<JarPatches, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("not a jar: {}", e))?;
    let (scanned, experimental) = scanned_packages(package);
    let mut out = JarPatches::default();
    let mut in_code: BTreeSet<&'static str> = BTreeSet::new();
    let mut classes: Vec<ClassAnns> = Vec::new();
    for idx in 0..zip.len() {
        let Ok(mut entry) = zip.by_index(idx) else { continue };
        if !entry.name().ends_with(".class") || entry.size() > 16 * 1024 * 1024 {
            continue;
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        if entry.read_to_end(&mut data).is_err() {
            continue;
        }
        let Some(class) = parse_class(&data) else { continue };
        out.classes += 1;
        // Only what a patch or the jar's own code can say is kept.
        if !is_bundled_library(&class.name) || class.annotations.iter().any(|a| a.type_desc == PATCH_ANNOTATION) {
            classes.push(class);
        }
    }
    let own: Vec<&ClassAnns> = classes.iter().filter(|c| !is_bundled_library(&c.name)).collect();
    let own_packages: BTreeSet<String> = own.iter().map(|c| package_of(&dotted(&c.name)).to_string()).collect();
    // The packages the jar's own code hands to ZombieBuddy: a class that refers to PatchEngine
    // and holds the name of one of the jar's packages as a string.
    let mut asked_in_code: BTreeSet<String> = BTreeSet::new();
    for c in &own {
        in_code.extend(c.class_refs.iter().filter_map(|r| code_patching_of(r)));
        if c.class_refs.iter().any(|r| r == "me/zed_0xff/zombie_buddy/PatchEngine") {
            asked_in_code.extend(c.strings.iter().filter(|s| own_packages.contains(*s)).cloned());
        }
    }
    for class in &classes {
        if let Some(p) = patch_of(class, &scanned, experimental.as_deref(), &asked_in_code) {
            out.patches.push(p);
        }
    }
    out.own_packages = own_packages.into_iter().filter(|p| !p.is_empty()).collect();
    out.patches.sort_by(|a, b| (&a.target_class, &a.target_method, &a.patch_class).cmp(&(&b.target_class, &b.target_method, &b.patch_class)));
    out.patches_in_code = in_code.into_iter().map(str::to_string).collect();
    Ok(out)
}

// ------------------------------------------------------------------ a mod

/// What a mod's Java declares it changes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModPatches {
    pub mod_id: String,
    pub name: String,
    /// The jar `mod.info` names (`javaJarFile=`), as written there. None: the mod has no Java
    /// that ZombieBuddy loads.
    pub jar: Option<String>,
    /// `javaPkgName=`: the one package ZombieBuddy scans for patches.
    pub package: Option<String>,
    /// `client` / `server` when the jar's path has `media/java/client/` or `media/java/server/`:
    /// ZombieBuddy skips the first on a dedicated server and the second in the game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    pub patches: Vec<JavaPatch>,
    /// The classes ZombieBuddy applies patches to (sorted, no repeats; wildcards as written).
    pub target_classes: Vec<String>,
    /// Class files read from the jar.
    pub classes_read: usize,
    /// The jar's own classes use ByteBuddy, `Instrumentation`, ASM or Javassist directly: it can
    /// change classes in code too, which no annotation declares. What it changes that way is
    /// not known here.
    pub patches_in_code: Vec<String>,
    /// The packages of the jar's own classes: a patch of another mod that targets one of them
    /// changes this mod's class, not the game's.
    #[serde(default)]
    pub own_packages: Vec<String>,
    /// Why the patches could not be read (`jar_missing`, `no_package`, `unreadable: ...`), when so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unknown: Option<String>,
    /// Jar and mod.info size and date, and the reader's version: the cache is for these.
    #[serde(default)]
    pub key: String,
}

impl ModPatches {
    /// The patches ZombieBuddy applies, with a known target class.
    pub fn applied(&self) -> impl Iterator<Item = &JavaPatch> {
        self.patches.iter().filter(|p| p.not_applied.is_none() && p.target_class.is_some())
    }
}

/// The first value of `key=` in a mod.info that `keep` accepts, read the way ZombieBuddy does:
/// the line starts with the key (any case), the value is trimmed, empty values are passed over.
fn info_value(text: &str, key: &str, keep: impl Fn(&str) -> bool) -> Option<String> {
    let prefix = format!("{}=", key.to_ascii_lowercase());
    text.split('\n')
        .flat_map(|l| l.split('\r'))
        .map(|l| l.trim_start_matches('\u{feff}'))
        .filter(|l| l.to_ascii_lowercase().starts_with(&prefix))
        .map(|l| l[prefix.len()..].trim().to_string())
        .find(|v| !v.is_empty() && keep(v))
}

fn file_key(path: &Path) -> String {
    std::fs::metadata(path)
        .ok()
        .map(|m| {
            let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
            format!("{}-{}", m.len(), t)
        })
        .unwrap_or_else(|| "none".to_string())
}

fn cache_file(user_zomboid_dir: &Path, mod_id: &str) -> PathBuf {
    user_zomboid_dir.join("PZModStudio_Cache").join("java_patches").join(format!("{}.json", crate::security::safe_name(mod_id)))
}

/// Where a mod's jar is, as ZombieBuddy finds it.
struct JarPlace {
    info_path: PathBuf,
    /// `javaJarFile=` as written.
    jar_rel: String,
    jar_path: PathBuf,
    package: Option<String>,
    /// `no_package` / `jar_missing` when ZombieBuddy would not load it from here.
    missing: Option<&'static str>,
}

/// The jar ZombieBuddy loads for this mod on the installed build (`Loader` + `JavaModInfo`):
/// Build 42 tries the version folder's mod.info with the jar in the version folder, then
/// `common/mod.info` with the jar in `common`, then `common/mod.info` with the jar in the
/// version folder; Build 41 the mod's root. The first with a package and a jar that exists wins.
/// None: no mod.info of the mod names a jar. When none loads, the first that names one is
/// returned with why not.
fn locate(m: &CatalogMod, game_version: Option<u32>) -> Option<JarPlace> {
    let root = Path::new(&m.root_dir);
    let common = root.join("common");
    let layers = crate::load_order::mod_info::media_layers_for_build(root, game_version);
    let mut tries: Vec<(PathBuf, PathBuf)> = Vec::new(); // (the mod.info's folder, the jar's folder)
    if layers.len() == 1 && layers[0] == root {
        tries.push((root.to_path_buf(), root.to_path_buf()));
    } else {
        let version = layers.iter().find(|d| **d != common).cloned();
        if let Some(v) = &version {
            tries.push((v.clone(), v.clone()));
        }
        if common.is_dir() {
            tries.push((common.clone(), common.clone()));
            if let Some(v) = &version {
                tries.push((common.clone(), v.clone()));
            }
        }
    }
    let mut fallback: Option<JarPlace> = None;
    for (info_dir, jar_dir) in tries {
        let info_path = info_dir.join("mod.info");
        let Some(text) = std::fs::read(&info_path).ok().map(|b| String::from_utf8_lossy(&b).into_owned()) else { continue };
        let Some(jar_rel) = info_value(&text, "javaJarFile", |v| v.ends_with(".jar")) else { continue };
        let package = info_value(&text, "javaPkgName", |_| true);
        let jar_path = jar_dir.join(jar_rel.replace('\\', "/"));
        let missing = if package.is_none() {
            Some("no_package")
        } else if !jar_path.is_file() {
            Some("jar_missing")
        } else {
            None
        };
        let place = JarPlace { info_path, jar_rel, jar_path, package, missing };
        if missing.is_none() {
            return Some(place);
        }
        fallback.get_or_insert(place);
    }
    fallback
}

fn read_place(m: &CatalogMod, place: &JarPlace) -> ModPatches {
    let lower = place.jar_rel.replace('\\', "/").to_ascii_lowercase();
    let side = if lower.contains("media/java/client/") {
        Some("client".to_string())
    } else if lower.contains("media/java/server/") {
        Some("server".to_string())
    } else {
        None
    };
    let mut out = ModPatches { mod_id: m.id.clone(), name: m.name.clone(), jar: Some(place.jar_rel.clone()), package: place.package.clone(), side, ..Default::default() };
    if let Some(why) = place.missing {
        out.unknown = Some(why.to_string());
        return out;
    }
    let package = place.package.clone().unwrap_or_default();
    if std::fs::metadata(&place.jar_path).map(|md| md.len() > MAX_JAR_BYTES).unwrap_or(false) {
        out.unknown = Some("unreadable: the jar is too large to read".to_string());
        return out;
    }
    match std::fs::read(&place.jar_path).map_err(|e| e.to_string()).and_then(|b| read_jar(&b, &package)) {
        Ok(jar) => {
            out.target_classes = jar
                .patches
                .iter()
                .filter(|p| p.not_applied.is_none())
                .filter_map(|p| p.target_class.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            out.patches = jar.patches;
            out.classes_read = jar.classes;
            out.patches_in_code = jar.patches_in_code;
            out.own_packages = jar.own_packages;
        }
        Err(e) => out.unknown = Some(format!("unreadable: {}", e)),
    }
    out
}

/// The mod's patches, read from its jar now (no cache). None when no mod.info of it names a jar.
pub fn read_mod_uncached(m: &CatalogMod, game_version: Option<u32>) -> Option<ModPatches> {
    locate(m, game_version).map(|place| read_place(m, &place))
}

/// The mod's patches, from the cache while its jar and mod.info are the same.
pub fn read_mod(user_zomboid_dir: &Path, m: &CatalogMod, game_version: Option<u32>) -> Option<ModPatches> {
    let place = locate(m, game_version)?;
    let key = format!("{}|{}|{}|{}", ENGINE, place.jar_path.display(), file_key(&place.jar_path), file_key(&place.info_path));
    let cache = cache_file(user_zomboid_dir, &m.id);
    if let Some(kept) = std::fs::read_to_string(&cache).ok().and_then(|t| serde_json::from_str::<ModPatches>(&t).ok()) {
        if kept.key == key && kept.mod_id == m.id {
            return Some(ModPatches { name: m.name.clone(), ..kept });
        }
    }
    let mut read = read_place(m, &place);
    read.key = key;
    if let Some(dir) = cache.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string(&read) {
        let _ = std::fs::write(&cache, text);
    }
    Some(read)
}

/// The Java mods among these list entries (or ids), each once, in list order.
pub fn read_mods(user_zomboid_dir: &Path, catalog: &ModCatalog, ids: &[String]) -> Vec<ModPatches> {
    let game_version = catalog.game_version_int();
    let mut seen = BTreeSet::new();
    ids.iter()
        .filter_map(|e| catalog.resolve(e.trim()))
        .filter(|m| seen.insert(m.id.clone()))
        .filter_map(|m| read_mod(user_zomboid_dir, m, game_version))
        .collect()
}

// ------------------------------------------------------------------ overlaps

/// Whether a name ZombieBuddy was given (`*` at either end is a wildcard) matches a plain name.
fn pattern_matches(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    match (pattern.strip_prefix('*'), pattern.strip_suffix('*')) {
        (Some(_), Some(_)) if pattern.len() >= 2 => name.contains(&pattern[1..pattern.len() - 1]),
        (Some(tail), _) => name.ends_with(tail),
        (_, Some(head)) => name.starts_with(head),
        _ => pattern == name,
    }
}

/// Whether two names two patches were given can mean the same class or method. Two different
/// wildcards may or may not meet: only said to when one is `*` or they are written the same.
fn names_meet(a: &str, b: &str) -> bool {
    match (a.contains('*'), b.contains('*')) {
        (false, false) => a == b,
        (true, false) => pattern_matches(a, b),
        (false, true) => pattern_matches(b, a),
        (true, true) => a == b || a == "*" || b == "*",
    }
}

/// One mod's side of an overlap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlapSide {
    pub mod_id: String,
    pub name: String,
    /// Its patches on the class (on the shared methods, when `same_method`).
    pub patches: Vec<JavaPatch>,
}

/// One class two or more of the mods change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overlap {
    pub target_class: String,
    /// The class is in this mod's own jar (a mod patching another mod's class, as the Viewpoint
    /// add-ons do with Viewpoint's): not a class of the game. None: no jar among the mods asked
    /// about has its package, so it is the game's or a library's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_mod: Option<String>,
    /// The method names at least two of the mods change. Empty: they change different methods
    /// of the class.
    pub methods: Vec<String>,
    /// `same_method` or `same_class`.
    pub kind: String,
    /// More than one of the mods replaces one of those methods (`isAdvice = false`): ZombieBuddy
    /// allows one replacement per method, so which one holds depends on the load order.
    pub delegations_meet: bool,
    pub mods: Vec<OverlapSide>,
}

fn sides_can_meet(a: &ModPatches, b: &ModPatches) -> bool {
    // A client-only jar and a server-only jar never run in the same process.
    !matches!((a.side.as_deref(), b.side.as_deref()), (Some("client"), Some("server")) | (Some("server"), Some("client")))
}

/// Where the mods' applied patches meet: one entry per target class two or more mods change,
/// saying which methods they share (none: different methods of the same class).
pub fn overlaps(mods: &[ModPatches]) -> Vec<Overlap> {
    // Every class name written by any mod is a candidate; wildcards join the plain names they match.
    let mut names: BTreeSet<String> = BTreeSet::new();
    for m in mods {
        names.extend(m.applied().filter_map(|p| p.target_class.clone()));
    }
    let mut out: Vec<Overlap> = Vec::new();
    for class in &names {
        if class.contains('*') && names.iter().any(|n| !n.contains('*') && pattern_matches(class, n)) {
            continue; // told under the plain names it matches
        }
        let on_class = |m: &ModPatches| -> Vec<JavaPatch> {
            m.applied().filter(|p| p.target_class.as_deref().is_some_and(|c| names_meet(c, class))).cloned().collect()
        };
        let involved: Vec<(&ModPatches, Vec<JavaPatch>)> = mods.iter().map(|m| (m, on_class(m))).filter(|(_, p)| !p.is_empty()).collect();
        if involved.len() < 2 {
            continue;
        }
        // The methods at least two mods that can run together both name.
        let mut shared: BTreeSet<String> = BTreeSet::new();
        let mut meeting: BTreeSet<&str> = BTreeSet::new();
        let mut class_meeting: BTreeSet<&str> = BTreeSet::new();
        for (i, (a, pa)) in involved.iter().enumerate() {
            for (b, pb) in involved.iter().skip(i + 1) {
                if !sides_can_meet(a, b) {
                    continue;
                }
                class_meeting.insert(a.mod_id.as_str());
                class_meeting.insert(b.mod_id.as_str());
                for x in pa {
                    for y in pb {
                        if let (Some(mx), Some(my)) = (x.target_method.as_deref(), y.target_method.as_deref()) {
                            if names_meet(mx, my) {
                                // The plainer of the two names says which method.
                                shared.insert(if mx.contains('*') { my.to_string() } else { mx.to_string() });
                                meeting.insert(a.mod_id.as_str());
                                meeting.insert(b.mod_id.as_str());
                            }
                        }
                    }
                }
            }
        }
        if class_meeting.len() < 2 {
            continue;
        }
        let same_method = !shared.is_empty();
        let on_shared = |p: &JavaPatch| p.target_method.as_deref().is_some_and(|m| shared.iter().any(|s| names_meet(m, s)));
        let sides: Vec<OverlapSide> = involved
            .iter()
            .filter(|(m, _)| if same_method { meeting.contains(m.mod_id.as_str()) } else { class_meeting.contains(m.mod_id.as_str()) })
            .map(|(m, patches)| OverlapSide {
                mod_id: m.mod_id.clone(),
                name: m.name.clone(),
                patches: patches.iter().filter(|p| !same_method || on_shared(p)).cloned().collect(),
            })
            .collect();
        let delegations_meet = same_method
            && shared.iter().any(|s| {
                sides
                    .iter()
                    .filter(|side| side.patches.iter().any(|p| p.kind == PatchKind::Delegation && p.target_method.as_deref().is_some_and(|m| names_meet(m, s))))
                    .count()
                    > 1
            });
        let owner_mod = (!class.contains('*'))
            .then(|| mods.iter().find(|m| m.own_packages.iter().any(|p| p == package_of(class))).map(|m| m.mod_id.clone()))
            .flatten();
        out.push(Overlap {
            target_class: class.clone(),
            owner_mod,
            methods: shared.into_iter().collect(),
            kind: if same_method { "same_method" } else { "same_class" }.to_string(),
            delegations_meet,
            mods: sides,
        });
    }
    // The game's classes first, shared methods first, then by class.
    out.sort_by(|a, b| (a.owner_mod.is_some(), a.kind != "same_method", &a.target_class).cmp(&(b.owner_mod.is_some(), b.kind != "same_method", &b.target_class)));
    out
}

/// What `java_patches` answers: each Java mod's patches and where they meet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Radar {
    /// Whether ZombieBuddy is among the mods asked about: without it no jar loads at all.
    pub zombiebuddy_in_list: bool,
    pub mods: Vec<ModPatches>,
    pub overlaps: Vec<Overlap>,
}

pub fn radar(user_zomboid_dir: &Path, catalog: &ModCatalog, ids: &[String]) -> Radar {
    let mods = read_mods(user_zomboid_dir, catalog, ids);
    let overlaps = overlaps(&mods);
    let zombiebuddy_in_list = ids.iter().filter_map(|e| catalog.resolve(e.trim())).any(|m| m.id.eq_ignore_ascii_case(ZOMBIEBUDDY_ID));
    Radar { zombiebuddy_in_list, mods, overlaps }
}

/// The patches by target class, for a short list: class -> method names (sorted, no repeats).
pub fn by_class(m: &ModPatches) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for p in m.applied() {
        let methods = out.entry(p.target_class.clone().unwrap_or_default()).or_default();
        if let Some(method) = &p.target_method {
            methods.insert(method.clone());
        }
    }
    out.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect()
}

/// One class a mod shares with other mods of the list, for that mod's own page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedClass {
    pub target_class: String,
    /// The methods they share; empty: the others change other methods of the class.
    pub methods: Vec<String>,
    /// The class is this mod's own (see `Overlap::owner_mod`), not the game's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_mod: Option<String>,
    /// The other mods' names.
    pub mods: Vec<String>,
}

/// The overlaps `mod_id` is part of, each with the other mods in it.
pub fn shared_with(radar: &Radar, mod_id: &str) -> Vec<SharedClass> {
    radar
        .overlaps
        .iter()
        .filter(|o| o.mods.iter().any(|s| s.mod_id == mod_id))
        .map(|o| SharedClass {
            target_class: o.target_class.clone(),
            methods: o.methods.clone(),
            owner_mod: o.owner_mod.clone(),
            mods: o.mods.iter().filter(|s| s.mod_id != mod_id).map(|s| s.name.clone()).collect(),
        })
        .collect()
}

/// A mod's patches with the classes it shares with the other mods of the active list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModJavaView {
    #[serde(flatten)]
    pub patches: ModPatches,
    /// Empty when the mod is not on the active list (nothing of it runs beside the others).
    pub shared: Vec<SharedClass>,
}

// ------------------------------------------------------------------ Mod Health

/// Mod Health's rows for the overlaps on the game's classes, one per class (`java_patch_overlap`):
/// a Warning where two or more mods patch the same method, Info where they patch different
/// methods of one class. Classes of another mod's jar are left out: a mod patching the hooks of
/// the framework it is an add-on of is what those hooks are for (the `java_patches` tool lists
/// them, with `owner_mod`). The text says only what the jars declare. A row belongs to no single
/// mod (`mod_id` is empty, `related` names them all): it is about the combination, and no mod's
/// row in the Mod List is coloured for it.
pub fn overlap_issues(radar: &Radar) -> Vec<crate::catalog::HealthIssue> {
    use crate::catalog::{issue_params, issue_state, HealthIssue, Severity};
    if !radar.zombiebuddy_in_list {
        return Vec::new(); // no jar loads without ZombieBuddy
    }
    radar
        .overlaps
        .iter()
        .filter(|o| o.owner_mod.is_none())
        .map(|o| {
            let names: Vec<&str> = o.mods.iter().map(|s| s.name.as_str()).collect();
            let mods = names.join(", ");
            let by_code = o.mods.iter().any(|s| s.patches.iter().any(|p| p.by_code));
            let by_code_text = if by_code { " Some of these patches are applied by the mod's own code, which may do so only in some cases." } else { "" };
            let same_method = o.kind == "same_method";
            let (severity, variant, message, methods, detail) = if same_method {
                let methods = o.methods.join(", ");
                let replace = if o.delegations_meet {
                    " More than one of them replaces the method rather than adding to it, and ZombieBuddy's own source says only one replacement per method can work."
                } else {
                    ""
                };
                let message = format!(
                    "{} each carry Java that changes the method{} {} of {}. Whether they clash depends on what each patch does, which the Studio cannot tell from the jar: patches that add code to a method stack, one after the other.{}{} Nothing to do unless this part of the game misbehaves; then try without one of them.",
                    mods,
                    if o.methods.len() > 1 { "s" } else { "" },
                    methods,
                    o.target_class,
                    replace,
                    by_code_text
                );
                (Severity::Warning, if o.methods.len() > 1 { "methods" } else { "method" }, message, methods, String::new())
            } else {
                let detail = o
                    .mods
                    .iter()
                    .map(|s| {
                        let methods: BTreeSet<&str> = s.patches.iter().filter_map(|p| p.target_method.as_deref()).collect();
                        format!("{}: {}", s.name, methods.into_iter().collect::<Vec<_>>().join(", "))
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                let message = format!(
                    "{} each carry Java that changes {}, in different methods ({}). Neither changes the method the other changes; whether they get along still depends on what each does.{} Nothing to do.",
                    mods, o.target_class, detail, by_code_text
                );
                (Severity::Info, "class", message, String::new(), detail)
            };
            let mut params = issue_params(&[
                ("variant", variant),
                ("mods", mods.as_str()),
                ("class", o.target_class.as_str()),
                ("methods", methods.as_str()),
                ("detail", detail.as_str()),
                ("replaceKey", if o.delegations_meet { "java_patch_replace" } else { "" }),
                ("byCodeKey", if by_code { "java_patch_by_code" } else { "" }),
            ]);
            params.insert("count".to_string(), o.mods.len().to_string());
            HealthIssue {
                severity,
                state: issue_state(severity, "java_patch_overlap"),
                code: "java_patch_overlap".to_string(),
                mod_id: None,
                message,
                fixes: Vec::new(),
                related: o.mods.iter().map(|s| s.mod_id.clone()).collect(),
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

    // ---- a class file built by hand, the way javac writes one

    #[derive(Default)]
    struct ClassBuilder {
        pool: Vec<u8>,
        count: u16,
    }

    impl ClassBuilder {
        fn utf8(&mut self, s: &str) -> u16 {
            self.pool.push(1);
            self.pool.extend((s.len() as u16).to_be_bytes());
            self.pool.extend(s.as_bytes());
            self.count += 1;
            self.count
        }
        fn int(&mut self, v: i32) -> u16 {
            self.pool.push(3);
            self.pool.extend(v.to_be_bytes());
            self.count += 1;
            self.count
        }
        /// A Long takes two slots, as in real pools: what comes after must still be found.
        fn long(&mut self) -> u16 {
            self.pool.push(5);
            self.pool.extend(7u64.to_be_bytes());
            self.count += 2;
            self.count - 1
        }
        fn class(&mut self, name: &str) -> u16 {
            let n = self.utf8(name);
            self.pool.push(7);
            self.pool.extend(n.to_be_bytes());
            self.count += 1;
            self.count
        }
    }

    enum V<'a> {
        S(&'a str),
        Z(bool),
        C(&'a str),
        /// An array of strings: read past, not kept.
        Arr(&'a [&'a str]),
    }

    fn ann(b: &mut ClassBuilder, type_desc: &str, values: &[(&str, V)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(b.utf8(type_desc).to_be_bytes());
        out.extend((values.len() as u16).to_be_bytes());
        for (name, v) in values {
            out.extend(b.utf8(name).to_be_bytes());
            match v {
                V::S(s) => {
                    out.push(b's');
                    out.extend(b.utf8(s).to_be_bytes());
                }
                V::Z(z) => {
                    out.push(b'Z');
                    out.extend(b.int(*z as i32).to_be_bytes());
                }
                V::C(c) => {
                    out.push(b'c');
                    out.extend(b.utf8(c).to_be_bytes());
                }
                V::Arr(items) => {
                    out.push(b'[');
                    out.extend((items.len() as u16).to_be_bytes());
                    for s in *items {
                        out.push(b's');
                        out.extend(b.utf8(s).to_be_bytes());
                    }
                }
            }
        }
        out
    }

    fn attribute(b: &mut ClassBuilder, name: &str, body: Vec<u8>) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(b.utf8(name).to_be_bytes());
        out.extend((body.len() as u32).to_be_bytes());
        out.extend(body);
        out
    }

    fn annotations_attribute(b: &mut ClassBuilder, anns: Vec<Vec<u8>>) -> Vec<u8> {
        let mut body = (anns.len() as u16).to_be_bytes().to_vec();
        for a in anns {
            body.extend(a);
        }
        attribute(b, "RuntimeVisibleAnnotations", body)
    }

    /// One parameter list: the annotations of each parameter.
    fn parameter_annotations_attribute(b: &mut ClassBuilder, params: Vec<Vec<Vec<u8>>>) -> Vec<u8> {
        let mut body = vec![params.len() as u8];
        for anns in params {
            body.extend((anns.len() as u16).to_be_bytes());
            for a in anns {
                body.extend(a);
            }
        }
        attribute(b, "RuntimeVisibleParameterAnnotations", body)
    }

    struct Method {
        name: &'static str,
        attrs: Vec<Vec<u8>>,
    }

    fn method(b: &mut ClassBuilder, m: Method) -> Vec<u8> {
        let mut out = vec![0, 9]; // public static
        out.extend(b.utf8(m.name).to_be_bytes());
        out.extend(b.utf8("()V").to_be_bytes());
        out.extend((m.attrs.len() as u16).to_be_bytes());
        for a in m.attrs {
            out.extend(a);
        }
        out
    }

    fn finish(mut b: ClassBuilder, name: &str, refs: &[&str], methods: Vec<Vec<u8>>, class_attrs: Vec<Vec<u8>>) -> Vec<u8> {
        let this = b.class(name);
        let sup = b.class("java/lang/Object");
        for r in refs {
            b.class(r);
        }
        // A field with an attribute of its own, to be read past.
        let field_name = b.utf8("COUNT");
        let field_desc = b.utf8("I");
        let field_attr = attribute(&mut b, "Synthetic", Vec::new());
        let mut out = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 61];
        out.extend((b.count + 1).to_be_bytes());
        out.extend(&b.pool);
        out.extend([0, 0x21]);
        out.extend(this.to_be_bytes());
        out.extend(sup.to_be_bytes());
        out.extend([0, 0]); // interfaces
        out.extend([0, 1, 0, 8]); // one field, static
        out.extend(field_name.to_be_bytes());
        out.extend(field_desc.to_be_bytes());
        out.extend([0, 1]);
        out.extend(field_attr);
        out.extend((methods.len() as u16).to_be_bytes());
        for m in methods {
            out.extend(m);
        }
        out.extend((class_attrs.len() as u16).to_be_bytes());
        for a in class_attrs {
            out.extend(a);
        }
        out
    }

    const PATCH: &str = "Lme/zed_0xff/zombie_buddy/Patch;";

    /// `@Patch(className, methodName)` with an `@Patch.OnEnter` method and nothing else.
    fn simple_patch(class: &str, target: &str, method_name: &str) -> Vec<u8> {
        let mut b = ClassBuilder::default();
        b.long();
        let enter = ann(&mut b, "Lme/zed_0xff/zombie_buddy/Patch$OnEnter;", &[]);
        let enter_attr = annotations_attribute(&mut b, vec![enter]);
        let m = method(&mut b, Method { name: "enter", attrs: vec![enter_attr] });
        let patch = ann(&mut b, PATCH, &[("className", V::S(target)), ("methodName", V::S(method_name))]);
        let class_attr = annotations_attribute(&mut b, vec![patch]);
        finish(b, class, &[], vec![m], vec![class_attr])
    }

    fn delegation_patch(class: &str, target: &str, method_name: &str) -> Vec<u8> {
        let mut b = ClassBuilder::default();
        let patch = ann(&mut b, PATCH, &[("className", V::S(target)), ("methodName", V::S(method_name)), ("isAdvice", V::Z(false))]);
        let class_attr = annotations_attribute(&mut b, vec![patch]);
        finish(b, class, &[], vec![], vec![class_attr])
    }

    fn plain_class(class: &str, refs: &[&str]) -> Vec<u8> {
        finish(ClassBuilder::default(), class, refs, vec![], vec![])
    }

    fn jar(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, bytes) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut zip, bytes).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn a_patch_annotation_gives_the_target_class_and_method() {
        let class = parse_class(&simple_patch("pz/mymod/Patch_Rand", "zombie.core.random.Rand", "Next")).unwrap();
        assert_eq!(class.name, "pz/mymod/Patch_Rand");
        assert_eq!(class.annotations.len(), 1);
        assert_eq!(class.annotations[0].str("className"), Some("zombie.core.random.Rand"));
        let none = BTreeSet::new();
        let p = patch_of(&class, &["pz.mymod".to_string()], None, &none).unwrap();
        assert_eq!(p.target_class.as_deref(), Some("zombie.core.random.Rand"));
        assert_eq!(p.target_method.as_deref(), Some("Next"));
        assert_eq!(p.patch_class, "pz.mymod.Patch_Rand");
        assert_eq!(p.kind, PatchKind::Advice);
        assert_eq!(p.effects, vec!["enter"]);
        assert_eq!(p.not_applied, None);
        // A class without the annotation is not a patch, whatever its name or strings say.
        let mut b = ClassBuilder::default();
        b.utf8(PATCH);
        b.utf8("zombie.core.random.Rand");
        let named = finish(b, "pz/mymod/Patch_Named", &[], vec![], vec![]);
        assert!(patch_of(&parse_class(&named).unwrap(), &["pz.mymod".to_string()], None, &none).is_none());
        assert!(parse_class(b"not a class").is_none());
    }

    #[test]
    fn what_the_advice_methods_declare_is_read_from_their_annotations() {
        let mut b = ClassBuilder::default();
        // @Patch.OnEnter(skipOn = true) static boolean enter(@Patch.Argument(value = 0, readOnly = false) x)
        let enter = ann(&mut b, "Lme/zed_0xff/zombie_buddy/Patch$OnEnter;", &[("skipOn", V::Z(true))]);
        let enter_attr = annotations_attribute(&mut b, vec![enter]);
        let arg = ann(&mut b, "Lme/zed_0xff/zombie_buddy/Patch$Argument;", &[("readOnly", V::Z(false))]);
        let enter_params = parameter_annotations_attribute(&mut b, vec![vec![arg]]);
        let m1 = method(&mut b, Method { name: "enter", attrs: vec![enter_attr, enter_params] });
        // @Patch.OnExit static void exit(@Patch.This self, @Patch.Return(readOnly = false) r)
        let exit = ann(&mut b, "Lme/zed_0xff/zombie_buddy/Patch$OnExit;", &[("onThrowable", V::C("Ljava/lang/Throwable;"))]);
        let other = ann(&mut b, "Lsome/Other;", &[("names", V::Arr(&["a", "b"]))]);
        let exit_attr = annotations_attribute(&mut b, vec![other, exit]);
        let this = ann(&mut b, "Lme/zed_0xff/zombie_buddy/Patch$This;", &[]);
        let ret = ann(&mut b, "Lme/zed_0xff/zombie_buddy/Patch$Return;", &[("readOnly", V::Z(false))]);
        let exit_params = parameter_annotations_attribute(&mut b, vec![vec![this], vec![ret]]);
        let m2 = method(&mut b, Method { name: "exit", attrs: vec![exit_attr, exit_params] });
        let patch = ann(&mut b, PATCH, &[("className", V::S("zombie.characters.IsoPlayer")), ("methodName", V::S("update")), ("warmUp", V::Z(true))]);
        let class_attr = annotations_attribute(&mut b, vec![patch]);
        let bytes = finish(b, "pz/mymod/Outer$Inner", &[], vec![m1, m2], vec![class_attr]);
        let none = BTreeSet::new();
        let p = patch_of(&parse_class(&bytes).unwrap(), &["pz.mymod".to_string()], None, &none).unwrap();
        assert_eq!(p.effects, vec!["enter", "exit", "skip", "return", "args"]);
        assert_eq!(p.patch_class, "pz.mymod.Outer$Inner");

        // A read-only @Return and ByteBuddy's own annotations.
        let mut b = ClassBuilder::default();
        let enter = ann(&mut b, "Lnet/bytebuddy/asm/Advice$OnMethodEnter;", &[("skipOn", V::C("Lnet/bytebuddy/asm/Advice$OnNonDefaultValue;"))]);
        let enter_attr = annotations_attribute(&mut b, vec![enter]);
        let ret = ann(&mut b, "Lme/zed_0xff/zombie_buddy/Patch$Return;", &[]);
        let params = parameter_annotations_attribute(&mut b, vec![vec![ret]]);
        let m = method(&mut b, Method { name: "enter", attrs: vec![enter_attr, params] });
        let patch = ann(&mut b, PATCH, &[("className", V::S("zombie.X")), ("methodName", V::S("y"))]);
        let class_attr = annotations_attribute(&mut b, vec![patch]);
        let bytes = finish(b, "pz/mymod/P", &[], vec![m], vec![class_attr]);
        let p = patch_of(&parse_class(&bytes).unwrap(), &["pz.mymod".to_string()], None, &none).unwrap();
        assert_eq!(p.effects, vec!["enter", "skip"]);
    }

    #[test]
    fn a_jar_gives_the_patches_zombiebuddy_applies_and_names_the_ones_it_does_not() {
        let bytes = jar(&[
            ("pz/mymod/Patch_Rand.class", simple_patch("pz/mymod/Patch_Rand", "zombie.core.random.Rand", "Next")),
            ("pz/mymod/Patch_Door.class", delegation_patch("pz/mymod/Patch_Door", "zombie.iso.objects.IsoDoor", "<init>")),
            // ZombieBuddy scans the package itself, not what is under it.
            ("pz/mymod/sub/Patch_Deep.class", simple_patch("pz/mymod/sub/Patch_Deep", "zombie.iso.IsoCell", "update")),
            ("pz/mymod/Patch_ZB.class", simple_patch("pz/mymod/Patch_ZB", "me.zed_0xff.zombie_buddy.Loader", "loadJar")),
            ("pz/mymod/Patch_Exposer.class", simple_patch("pz/mymod/Patch_Exposer", "zombie.Lua.LuaManager$Exposer", "exposeAll")),
            ("pz/mymod/Main.class", plain_class("pz/mymod/Main", &["java/lang/instrument/Instrumentation"])),
            // A library bundled in the jar says nothing of the mod's own code.
            ("net/bytebuddy/Thing.class", plain_class("net/bytebuddy/Thing", &["org/objectweb/asm/ClassWriter"])),
            ("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\n".to_vec()),
            ("pz/mymod/broken.class", b"\xCA\xFE\xBA\xBE".to_vec()),
        ]);
        let read = read_jar(&bytes, "pz.mymod").unwrap();
        assert_eq!(read.classes, 7);
        assert_eq!(read.patches_in_code, vec!["instrumentation"]);
        let said: Vec<(String, String, PatchKind, Option<String>)> = read
            .patches
            .iter()
            .map(|p| (p.target_class.clone().unwrap(), p.target_method.clone().unwrap(), p.kind, p.not_applied.clone()))
            .collect();
        assert_eq!(
            said,
            vec![
                ("me.zed_0xff.zombie_buddy.Loader".into(), "loadJar".into(), PatchKind::Advice, Some("zombiebuddy_class".into())),
                ("zombie.Lua.LuaManager$Exposer".into(), "exposeAll".into(), PatchKind::Advice, Some("exposer".into())),
                ("zombie.core.random.Rand".into(), "Next".into(), PatchKind::Advice, None),
                ("zombie.iso.IsoCell".into(), "update".into(), PatchKind::Advice, Some("outside_package".into())),
                ("zombie.iso.objects.IsoDoor".into(), "<init>".into(), PatchKind::Delegation, None),
            ]
        );
        assert!(read_jar(b"not a zip", "pz.mymod").is_err());

        // ZombieBuddy's own jar: .patches always, .patches.experimental only when switched on.
        let zb = jar(&[
            ("me/zed_0xff/zombie_buddy/patches/Patch_Core.class", simple_patch("me/zed_0xff/zombie_buddy/patches/Patch_Core", "zombie.core.Core", "init")),
            (
                "me/zed_0xff/zombie_buddy/patches/experimental/Patch_ServerMap.class",
                simple_patch("me/zed_0xff/zombie_buddy/patches/experimental/Patch_ServerMap", "zombie.network.ServerMap", "preupdate"),
            ),
        ]);
        let read = read_jar(&zb, ZB_PACKAGE).unwrap();
        let said: Vec<(Option<String>, Option<String>)> = read.patches.iter().map(|p| (p.target_class.clone(), p.not_applied.clone())).collect();
        assert_eq!(said, vec![(Some("zombie.core.Core".into()), None), (Some("zombie.network.ServerMap".into()), Some("experimental".into()))]);
    }

    fn patch(class: &str, method: &str, kind: PatchKind) -> JavaPatch {
        JavaPatch {
            target_class: Some(class.to_string()),
            target_method: Some(method.to_string()),
            patch_class: format!("p.Patch_{}", method),
            kind,
            effects: vec![],
            by_code: false,
            not_applied: None,
        }
    }

    fn a_mod(id: &str, patches: Vec<JavaPatch>) -> ModPatches {
        ModPatches { mod_id: id.to_string(), name: id.to_string(), jar: Some("x.jar".into()), package: Some("p".into()), patches, ..Default::default() }
    }

    #[test]
    fn mods_meet_on_a_method_or_only_on_a_class() {
        use PatchKind::*;
        let a = a_mod("A", vec![patch("zombie.X", "update", Advice), patch("zombie.X", "render", Advice), patch("zombie.Only", "a", Advice)]);
        let b = a_mod("B", vec![patch("zombie.X", "update", Advice), patch("zombie.Y", "load", Advice)]);
        let c = a_mod("C", vec![patch("zombie.Y", "save", Advice), patch("zombie.X", "tick", Advice)]);
        let found = overlaps(&[a.clone(), b.clone(), c.clone()]);
        assert_eq!(found.len(), 2);
        // zombie.X: A and B share update; C is on another method and is not named with them.
        assert_eq!((found[0].target_class.as_str(), found[0].kind.as_str()), ("zombie.X", "same_method"));
        assert_eq!(found[0].methods, vec!["update"]);
        assert_eq!(found[0].mods.iter().map(|s| s.mod_id.as_str()).collect::<Vec<_>>(), vec!["A", "B"]);
        assert_eq!(found[0].mods[0].patches.len(), 1, "only the patches on the shared method");
        assert!(!found[0].delegations_meet);
        // zombie.Y: B and C, different methods.
        assert_eq!((found[1].target_class.as_str(), found[1].kind.as_str()), ("zombie.Y", "same_class"));
        assert!(found[1].methods.is_empty());
        assert_eq!(found[1].mods.iter().map(|s| s.mod_id.as_str()).collect::<Vec<_>>(), vec!["B", "C"]);

        // One mod patching a method twice does not meet itself.
        assert!(overlaps(&[a_mod("A", vec![patch("zombie.X", "update", Advice), patch("zombie.X", "update", Advice)])]).is_empty());

        // A patch ZombieBuddy does not apply, or one with no readable target, meets nothing.
        let mut off = patch("zombie.X", "update", Advice);
        off.not_applied = Some("outside_package".into());
        let mut unknown = patch("zombie.X", "update", Advice);
        unknown.target_class = None;
        assert!(overlaps(&[a_mod("A", vec![patch("zombie.X", "update", Advice)]), a_mod("D", vec![off, unknown])]).is_empty());

        // Two replacements of one method: said so.
        let found = overlaps(&[a_mod("A", vec![patch("zombie.X", "update", Delegation)]), a_mod("B", vec![patch("zombie.X", "update", Delegation)])]);
        assert!(found[0].delegations_meet);
        let found = overlaps(&[a_mod("A", vec![patch("zombie.X", "update", Delegation)]), a_mod("B", vec![patch("zombie.X", "update", Advice)])]);
        assert!(!found[0].delegations_meet);

        // A client-only jar and a server-only jar never run together.
        let mut client = a_mod("A", vec![patch("zombie.X", "update", Advice)]);
        client.side = Some("client".into());
        let mut server = a_mod("B", vec![patch("zombie.X", "update", Advice)]);
        server.side = Some("server".into());
        assert!(overlaps(&[client.clone(), server]).is_empty());
        assert_eq!(overlaps(&[client, a_mod("B", vec![patch("zombie.X", "update", Advice)])]).len(), 1);
    }

    #[test]
    fn wildcards_meet_the_names_they_match() {
        use PatchKind::*;
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("zombie.iso.*", "zombie.iso.IsoCell"));
        assert!(pattern_matches("*Cell", "zombie.iso.IsoCell"));
        assert!(pattern_matches("*Iso*", "zombie.iso.IsoCell"));
        assert!(!pattern_matches("zombie.ui.*", "zombie.iso.IsoCell"));
        assert!(names_meet("<init>", "<init>") && !names_meet("<init>", "init"));
        assert!(names_meet("get*", "getX") && names_meet("getX", "get*") && !names_meet("get*", "set*"));

        let found = overlaps(&[a_mod("A", vec![patch("zombie.iso.IsoCell", "update", Advice)]), a_mod("B", vec![patch("zombie.iso.*", "*", Advice)])]);
        assert_eq!(found.len(), 1, "the wildcard is told under the class it matches: {:?}", found);
        assert_eq!((found[0].target_class.as_str(), found[0].kind.as_str()), ("zombie.iso.IsoCell", "same_method"));
        assert_eq!(found[0].methods, vec!["update"]);
    }

    #[test]
    fn mod_info_values_are_read_the_way_zombiebuddy_reads_them() {
        let text = "\u{feff}id=X\r\nname=X\r\njavaJarFile=media/java/a.jar\r\njavaPkgName= pz.x \r\njavaJarFile2=media/java/b.jar\r\njavaJarFile=media/java/c.jar\r\n";
        let any = |_: &str| true;
        assert_eq!(info_value(text, "javaJarFile", any).as_deref(), Some("media/java/a.jar"));
        assert_eq!(info_value(text, "javaPkgName", any).as_deref(), Some("pz.x"));
        assert_eq!(info_value("JAVAJARFILE=x.jar\n", "javaJarFile", any).as_deref(), Some("x.jar"));
        assert_eq!(info_value("description=javaJarFile=x.jar\n", "javaJarFile", any), None);
        assert_eq!(info_value("javaJarFile=\njavaJarFile=a.zip\njavaJarFile=b.jar\n", "javaJarFile", |v| v.ends_with(".jar")).as_deref(), Some("b.jar"));
    }

    #[test]
    fn a_mods_jar_is_read_once_and_again_when_it_changes() {
        let base = std::env::temp_dir().join(format!("pzms-javapatches-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let workshop = base.join("108600");
        let zomboid = base.join("Zomboid");
        std::fs::create_dir_all(&workshop).unwrap();
        let root = zomboid.join("mods").join("JavaMod");
        let java = root.join("42").join("media").join("java");
        std::fs::create_dir_all(&java).unwrap();
        std::fs::write(root.join("42").join("mod.info"), "id=JavaMod\nname=Java Mod\njavaJarFile=media/java/client/JavaMod.jar\njavaPkgName=pz.mymod\n").unwrap();
        std::fs::create_dir_all(java.join("client")).unwrap();
        let jar_path = java.join("client").join("JavaMod.jar");
        std::fs::write(&jar_path, jar(&[("pz/mymod/Patch_Rand.class", simple_patch("pz/mymod/Patch_Rand", "zombie.core.random.Rand", "Next"))])).unwrap();
        let plain = zomboid.join("mods").join("LuaMod");
        std::fs::create_dir_all(plain.join("42")).unwrap();
        std::fs::write(plain.join("42").join("mod.info"), "id=LuaMod\nname=Lua Mod\n").unwrap();
        let nopkg = zomboid.join("mods").join("NoPkg");
        std::fs::create_dir_all(nopkg.join("42")).unwrap();
        std::fs::write(nopkg.join("42").join("mod.info"), "id=NoPkg\nname=No Pkg\njavaJarFile=media/java/gone.jar\n").unwrap();

        let catalog = crate::catalog::build_catalog(&workshop.to_string_lossy(), &zomboid.to_string_lossy(), None);
        let ids = vec!["JavaMod".to_string(), "LuaMod".to_string(), "NoPkg".to_string(), "JavaMod".to_string()];
        let mods = read_mods(&zomboid, &catalog, &ids);
        assert_eq!(mods.iter().map(|m| m.mod_id.as_str()).collect::<Vec<_>>(), vec!["JavaMod", "NoPkg"], "a mod without a jar is not a Java mod");
        assert_eq!(mods[0].target_classes, vec!["zombie.core.random.Rand"]);
        assert_eq!(mods[0].side.as_deref(), Some("client"));
        assert_eq!(mods[0].unknown, None);
        assert_eq!(mods[1].unknown.as_deref(), Some("no_package"), "what cannot be read is said, not guessed");
        assert!(mods[1].patches.is_empty());
        assert!(cache_file(&zomboid, "JavaMod").is_file());
        assert_eq!(by_class(&mods[0]).get("zombie.core.random.Rand"), Some(&vec!["Next".to_string()]));

        // The jar is rebuilt: read again.
        std::fs::write(
            &jar_path,
            jar(&[
                ("pz/mymod/Patch_Rand.class", simple_patch("pz/mymod/Patch_Rand", "zombie.core.random.Rand", "Next")),
                ("pz/mymod/Patch_Cell.class", simple_patch("pz/mymod/Patch_Cell", "zombie.iso.IsoCell", "update")),
            ]),
        )
        .unwrap();
        let again = read_mods(&zomboid, &catalog, &ids);
        assert_eq!(again[0].target_classes, vec!["zombie.core.random.Rand", "zombie.iso.IsoCell"]);

        // The jar is gone: unknown, not "no patches".
        std::fs::remove_file(&jar_path).unwrap();
        assert_eq!(read_mods(&zomboid, &catalog, &ids)[0].unknown.as_deref(), Some("jar_missing"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_jar_in_the_version_folder_is_found_from_the_common_mod_info() {
        // As Viewpoint ships it: common/mod.info names media/java/client/X.jar, which sits in 42/.
        let base = std::env::temp_dir().join(format!("pzms-javapatches-common-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let workshop = base.join("108600");
        let zomboid = base.join("Zomboid");
        std::fs::create_dir_all(&workshop).unwrap();
        let root = zomboid.join("mods").join("Framework");
        std::fs::create_dir_all(root.join("common")).unwrap();
        std::fs::write(root.join("common").join("mod.info"), "id=Framework\nname=Framework\njavaJarFile=media/java/client/Framework.jar\njavaPkgName=fw\n").unwrap();
        let java = root.join("42").join("media").join("java").join("client");
        std::fs::create_dir_all(&java).unwrap();
        std::fs::write(
            java.join("Framework.jar"),
            jar(&[
                ("fw/Patch_Cell.class", simple_patch("fw/Patch_Cell", "zombie.iso.IsoCell", "update")),
                ("fw/Hooks.class", plain_class("fw/Hooks", &[])),
                // Asked for by the jar's own code: fw.Main refers to PatchEngine and names "fwextra".
                ("fwextra/Patch_Door.class", simple_patch("fwextra/Patch_Door", "zombie.iso.objects.IsoDoor", "update")),
                ("fw/Main.class", {
                    let mut b = ClassBuilder::default();
                    let text = b.utf8("fwextra");
                    b.pool.push(8);
                    b.pool.extend(text.to_be_bytes());
                    b.count += 1;
                    finish(b, "fw/Main", &["me/zed_0xff/zombie_buddy/PatchEngine"], vec![], vec![])
                }),
                // Named by no one: ZombieBuddy never sees it.
                ("fwunused/Patch_World.class", simple_patch("fwunused/Patch_World", "zombie.iso.IsoWorld", "update")),
            ]),
        )
        .unwrap();
        // An add-on that patches the framework's own class, and the game's.
        let addon = zomboid.join("mods").join("Addon");
        std::fs::create_dir_all(addon.join("42").join("media").join("java")).unwrap();
        std::fs::write(addon.join("42").join("mod.info"), "id=Addon\nname=Addon\njavaJarFile=media/java/Addon.jar\njavaPkgName=addon\n").unwrap();
        let addon_jar = |files: &[(&str, Vec<u8>)]| std::fs::write(addon.join("42").join("media").join("java").join("Addon.jar"), jar(files)).unwrap();
        addon_jar(&[
            ("addon/Patch_Hooks.class", simple_patch("addon/Patch_Hooks", "fw.Hooks", "frame")),
            ("addon/Patch_Cell.class", simple_patch("addon/Patch_Cell", "zombie.iso.IsoCell", "update")),
            ("addon/Patch_Door.class", simple_patch("addon/Patch_Door", "zombie.iso.objects.IsoDoor", "open")),
        ]);
        let other = zomboid.join("mods").join("Other");
        std::fs::create_dir_all(other.join("42").join("media").join("java")).unwrap();
        std::fs::write(other.join("42").join("mod.info"), "id=Other\nname=Other\njavaJarFile=media/java/Other.jar\njavaPkgName=other\n").unwrap();
        std::fs::write(other.join("42").join("media").join("java").join("Other.jar"), jar(&[("other/Patch_Hooks.class", simple_patch("other/Patch_Hooks", "fw.Hooks", "frame"))])).unwrap();
        std::fs::create_dir_all(zomboid.join("mods").join("ZombieBuddy").join("42")).unwrap();
        std::fs::write(zomboid.join("mods").join("ZombieBuddy").join("42").join("mod.info"), "id=ZombieBuddy\nname=ZombieBuddy\n").unwrap();

        let catalog = crate::catalog::build_catalog(&workshop.to_string_lossy(), &zomboid.to_string_lossy(), None);
        let ids: Vec<String> = ["ZombieBuddy", "Framework", "Addon", "Other"].iter().map(|s| s.to_string()).collect();
        let r = radar(&zomboid, &catalog, &ids);
        assert!(r.zombiebuddy_in_list);
        let fw = &r.mods[0];
        assert_eq!((fw.mod_id.as_str(), fw.unknown.as_deref(), fw.side.as_deref()), ("Framework", None, Some("client")));
        assert_eq!(fw.own_packages, vec!["fw", "fwextra", "fwunused"]);
        let said: Vec<(&str, bool, Option<&str>)> = fw.patches.iter().map(|p| (p.patch_class.as_str(), p.by_code, p.not_applied.as_deref())).collect();
        assert_eq!(said, vec![("fw.Patch_Cell", false, None), ("fwunused.Patch_World", false, Some("outside_package")), ("fwextra.Patch_Door", true, None)]);
        assert_eq!(fw.target_classes, vec!["zombie.iso.IsoCell", "zombie.iso.objects.IsoDoor"]);

        // The game's classes first: IsoCell.update (same method), IsoDoor (different methods),
        // then the framework's own class.
        let told: Vec<(&str, &str, Option<&str>)> = r.overlaps.iter().map(|o| (o.target_class.as_str(), o.kind.as_str(), o.owner_mod.as_deref())).collect();
        assert_eq!(
            told,
            vec![("zombie.iso.IsoCell", "same_method", None), ("zombie.iso.objects.IsoDoor", "same_class", None), ("fw.Hooks", "same_method", Some("Framework"))]
        );

        // Mod Health: the game's classes only; same method a warning, same class info, and the
        // words claim no conflict.
        let issues = overlap_issues(&r);
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().all(|i| i.code == "java_patch_overlap" && i.state == crate::catalog::IssueState::Info));
        assert_eq!(issues[0].severity, crate::catalog::Severity::Warning);
        assert_eq!(issues[0].params.get("variant").map(String::as_str), Some("method"));
        assert_eq!((issues[0].mod_id.as_deref(), issues[0].related.clone()), (None, vec!["Framework".to_string(), "Addon".to_string()]));
        assert_eq!(shared_with(&r, "Addon"), vec![
            SharedClass { target_class: "zombie.iso.IsoCell".into(), methods: vec!["update".into()], owner_mod: None, mods: vec!["Framework".into()] },
            SharedClass { target_class: "zombie.iso.objects.IsoDoor".into(), methods: vec![], owner_mod: None, mods: vec!["Framework".into()] },
            SharedClass { target_class: "fw.Hooks".into(), methods: vec!["frame".into()], owner_mod: Some("Framework".into()), mods: vec!["Other".into()] },
        ]);
        assert!(shared_with(&r, "LuaMod").is_empty());
        assert!(issues[0].message.starts_with("Framework, Addon each carry Java that changes the method update of zombie.iso.IsoCell."), "{}", issues[0].message);
        assert_eq!(issues[1].severity, crate::catalog::Severity::Info);
        assert_eq!(issues[1].params.get("variant").map(String::as_str), Some("class"));
        assert_eq!(issues[1].params.get("detail").map(String::as_str), Some("Framework: update; Addon: open"));
        assert_eq!(issues[1].params.get("byCodeKey").map(String::as_str), Some("java_patch_by_code"));
        for i in &issues {
            let lower = i.message.to_lowercase();
            assert!(!lower.contains("conflict") && !lower.contains("incompatible") && !lower.contains("break"), "{}", i.message);
        }

        // Without ZombieBuddy on the list no jar loads: nothing to tell.
        let without = radar(&zomboid, &catalog, &ids[1..]);
        assert_eq!(without.overlaps.len(), 3);
        assert!(overlap_issues(&without).is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Live: `cargo test --lib -- --ignored live_java_patches --nocapture`. Reads this PC's
    /// active list and compares with what ZombieBuddy logged in console.txt.
    #[test]
    #[ignore]
    fn live_java_patches() {
        let paths = crate::vfs::studio_paths();
        let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
        let active = crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default();
        let scratch = std::env::temp_dir().join(format!("pzms-javapatches-live-{}", std::process::id()));
        let r = radar(&scratch, &catalog, &active);
        println!("zombiebuddy_in_list: {}", r.zombiebuddy_in_list);
        for m in &r.mods {
            println!(
                "MOD {} | pkg {:?} | side {:?} | classes read {} | patches {} (applied {}) | target classes {} | in code {:?} | unknown {:?}",
                m.mod_id,
                m.package,
                m.side,
                m.classes_read,
                m.patches.len(),
                m.applied().count(),
                m.target_classes.len(),
                m.patches_in_code,
                m.unknown
            );
            let mut why: BTreeMap<String, usize> = BTreeMap::new();
            for p in m.patches.iter().filter(|p| p.not_applied.is_some()) {
                *why.entry(p.not_applied.clone().unwrap()).or_default() += 1;
            }
            if !why.is_empty() {
                println!("    not applied: {:?}", why);
            }
        }
        for o in &r.overlaps {
            println!(
                "OVERLAP {} owner={:?} {} {:?} delegations_meet={} :: {}",
                o.kind,
                o.owner_mod,
                o.target_class,
                o.methods,
                o.delegations_meet,
                o.mods.iter().map(|s| s.mod_id.clone()).collect::<Vec<_>>().join(" + ")
            );
        }
        let issues = overlap_issues(&r);
        println!(
            "HEALTH: {} rows ({} warning, {} info); overlaps on other mods' classes left to the tool: {}",
            issues.len(),
            issues.iter().filter(|i| i.severity == crate::catalog::Severity::Warning).count(),
            issues.iter().filter(|i| i.severity == crate::catalog::Severity::Info).count(),
            r.overlaps.iter().filter(|o| o.owner_mod.is_some()).count()
        );
        for i in issues.iter().take(2).chain(issues.iter().rev().take(1)) {
            println!("  e.g. {}", i.message);
        }
        // Against the log of the last session, for the jars that session loaded (its list may
        // differ from the active one): every "Found patch class", "patching C.m", "Transformed: C".
        let log = std::fs::read(Path::new(&paths.user_zomboid_dir).join("console.txt")).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
        let mut log_packages: BTreeSet<String> = BTreeSet::new();
        let mut log_found: BTreeSet<String> = BTreeSet::new();
        let mut log_patching: BTreeSet<String> = BTreeSet::new();
        let mut log_transformed: BTreeSet<String> = BTreeSet::new();
        for line in log.lines() {
            if let Some(rest) = line.split("[ZB] Found patch class: ").nth(1) {
                log_found.insert(rest.trim().to_string());
            } else if let Some(rest) = line.split("[ZB] patching ").nth(1) {
                log_patching.insert(rest.split(" with ").next().unwrap_or("").trim().to_string());
            } else if let Some(rest) = line.split("[ZB] Transformed: ").nth(1) {
                log_transformed.insert(rest.split(' ').next().unwrap_or("").trim().to_string());
            } else if let Some(rest) = line.split(" classes in package ").nth(1) {
                log_packages.insert(rest.trim().to_string());
            }
        }
        // Every installed Java mod whose package the session scanned, on the list or not.
        let all_ids: Vec<String> = catalog.mods.iter().filter(|m| m.shadowed_by.is_none()).map(|m| m.id.clone()).collect();
        let session: Vec<ModPatches> = read_mods(&scratch, &catalog, &all_ids)
            .into_iter()
            .filter(|m| m.package.as_deref().is_some_and(|p| log_packages.contains(p) || p == ZB_PACKAGE))
            .collect();
        println!("SESSION: {} packages scanned in the log; {} installed Java mods match them: {:?}", log_packages.len(), session.len(), session.iter().map(|m| m.mod_id.as_str()).collect::<Vec<_>>());
        let in_session = |p: &JavaPatch| log_packages.contains(package_of(&p.patch_class)) || package_of(&p.patch_class) == "me.zed_0xff.zombie_buddy.patches";
        let read_found: BTreeSet<String> = session.iter().flat_map(|m| m.patches.iter().filter(|p| in_session(p)).map(|p| p.patch_class.clone())).collect();
        let read_patching: BTreeSet<String> = session
            .iter()
            .flat_map(|m| m.patches.iter().filter(|p| in_session(p) && p.target_class.is_some()).map(|p| format!("{}.{}", p.target_class.clone().unwrap(), p.target_method.clone().unwrap_or_default())))
            .collect();
        let read_classes: BTreeSet<String> = session.iter().flat_map(|m| m.patches.iter().filter(|p| in_session(p)).filter_map(|p| p.target_class.clone())).collect();
        println!("LOG found patch classes: {} | reader: {}", log_found.len(), read_found.len());
        println!("  in log, not read: {:?}", log_found.difference(&read_found).collect::<Vec<_>>());
        println!("  read, not in log: {:?}", read_found.difference(&log_found).collect::<Vec<_>>());
        println!("LOG patching class.method: {} | reader: {}", log_patching.len(), read_patching.len());
        println!("  in log, not read: {:?}", log_patching.difference(&read_patching).collect::<Vec<_>>());
        println!("  read, not in log (class not loaded in that session, or refused): {}", read_patching.difference(&log_patching).count());
        println!("LOG transformed classes: {} | reader target classes: {}", log_transformed.len(), read_classes.len());
        println!("  transformed, not read: {:?}", log_transformed.difference(&read_classes).collect::<Vec<_>>());
        let _ = std::fs::remove_dir_all(&scratch);
    }
}
