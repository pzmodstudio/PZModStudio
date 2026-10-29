//! What the game's script files define, and who defines the same thing twice.
//!
//! Two mods shipping the same file at the same path is a conflict the Studio has always found,
//! because the game reads only one of the two copies. The quieter conflict is two mods defining
//! the same **thing** in differently named files: `item Base.Axe` in one mod's `items.txt` and
//! again in another's `myitems.txt`. No file collides, nothing is reported anywhere, and the game
//! simply keeps whichever definition was read last. That is how a patch mod is meant to work, and
//! also how two mods silently undo each other.
//!
//! So the scripts are parsed for what they declare. The parser walks braces rather than lines,
//! which is what the game's own parser does: a block's header may sit on the line above its brace,
//! blocks nest (a `craftRecipe` has `inputs` and `outputs`, a `vehicle` has `part`s), and comments
//! can appear anywhere. Nothing is rewritten here; this only reads.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use walkdir::WalkDir;

/// One thing a script file declares: `module Base { item Axe { ... } }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptDef {
    /// The module it is declared in, which is part of its full name (`Base.Axe`). Empty when the
    /// file declares blocks without a module around them.
    pub module: String,
    /// `item`, `recipe`, `craftRecipe`, `fluid`, `vehicle`, `model`, `sound`, anything.
    pub kind: String,
    pub name: String,
    /// The block's body exactly as the author wrote it, between its braces.
    pub body: String,
}

/// The `Key = value` lines a block sets directly, with whatever is inside nested blocks left
/// out: those belong to the nested block, not to this one.
pub fn top_level_props(body: &str) -> BTreeMap<String, String> {
    let mut props = BTreeMap::new();
    let mut depth = 0usize;
    let mut statement = String::new();
    let take = |statement: &str, props: &mut BTreeMap<String, String>| {
        if let Some((key, value)) = statement.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if !key.is_empty() && !key.contains(char::is_whitespace) {
                props.insert(key.to_string(), value.to_string());
            }
        }
    };
    for c in body.chars() {
        match c {
            '{' => {
                depth += 1;
                statement.clear();
            }
            '}' => {
                depth = depth.saturating_sub(1);
                statement.clear();
            }
            // A property ends at its comma. A semicolon does not end anything: it separates the
            // values inside one, as in `XPBoosts = Nimble=2;Sneak=2;Lockpicking=1`, and treating
            // it as an ending cut every such value down to its first entry — which made two mods
            // granting different XP look like they agreed.
            ',' if depth == 0 => {
                take(&statement, &mut props);
                statement.clear();
            }
            _ if depth == 0 => statement.push(c),
            _ => {}
        }
    }
    take(&statement, &mut props);
    props
}

impl ScriptDef {
    /// How the game would name it: `item Base.Axe`.
    pub fn full_name(&self) -> String {
        if self.module.is_empty() {
            format!("{} {}", self.kind, self.name)
        } else {
            format!("{} {}.{}", self.kind, self.module, self.name)
        }
    }
}

/// Removes `/* ... */` and `// ...` comments, keeping the rest of the text as it was.
fn without_comments(text: &str) -> String {
    let bytes: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '/' && i + 1 < bytes.len() && bytes[i + 1] == '*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == '*' && bytes[i + 1] == '/') {
                // Newlines are kept so a header never swallows the line after a comment.
                if bytes[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        if bytes[i] == '/' && i + 1 < bytes.len() && bytes[i + 1] == '/' {
            while i < bytes.len() && bytes[i] != '\n' {
                i += 1;
            }
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Everything a script file declares, in the order it declares it.
pub fn parse_script_defs(text: &str) -> Vec<ScriptDef> {
    let clean = without_comments(text);
    let chars: Vec<char> = clean.chars().collect();

    let mut defs = Vec::new();
    // Each open block: its header, and where its body starts.
    let mut stack: Vec<(String, usize)> = Vec::new();
    let mut header = String::new();
    let mut module = String::new();

    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '{' => {
                let head = collapse(&header);
                header.clear();
                if stack.is_empty() && head.split_whitespace().next() == Some("module") {
                    module = head.split_whitespace().nth(1).unwrap_or_default().to_string();
                }
                stack.push((head, i + 1));
            }
            '}' => {
                if let Some((head, body_from)) = stack.pop() {
                    // A definition is a block one level inside a module, or a top-level block in a
                    // file with no module around it. Deeper blocks are parts of the definition.
                    let is_definition = match stack.len() {
                        0 => head.split_whitespace().next() != Some("module"),
                        1 => stack[0].0.split_whitespace().next() == Some("module"),
                        _ => false,
                    };
                    if is_definition {
                        let mut words = head.split_whitespace();
                        let kind = words.next().unwrap_or_default().to_string();
                        let name = words.collect::<Vec<_>>().join(" ");
                        // `imports { Base }` and other nameless blocks declare nothing.
                        if !kind.is_empty() && !name.is_empty() {
                            defs.push(ScriptDef {
                                module: if stack.is_empty() { String::new() } else { module.clone() },
                                kind,
                                name,
                                body: chars[body_from..i].iter().collect::<String>(),
                            });
                        }
                    }
                }
                header.clear();
            }
            // A property ends at its comma; what comes after it is the next header.
            ',' | ';' => header.clear(),
            c => header.push(c),
        }
        i += 1;
    }
    defs
}

/// One property two mods set differently on the same thing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClashingProperty {
    pub key: String,
    /// What each mod sets it to, in the same order as `mods`; `None` where a mod does not set it.
    pub values: Vec<Option<String>>,
}

/// The same thing declared by more than one active mod, and what they really disagree about.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDefConflict {
    /// `item Base.Axe`, as the game would name it.
    pub definition: String,
    pub kind: String,
    /// The mods declaring it, in load order: for a clashing property, the last one decides.
    pub mod_ids: Vec<String>,
    pub mods: Vec<String>,
    /// The file each mod declares it in, in the same order.
    pub files: Vec<String>,
    /// Those files on disk, for telling whether a mod changed after a merge was made.
    pub absolute_files: Vec<String>,
    /// The module and bare name, which a merged block has to be written under.
    pub module: String,
    pub name: String,
    /// Each mod's own block, as its author wrote it, in the same order.
    pub bodies: Vec<String>,
    /// Properties more than one of them sets, to different values. Everything else they set is
    /// theirs alone and survives whatever the order.
    pub clashing: Vec<ClashingProperty>,
}

/// A value made of `;`-separated entries, as the game writes lists
/// (`GrantedRecipes = A;B;C`, `XPBoosts = Nimble=2;Sneak=2`).
fn entries(value: &str) -> Vec<&str> {
    value.split(';').map(str::trim).filter(|e| !e.is_empty()).collect()
}

/// What an entry is about, so `Nimble=2` and `Nimble=3` are known to be the same setting, and
/// `base:hemp growing season` the same recipe as `Hemp Growing Season`.
fn entry_key(entry: &str) -> String {
    let before_equals = entry.split('=').next().unwrap_or(entry).trim();
    before_equals
        .trim_start_matches("base:")
        .trim_start_matches("Base:")
        .to_lowercase()
}

/// Both mods' values for one property, kept together where that means something.
///
/// A list can hold everything both mods put in it: two mods adding recipes to the same profession
/// both get their recipes. A single value cannot — an item has one model, and keeping two would
/// mean nothing — and neither can a list whose same entry is set to two different numbers. Those
/// come back as `None`, for a person to decide.
pub fn union_of(values: &[Option<String>]) -> Option<String> {
    let present: Vec<&String> = values.iter().flatten().collect();
    if present.is_empty() {
        return None;
    }
    if present.iter().all(|v| *v == present[0]) {
        return Some(present[0].clone());
    }
    // A single value (no `;` anywhere) is a slot, not a list.
    if present.iter().all(|v| !v.contains(';')) && present.iter().any(|v| !v.contains('=')) {
        return None;
    }

    let mut merged: Vec<String> = Vec::new();
    let mut seen: Vec<(String, String)> = Vec::new(); // (what it is about, how it is set)
    for value in &present {
        for entry in entries(value) {
            let key = entry_key(entry);
            match seen.iter().find(|(k, _)| *k == key) {
                Some((_, already)) if already == entry => {}
                // The same setting given two different values -- `Nimble=2` against `Nimble=3` --
                // is a slot again: nobody can have both.
                Some((_, already)) if already.contains('=') && entry.contains('=') => return None,
                // The same thing written two ways, like `Hemp Growing Season` and
                // `base:hemp growing season`: keep the first and do not list it twice.
                Some(_) => {}
                None => {
                    seen.push((key, entry.to_string()));
                    merged.push(entry.to_string());
                }
            }
        }
    }
    Some(merged.join(";"))
}

/// The block the Studio adds to a package to settle a conflict.
///
/// It is the whole declaration, not only the properties the mods disagree about. Writing only
/// those was enough for an `item`, where the game applies each declaration to the item it already
/// has -- and fatal for a `character_profession_definition`, where it builds the profession from
/// the declaration it is reading: a block with two properties and no description crashed the game
/// on startup with "Script load errors", because the description it then asked for was null. The
/// game decides which kinds work which way, so the safe block is the complete one, which is
/// right under both.
///
/// It starts from the declaration of the mod that loads last -- its text, its nested blocks, its
/// formatting -- then settles the clashing properties in place and adds what earlier mods set and
/// it does not, so nothing either mod contributed is lost.
pub fn merged_block(conflict: &ScriptDefConflict) -> String {
    let indent = if conflict.module.is_empty() { "" } else { "    " };
    let field = format!("{}    ", indent);

    // How each clashing property should read, and the other values worth keeping in sight.
    let resolved: Vec<(String, String, Vec<String>)> = conflict
        .clashing
        .iter()
        .map(|property| {
            let last = property.values.iter().flatten().next_back().cloned().unwrap_or_default();
            match union_of(&property.values) {
                Some(merged) => (property.key.clone(), merged, Vec::new()),
                None => {
                    let others: Vec<String> = property
                        .values
                        .iter()
                        .enumerate()
                        .filter_map(|(i, v)| {
                            let v = v.as_ref()?;
                            (*v != last).then(|| {
                                format!(
                                    "{} wants: {} = {},",
                                    conflict.mods.get(i).cloned().unwrap_or_default(),
                                    property.key,
                                    v
                                )
                            })
                        })
                        .collect();
                    (property.key.clone(), last, others)
                }
            }
        })
        .collect();

    // The last mod's own body, with those properties rewritten where they appear.
    let last_body = conflict.bodies.last().cloned().unwrap_or_default();
    let mut body = String::new();
    let mut written: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut statement = String::new();
    let flush = |statement: &str,
                 body: &mut String,
                 written: &mut Vec<String>,
                 ended: bool| {
        let key = statement.split('=').next().unwrap_or("").trim().to_string();
        if let Some((_, value, others)) = resolved.iter().find(|(k, _, _)| *k == key) {
            for other in others {
                body.push_str(&format!("\n{}/* {} */", field, other));
            }
            body.push_str(&format!("\n{}{} = {}", field, key, value));
            if ended {
                body.push(',');
            }
            written.push(key);
        } else {
            // Everything the last mod sets is already here: remember it, or it would be added a
            // second time below as if only an earlier mod had set it.
            if !key.is_empty() && statement.contains('=') {
                written.push(key);
            }
            body.push_str(statement);
            if ended {
                body.push(',');
            }
        }
    };
    for c in last_body.chars() {
        match c {
            '{' => {
                depth += 1;
                body.push_str(&statement);
                statement.clear();
                body.push('{');
            }
            '}' => {
                depth = depth.saturating_sub(1);
                body.push_str(&statement);
                statement.clear();
                body.push('}');
            }
            ',' if depth == 0 => {
                flush(&statement, &mut body, &mut written, true);
                statement.clear();
            }
            _ if depth == 0 => statement.push(c),
            other => {
                body.push(other);
            }
        }
    }
    flush(&statement, &mut body, &mut written, false);

    // Anything an earlier mod set that the last one never mentions, so a merge keeps it even
    // where the game rebuilds the thing from this declaration alone.
    let mut extra = String::new();
    for (key, value, others) in &resolved {
        if written.iter().any(|k| k == key) {
            continue;
        }
        for other in others {
            extra.push_str(&format!("{}/* {} */\n", field, other));
        }
        extra.push_str(&format!("{}{} = {},\n", field, key, value));
    }
    for (i, raw) in conflict.bodies.iter().enumerate() {
        if i + 1 == conflict.bodies.len() {
            continue;
        }
        for (key, value) in top_level_props(raw) {
            if written.iter().any(|k| *k == key) || resolved.iter().any(|(k, _, _)| *k == key) {
                continue;
            }
            written.push(key.clone());
            extra.push_str(&format!(
                "{}{} = {}, /* from {} */\n",
                field,
                key,
                value,
                conflict.mods.get(i).cloned().unwrap_or_default()
            ));
        }
    }

    let mut out = format!(
        "/* {} is declared by {}.\n   This is the whole declaration, settled: it loads last, so \
the game reads this one.\n   Built from {}'s, with what the others set added or merged in. */\n",
        conflict.definition,
        conflict.mods.join(" and "),
        conflict.mods.last().cloned().unwrap_or_default(),
    );
    if !conflict.module.is_empty() {
        out.push_str(&format!("module {}\n{{\n", conflict.module));
    }
    out.push_str(&format!("{}{} {}\n{}{{", indent, conflict.kind, conflict.name, indent));
    out.push_str(body.trim_end());
    if !extra.is_empty() {
        out.push('\n');
        out.push_str(extra.trim_end());
    }
    out.push_str(&format!("\n{}}}\n", indent));
    if !conflict.module.is_empty() {
        out.push_str("}\n");
    }
    out
}

/// A file name for the package that is unmistakably about this definition and safe as a path.
pub fn merged_file_path(conflict: &ScriptDefConflict) -> String {
    let safe: String = conflict
        .definition
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' { c } else { '_' })
        .collect();
    format!("media/scripts/PZModStudio_{}.txt", safe)
}

/// The same conflicts in the shape the Mod Merger resolves everything else: a path to write,
/// what each mod says, and what the Studio proposes. A definition conflict is not a file
/// conflict -- the two mods' files sit at different paths and never collide -- but it is the same
/// question, so it belongs in the same place instead of a second merging tool.
pub fn as_merger_conflicts(
    conflicts: &[ScriptDefConflict],
    drafts: &std::collections::HashMap<String, (String, String)>,
) -> Vec<crate::vfs::VfsConflictRaw> {
    conflicts
        .iter()
        .map(|c| {
            let rel_path = merged_file_path(c);
            let proposal = merged_block(c);
            let competing: Vec<crate::vfs::CompetingModFileRaw> = c
                .bodies
                .iter()
                .enumerate()
                .map(|(i, body)| crate::vfs::CompetingModFileRaw {
                    mod_id: c.mod_ids.get(i).cloned().unwrap_or_default(),
                    mod_name: c.mods.get(i).cloned().unwrap_or_default(),
                    absolute_path: c.files.get(i).cloned().unwrap_or_default(),
                    content: if c.module.is_empty() {
                        format!("{} {}\n{{{}}}\n", c.kind, c.name, body)
                    } else {
                        format!(
                            "module {}\n{{\n    {} {}\n    {{{}}}\n}}\n",
                            c.module, c.kind, c.name, body
                        )
                    },
                })
                .collect();
            let undecided = c.clashing.iter().filter(|p| union_of(&p.values).is_none()).count();
            let strategy = if c.clashing.is_empty() {
                format!(
                    "{} declare {} with different contents, and what differs is inside nested \
                     blocks. Decide by hand what the package should say.",
                    c.mods.join(" and "),
                    c.definition
                )
            } else if undecided == 0 {
                format!(
                    "Everything {} set differently on {} is a list, so the proposal keeps all of \
                     it: nothing either mod added is lost.",
                    c.mods.join(" and "),
                    c.definition
                )
            } else {
                format!(
                    "{} of the properties {} set differently on {} can only hold one value. The \
                     proposal keeps what the game uses today and leaves the other in a comment, \
                     to swap in a word.",
                    undecided,
                    c.mods.join(" and "),
                    c.definition
                )
            };
            let (merged_output, status) = match drafts.get(&rel_path) {
                Some((saved, saved_status)) => (saved.clone(), Some(saved_status.clone())),
                None => (proposal.clone(), None),
            };
            let lines = proposal.lines().count().max(1);
            crate::vfs::VfsConflictRaw {
                id: format!("definition_{}", rel_path),
                relative_path: rel_path,
                file_type: "SCRIPT_DEFINITION".to_string(),
                start_line: 1,
                end_line: lines,
                conflict_line: 1,
                total_file_lines: lines,
                // The base game has no file here: the package adds one.
                base_content: "-- vanilla file not present for this path".to_string(),
                competing_files: competing,
                merged_output,
                auto_ast_output: proposal,
                is_identical_noise: false,
                status,
                mod_changes: Vec::new(),
                collisions: Vec::new(),
                strategy,
                // A list merges by itself; a single value is a decision.
                auto_mergeable: !c.clashing.is_empty() && undecided == 0,
            }
        })
        .collect()
}

/// Script files the game merges rather than overrides, where declaring the same name twice is
/// how they are meant to work.
fn is_additive(rel_path: &str) -> bool {
    crate::vfs::is_loaded_from_every_mod(rel_path)
}

/// Things declared by two or more active mods with different bodies, in load order. Identical
/// declarations are left out: there is nothing to decide about them.
pub fn script_definition_conflicts(
    catalog: &crate::catalog::ModCatalog,
    active: &[String],
) -> Vec<ScriptDefConflict> {
    let game = catalog.game_version_int();
    // definition -> the mods declaring it, in load order.
    type Declaration = (usize, String, String, String, String, BTreeMap<String, String>, String);
    let mut by_def: HashMap<String, Vec<Declaration>> = HashMap::new();
    let mut seen_mods = std::collections::HashSet::new();

    for (position, entry) in active.iter().enumerate() {
        if entry.starts_with("Z_PZModStudio_") || entry == "PZModStudioCarrier" {
            continue;
        }
        let Some(m) = catalog.resolve(entry.trim()) else { continue };
        if m.loadability != crate::catalog::Loadability::Loadable || !seen_mods.insert(m.id.clone()) {
            continue;
        }

        // The version folder's copy of a file replaces the mod's own `common/` copy, like the game.
        let mut files: std::collections::BTreeMap<String, std::path::PathBuf> = Default::default();
        for layer in crate::load_order::mod_info::media_layers_for_build(Path::new(&m.root_dir), game) {
            let scripts = layer.join("media").join("scripts");
            for e in WalkDir::new(&scripts).into_iter().filter_map(|e| e.ok()) {
                if !e.file_type().is_file() || !e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("txt")) {
                    continue;
                }
                let Ok(rel) = e.path().strip_prefix(layer.join("media")) else { continue };
                files.insert(
                    format!("media/{}", rel.to_string_lossy().replace('\\', "/")),
                    e.path().to_path_buf(),
                );
            }
        }

        for (rel, abs) in files {
            if is_additive(&rel) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&abs) else { continue };
            for def in parse_script_defs(&text) {
                let props = top_level_props(&def.body);
                by_def.entry(def.full_name()).or_default().push((
                    position,
                    m.id.clone(),
                    m.name.clone(),
                    rel.clone(),
                    def.body,
                    props,
                    abs.to_string_lossy().to_string(),
                ));
            }
        }
    }

    let mut out: Vec<ScriptDefConflict> = Vec::new();
    for (definition, mut owners) in by_def {
        // One mod declaring something twice in its own files is its own business.
        owners.dedup_by(|a, b| a.1 == b.1);
        if owners.len() < 2 {
            continue;
        }
        owners.sort_by_key(|o| o.0);

        // The game applies each declaration to the same thing in turn, which is how a mod can
        // ship a three-line `item` block that only adds a model to someone else's item. So what
        // matters is not who declared it last, but which single properties two of them set to
        // different values: those are decided by the order, and nothing else is.
        let mut clashing: Vec<ClashingProperty> = Vec::new();
        let mut keys: Vec<&String> = owners.iter().flat_map(|o| o.5.keys()).collect();
        keys.sort();
        keys.dedup();
        for key in keys {
            let values: Vec<Option<String>> = owners.iter().map(|o| o.5.get(key).cloned()).collect();
            let set: Vec<&String> = values.iter().flatten().collect();
            if set.len() > 1 && set.iter().any(|v| *v != set[0]) {
                clashing.push(ClashingProperty { key: key.clone(), values });
            }
        }

        // Blocks with no properties at all (a recipe's ingredient lines, a model's own nesting)
        // cannot be compared this way: fall back to whether the text differs at all.
        let comparable = owners.iter().any(|o| !o.5.is_empty());
        if clashing.is_empty() {
            let first_body = collapse(&owners[0].4);
            if comparable || owners.iter().all(|o| collapse(&o.4) == first_body) {
                continue;
            }
        }

        // "item Base.ModernTire1" -> item / Base / ModernTire1; a name may hold a colon
        // ("base:burglar"), and a module-less declaration has no dot at all.
        let mut words = definition.split_whitespace();
        let kind = words.next().unwrap_or_default().to_string();
        let rest = words.collect::<Vec<_>>().join(" ");
        let (module, name) = match rest.split_once('.') {
            Some((module, name)) => (module.to_string(), name.to_string()),
            None => (String::new(), rest.clone()),
        };
        out.push(ScriptDefConflict {
            kind,
            module,
            name,
            bodies: owners.iter().map(|o| o.4.clone()).collect(),
            definition,
            mod_ids: owners.iter().map(|o| o.1.clone()).collect(),
            mods: owners.iter().map(|o| o.2.clone()).collect(),
            files: owners.iter().map(|o| o.3.clone()).collect(),
            absolute_files: owners.iter().map(|o| o.6.clone()).collect(),
            clashing,
        });
    }
    // Stable and readable: by what is declared.
    out.sort_by(|a, b| a.definition.cmp(&b.definition));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nested_block_does_not_end_the_definition_around_it() {
        let text = r#"
module Base
{
    craftRecipe MakeAxe
    {
        inputs
        {
            item 1 [Base.AxeHandle],
            item 1 tags[Welder] mode:keep,
        }
        outputs
        {
            item 1 Base.Axe,
        }
        time = 100,
    }

    item Axe
    {
        DisplayName = Axe,
        Weight = 3,
    }
}
"#;
        let defs = parse_script_defs(text);
        let names: Vec<String> = defs.iter().map(|d| d.full_name()).collect();
        assert_eq!(names, vec!["craftRecipe Base.MakeAxe", "item Base.Axe"]);
        // The whole recipe came back, inputs and outputs included.
        assert!(defs[0].body.contains("Base.AxeHandle") && defs[0].body.contains("time = 100"));
    }

    #[test]
    fn comments_and_braces_on_their_own_line_are_read_the_way_the_game_reads_them() {
        let text = r#"
/* a header comment { with a brace in it } */
module Vehicles {
    // the car
    vehicle Van
    {
        mechanicType = 2,
        part Engine
        {
            category = engine,
        }
    }
}
"#;
        let defs = parse_script_defs(text);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].full_name(), "vehicle Vehicles.Van");
        assert!(defs[0].body.contains("part Engine"));
    }

    #[test]
    fn a_template_keeps_its_whole_header_so_two_templates_are_told_apart() {
        let text = "module Base { template item Clothing { Type = Clothing, } }";
        let defs = parse_script_defs(text);
        assert_eq!(defs[0].kind, "template");
        assert_eq!(defs[0].name, "item Clothing");
        assert_eq!(defs[0].full_name(), "template Base.item Clothing");
    }

    #[test]
    fn nameless_blocks_declare_nothing() {
        let text = "module Base { imports { Base } item Axe { Weight = 3, } }";
        let defs = parse_script_defs(text);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "Axe");
    }

    #[test]
    fn a_file_without_a_module_still_declares_its_blocks() {
        let text = "item Axe { Weight = 3, }\nitem Saw { Weight = 1, }";
        let defs = parse_script_defs(text);
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].module, "");
        assert_eq!(defs[0].full_name(), "item Axe");
    }

    #[test]
    fn whitespace_does_not_make_two_mods_disagree() {
        let a = parse_script_defs("module Base { item Axe { Weight = 3, } }");
        let b = parse_script_defs("module Base\n{\n\titem Axe\n\t{\n\t\tWeight = 3,\n\t}\n}");
        // Each keeps its author's own text,
        assert_ne!(a[0].body, b[0].body);
        // but neither what they set nor the comparison sees a difference.
        assert_eq!(top_level_props(&a[0].body), top_level_props(&b[0].body));
        assert_eq!(collapse(&a[0].body), collapse(&b[0].body));
    }

    /// Reads every script of the installed game and reports what it found.
    /// PZMS_PZ_DIR=... cargo test every_vanilla_script -- --ignored --nocapture
    #[test]
    #[ignore]
    fn every_vanilla_script_parses() {
        let Ok(pz) = std::env::var("PZMS_PZ_DIR") else { return };
        let scripts = Path::new(&pz).join("media").join("scripts");
        let mut files = 0;
        let mut defs = 0;
        let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
        let mut empty_files: Vec<String> = Vec::new();
        for e in WalkDir::new(&scripts).into_iter().filter_map(|e| e.ok()) {
            if !e.file_type().is_file() || !e.path().extension().is_some_and(|x| x == "txt") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
            if text.trim().is_empty() {
                continue;
            }
            files += 1;
            let found = parse_script_defs(&text);
            if found.is_empty() {
                empty_files.push(e.path().display().to_string());
            }
            defs += found.len();
            for d in found {
                *kinds.entry(d.kind).or_default() += 1;
            }
        }
        println!("{} files, {} definitions", files, defs);
        for (kind, n) in &kinds {
            println!("  {:>6}  {}", n, kind);
        }
        println!("files with nothing in them: {}", empty_files.len());
        for f in empty_files.iter().take(20) {
            println!("  {}", f);
        }
        assert!(defs > 1000, "the game ships far more than this");
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    /// What a real mod list declares twice, and how long it takes to find out.
    /// PZMS_PZ_DIR=... PZMS_WORKSHOP_DIR=... PZMS_ZOMBOID_DIR=...
    /// cargo test --release the_same_thing_defined_twice -- --ignored --nocapture
    #[test]
    #[ignore]
    fn the_same_thing_defined_twice() {
        let (Ok(pz), Ok(workshop), Ok(zomboid)) = (
            std::env::var("PZMS_PZ_DIR"),
            std::env::var("PZMS_WORKSHOP_DIR"),
            std::env::var("PZMS_ZOMBOID_DIR"),
        ) else {
            return;
        };
        let paths = crate::vfs::StudioPaths {
            pz_install_dir: pz,
            workshop_dir: workshop.clone(),
            mod_list_ini_path: format!("{}/mods/default.txt", zomboid),
            user_zomboid_dir: zomboid.clone(),
            carrier_workshop_id: None,
            is_valid: true,
        };
        let active = crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path)
            .map(|d| d.active_mods)
            .unwrap_or_default();
        let started = std::time::Instant::now();
        let catalog = crate::catalog::build_catalog(&workshop, &zomboid, None);
        let built = started.elapsed();
        let scanning = std::time::Instant::now();
        let conflicts = script_definition_conflicts(&catalog, &active);
        println!(
            "{} active mods: catalog in {:?}, scripts in {:?}, {} definitions declared twice",
            active.len(),
            built,
            scanning.elapsed(),
            conflicts.len()
        );
        for c in conflicts.iter().take(25) {
            println!("  {} <- {}", c.definition, c.mods.join(" , "));
        }
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;

    #[test]
    fn a_semicolon_separates_values_inside_a_property_and_does_not_end_it() {
        let props = top_level_props(
            "CharacterProfession = base:burglar, XPBoosts = Nimble=2;Sneak=2;Lockpicking=1, Cost = -6,",
        );
        assert_eq!(props.get("XPBoosts").map(String::as_str), Some("Nimble=2;Sneak=2;Lockpicking=1"));
        assert_eq!(props.get("Cost").map(String::as_str), Some("-6"));
        // Two professions granting different XP must not look the same.
        let other = top_level_props("XPBoosts = Nimble=2;Sneak=2,");
        assert_ne!(props.get("XPBoosts"), other.get("XPBoosts"));
    }

    #[test]
    fn what_a_nested_block_sets_belongs_to_the_nested_block() {
        let props = top_level_props(
            "mechanicType = 2, part Engine { category = engine, Weight = 9, } DisplayName = Van,",
        );
        assert_eq!(props.len(), 2, "{:?}", props);
        assert!(props.contains_key("mechanicType") && props.contains_key("DisplayName"));
        assert!(!props.contains_key("category"));
    }
}

#[cfg(test)]
mod merge_tests {
    use super::*;

    /// Two whole declarations, as two mods would write them.
    fn with_bodies(kind: &str, first: &str, second: &str, clashing: Vec<ClashingProperty>) -> ScriptDefConflict {
        ScriptDefConflict {
            definition: format!("{} Base.Thing", kind),
            kind: kind.to_string(),
            module: "Base".into(),
            name: "Thing".into(),
            mod_ids: vec!["A".into(), "B".into()],
            mods: vec!["Mod A".into(), "Mod B".into()],
            files: vec!["a.txt".into(), "b.txt".into()],
            absolute_files: vec!["a.txt".into(), "b.txt".into()],
            bodies: vec![first.to_string(), second.to_string()],
            clashing,
        }
    }

    #[test]
    fn the_merged_block_is_a_whole_declaration_the_game_can_build_from() {
        // What crashed the game: a profession written with only the two properties in dispute.
        // The last mod's own declaration is the starting point, so the description is still there.
        let first = "
        UIName = UI_prof_Burglar,
        UIDescription = UI_profdesc_burglar,
        Cost = -6,
        XPBoosts = Nimble=2;Lockpicking=1,
    ";
        let second = "
        UIName = UI_prof_Burglar,
        UIDescription = UI_profdesc_burglar,
        Cost = -6,
        XPBoosts = Nimble=2;Sneak=2,
    ";
        let c = with_bodies(
            "character_profession_definition",
            first,
            second,
            vec![ClashingProperty {
                key: "XPBoosts".into(),
                values: vec![Some("Nimble=2;Lockpicking=1".into()), Some("Nimble=2;Sneak=2".into())],
            }],
        );
        let block = merged_block(&c);
        assert!(block.contains("UIDescription = UI_profdesc_burglar,"), "{}", block);
        assert!(block.contains("Cost = -6,"), "{}", block);
        // And the property they disagree about holds both.
        // Both mods' boosts, in the order they were met.
        assert!(block.contains("XPBoosts = Nimble=2;Lockpicking=1;Sneak=2,"), "{}", block);
        // And nothing is written twice.
        assert_eq!(block.matches("Cost =").count(), 1, "{}", block);
        assert_eq!(block.matches("UIDescription =").count(), 1, "{}", block);
        // Written once, not twice.
        assert_eq!(block.matches("XPBoosts =").count(), 1, "{}", block);
    }

    #[test]
    fn what_only_the_earlier_mod_sets_is_carried_over() {
        let first = "
        StaticModel = FR_Tire,
        Weight = 15,
    ";
        let second = "
        StaticModel = Wheel_Hand,
        primaryAnimMask = holdingbagright,
    ";
        let c = with_bodies(
            "item",
            first,
            second,
            vec![ClashingProperty {
                key: "StaticModel".into(),
                values: vec![Some("FR_Tire".into()), Some("Wheel_Hand".into())],
            }],
        );
        let block = merged_block(&c);
        // The last mod's own line, kept.
        assert!(block.contains("primaryAnimMask = holdingbagright,"), "{}", block);
        // The earlier mod's weight, which the last one never mentions, carried over and credited.
        assert!(block.contains("Weight = 15, /* from Mod A */"), "{}", block);
        // The one value that cannot hold both: today's active, the other in sight.
        assert!(block.contains("StaticModel = Wheel_Hand"), "{}", block);
        assert!(block.contains("/* Mod A wants: StaticModel = FR_Tire, */"), "{}", block);
    }

    #[test]
    fn a_nested_block_inside_the_declaration_survives() {
        let first = "
        mechanicType = 1,
    ";
        let second = "
        mechanicType = 2,
        part Engine
        {
            category = engine,
        }
    ";
        let c = with_bodies(
            "vehicle",
            first,
            second,
            vec![ClashingProperty {
                key: "mechanicType".into(),
                values: vec![Some("1".into()), Some("2".into())],
            }],
        );
        let block = merged_block(&c);
        assert!(block.contains("part Engine"), "{}", block);
        assert!(block.contains("category = engine,"), "{}", block);
    }

    fn conflict(key: &str, a: &str, b: &str) -> ScriptDefConflict {
        ScriptDefConflict {
            definition: "item Base.Thing".into(),
            kind: "item".into(),
            module: "Base".into(),
            name: "Thing".into(),
            mod_ids: vec!["A".into(), "B".into()],
            mods: vec!["Mod A".into(), "Mod B".into()],
            files: vec!["a.txt".into(), "b.txt".into()],
            absolute_files: vec!["a.txt".into(), "b.txt".into()],
            bodies: vec![String::new(), String::new()],
            clashing: vec![ClashingProperty {
                key: key.into(),
                values: vec![Some(a.into()), Some(b.into())],
            }],
        }
    }

    #[test]
    fn two_mods_adding_to_the_same_list_both_get_what_they_added() {
        let merged = union_of(&[
            Some("MakeShiv;CopyBuildingKey;Hemp Growing Season".into()),
            Some("MakeShiv;base:hemp growing season;MakeBlankID".into()),
        ])
        .expect("lists merge");
        // Each recipe once, in the order they were met, and the two spellings of the same recipe
        // are not both kept.
        assert_eq!(merged, "MakeShiv;CopyBuildingKey;Hemp Growing Season;MakeBlankID");
    }

    #[test]
    fn one_slot_cannot_hold_two_values() {
        assert!(union_of(&[Some("FR_Equipped_TireSmall".into()), Some("Wheel_Hand".into())]).is_none());
        // The same skill set to different numbers is the same slot inside a list.
        assert!(union_of(&[Some("Nimble=2;Sneak=2".into()), Some("Nimble=3".into())]).is_none());
        // Different skills are not.
        assert_eq!(
            union_of(&[Some("Nimble=2;Sneak=2".into()), Some("Nimble=2;Lockpicking=1".into())]).as_deref(),
            Some("Nimble=2;Sneak=2;Lockpicking=1")
        );
    }

    #[test]
    fn a_choice_keeps_what_the_game_does_today_and_names_the_other_option() {
        let block = merged_block(&conflict("StaticModel", "FR_Equipped_TireSmall", "Wheel_Hand"));
        // What the game does today (the last mod's value) is what is active.
        assert!(block.contains("StaticModel = Wheel_Hand,"), "{}", block);
        // The other is there to switch to, in a comment, with whose it is.
        assert!(block.contains("/* Mod A wants: StaticModel = FR_Equipped_TireSmall, */"), "{}", block);
    }

    #[test]
    fn the_file_it_goes_in_is_named_after_the_definition_and_is_a_safe_path() {
        let mut c = conflict("X", "1", "2");
        c.definition = "character_profession_definition Base.base:burglar".into();
        assert_eq!(
            merged_file_path(&c),
            "media/scripts/PZModStudio_character_profession_definition_Base.base_burglar.txt"
        );
    }
}

#[cfg(test)]
mod live_merger_tests {
    /// What the Mod Merger would now list for the real mod list, definitions included.
    /// PZMS_PZ_DIR=... PZMS_WORKSHOP_DIR=... PZMS_ZOMBOID_DIR=...
    /// cargo test --release the_real_merger_list -- --ignored --nocapture
    #[test]
    #[ignore]
    fn the_real_merger_list() {
        let (Ok(pz), Ok(workshop), Ok(zomboid)) = (
            std::env::var("PZMS_PZ_DIR"),
            std::env::var("PZMS_WORKSHOP_DIR"),
            std::env::var("PZMS_ZOMBOID_DIR"),
        ) else {
            return;
        };
        let paths = crate::vfs::StudioPaths {
            pz_install_dir: pz,
            workshop_dir: workshop.clone(),
            mod_list_ini_path: format!("{}/mods/default.txt", zomboid),
            user_zomboid_dir: zomboid.clone(),
            carrier_workshop_id: None,
            is_valid: true,
        };
        let catalog = crate::catalog::build_catalog(&workshop, &zomboid, None);
        let active = crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path)
            .map(|d| d.active_mods)
            .unwrap_or_default();
        let all = crate::vfs::scan_conflicts_for_package(&paths, &catalog, &active, None);
        let defs: Vec<_> = all.iter().filter(|c| c.file_type == "SCRIPT_DEFINITION").collect();
        println!("{} conflicts in the Merger, {} of them definitions", all.len(), defs.len());
        for c in defs.iter().take(3) {
            println!("\n=== {} (auto: {})", c.relative_path, c.auto_mergeable);
            println!("    {}", c.strategy);
            println!("--- what the Studio proposes ---\n{}", c.merged_output);
        }
    }
}
