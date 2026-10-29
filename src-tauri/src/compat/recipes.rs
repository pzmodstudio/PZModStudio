//! Build 41 `recipe` blocks as Build 42 `craftRecipe` blocks, for the compatible copy.
//!
//! Build 42 does not read `recipe` at all, so a Build 41 mod's crafting is simply gone until
//! its recipes are written the new way. The shapes are the game's own (read from the 42.21
//! scripts, where every one of its 969 recipes is a `craftRecipe`): the name is an id without
//! spaces and the words shown come from `Translate/<LANG>/Recipes.json` under that id; the
//! ingredients go in `inputs { item N Base.X, item N [Base.A;Base.B] mode:keep, item N
//! tags[base:hammer] }`, the result in `outputs { item N Base.X }`; `Time:` is `time =`,
//! `SkillRequired:Woodwork=2` is `SkillRequired = Woodwork:2`, `OnGiveXP:Recipe.OnGiveXP.
//! WoodWork3` is `xpAward = Woodwork:3`; literature's `TeachedRecipes` is `LearnedRecipes`
//! with the new ids. What has no Build 42 shape this writer knows (`Sound`, `AnimNode`,
//! `Heat`, `Water=10`, `OnCanPerform`, the boolean switches) is kept inside the block as a
//! `/* B41: ... */` comment and listed to review, never guessed: a recipe that half works is
//! worse than one the author looks at.
//!
//! The file is rewritten block by block: everything that is not a `recipe` block stays byte
//! for byte, so the author's `item` blocks, comments and order survive.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A Build 41 recipe written the Build 42 way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converted {
    /// The Build 42 id (`MakeStoneAxe`).
    pub id: String,
    /// The Build 41 name as written (`Make Stone Axe`): the words to show.
    pub name: String,
    pub text: String,
    /// Lines kept as `B41:` comments, for the author.
    pub review: Vec<String>,
}

/// `Make Stone Axe` -> `MakeStoneAxe`: letters, digits and underscores only, a letter first.
pub fn recipe_id(name: &str) -> String {
    let mut id: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    if id.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true) {
        id.insert(0, 'R');
    }
    id
}

/// The skills Build 42 awards XP in, by the spelling `xpAward` takes; a Build 41 `OnGiveXP`
/// function is named `Recipe.OnGiveXP.<Skill><N>` with any capitalisation.
const SKILLS: &[&str] = &[
    "Woodwork", "Cooking", "Farming", "Doctor", "Electricity", "MetalWelding", "Mechanics", "Tailoring", "Fishing", "Trapping",
    "PlantScavenging", "Fitness", "Strength", "Sprinting", "Lightfoot", "Nimble", "Sneak", "Axe", "Blunt", "SmallBlunt", "LongBlade",
    "SmallBlade", "Spear", "Maintenance", "Aiming", "Reloading", "Blacksmith", "Masonry", "Carving", "Pottery", "Glassmaking",
    "FlintKnapping", "Butchering", "Husbandry", "Tracking",
];

fn skill_named(raw: &str) -> Option<&'static str> {
    let low = raw.to_ascii_lowercase();
    SKILLS.iter().copied().find(|s| s.to_ascii_lowercase() == low).or(match low.as_str() {
        "carpentry" => Some("Woodwork"),
        "firstaid" => Some("Doctor"),
        "foraging" => Some("PlantScavenging"),
        _ => None,
    })
}

/// The items the mod itself declares: name -> module. A bare name in a recipe is one of the
/// mod's own when it declares it, else the game's (`Base.`), the way Build 41 resolved it
/// through `imports { Base }`.
pub type OwnItems = BTreeMap<String, String>;

fn qualify(item: &str, own: &OwnItems) -> String {
    let item = item.trim();
    if item.contains('.') {
        return item.to_string();
    }
    match own.get(item) {
        Some(module) if !module.is_empty() => format!("{}.{}", module, item),
        _ => format!("Base.{}", item),
    }
}

/// `Nails=5`, `Glue;2`, `Plank` -> (name, count).
fn item_and_count(raw: &str) -> (String, u32) {
    let raw = raw.trim();
    for sep in ['=', ';'] {
        if let Some((name, n)) = raw.split_once(sep) {
            return (name.trim().to_string(), n.trim().parse().unwrap_or(1));
        }
    }
    (raw.to_string(), 1)
}

/// One ingredient line as Build 42 writes it, or None for one that has no shape here.
fn ingredient(line: &str, own: &OwnItems) -> Option<String> {
    let (mode, rest) = if let Some(r) = line.strip_prefix("keep ") {
        ("keep", r.trim())
    } else if let Some(r) = line.strip_prefix("destroy ") {
        ("destroy", r.trim())
    } else {
        ("destroy", line.trim())
    };
    // [Recipe.GetItemTypes.Hammer] -> tags[base:hammer]
    if let Some(inner) = rest.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let tags: Vec<String> = inner
            .split('/')
            .filter_map(|t| t.trim().rsplit('.').next().map(|n| format!("base:{}", n.to_ascii_lowercase())))
            .filter(|t| t.len() > 5)
            .collect();
        if tags.is_empty() {
            return None;
        }
        return Some(format!("item 1 tags[{}]{}", tags.join(";"), if mode == "keep" { " mode:keep" } else { "" }));
    }
    let alternatives: Vec<(String, u32)> = rest.split('/').map(item_and_count).filter(|(n, _)| !n.is_empty()).collect();
    if alternatives.is_empty() || alternatives.iter().any(|(n, _)| n.contains(char::is_whitespace) || n.contains(':')) {
        return None;
    }
    // Water is a fluid in Build 42, not an item count.
    if alternatives.iter().any(|(n, _)| n.eq_ignore_ascii_case("Water") || n.eq_ignore_ascii_case("Base.Water")) {
        return None;
    }
    let count = alternatives[0].1;
    let names: Vec<String> = alternatives.iter().map(|(n, _)| qualify(n, own)).collect();
    let what = if names.len() == 1 { names[0].clone() } else { format!("[{}]", names.join(";")) };
    Some(format!("item {} {}{}", count, what, if mode == "keep" { " mode:keep" } else { "" }))
}

/// The statements of a block body: split at commas and line ends, comments out, trimmed.
fn statements(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if chars[i] == ',' || chars[i] == '\n' {
            let s = cur.trim().to_string();
            if !s.is_empty() {
                out.push(s);
            }
            cur.clear();
        } else {
            cur.push(chars[i]);
        }
        i += 1;
    }
    let s = cur.trim().to_string();
    if !s.is_empty() {
        out.push(s);
    }
    out
}

/// One `recipe Name { body }` as a `craftRecipe`, indented by `indent`.
pub fn convert_block(name: &str, body: &str, own: &OwnItems, indent: &str) -> Converted {
    let id = recipe_id(name);
    let mut props: Vec<String> = Vec::new();
    let mut inputs: Vec<String> = Vec::new();
    let mut outputs: Vec<String> = Vec::new();
    let mut review: Vec<String> = Vec::new();
    let mut kept: Vec<String> = Vec::new();
    let mut prop_flags: Vec<(String, u8)> = Vec::new(); // (B41 value, 1 or 2)
    let mut has_anim = false;
    for st in statements(body) {
        let keyed = st.split_once(':').map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).filter(|(k, _)| !k.contains(char::is_whitespace) && !k.contains('.'));
        match keyed.as_ref().map(|(k, v)| (k.to_ascii_lowercase(), v.as_str())) {
            Some((k, v)) if k == "result" => {
                let (item, n) = item_and_count(v);
                outputs.push(format!("item {} {}", n, qualify(&item, own)));
            }
            Some((k, v)) if k == "time" => {
                let n: f64 = v.parse().unwrap_or(0.0);
                props.push(format!("time = {}", if n == n.trunc() { format!("{}", n as i64) } else { v.to_string() }));
            }
            Some((k, v)) if k == "category" => props.push(format!("category = {}", v)),
            Some((k, v)) if k == "skillrequired" => {
                let parts: Vec<String> = v.split(';').filter_map(|p| p.split_once('=').map(|(s, l)| format!("{}:{}", skill_named(s.trim()).unwrap_or(s.trim()), l.trim()))).collect();
                if parts.is_empty() {
                    kept.push(st.clone());
                } else {
                    props.push(format!("SkillRequired = {}", parts.join(";")));
                }
            }
            Some((k, v)) if k == "ongivexp" => {
                // Recipe.OnGiveXP.WoodWork3 -> Woodwork:3
                let tail = v.rsplit('.').next().unwrap_or(v);
                let digits: String = tail.chars().rev().take_while(|c| c.is_ascii_digit()).collect::<String>().chars().rev().collect();
                let word = &tail[..tail.len() - digits.len()];
                match (skill_named(word), digits.parse::<u32>()) {
                    (Some(skill), Ok(n)) => props.push(format!("xpAward = {}:{}", skill, n)),
                    _ => kept.push(st.clone()),
                }
            }
            Some((k, v)) if k == "oncreate" => props.push(format!("OnCreate = {}", v)),
            Some((k, v)) if k == "ontest" => props.push(format!("OnTest = {}", v)),
            Some((k, v)) if k == "needtobelearn" => props.push(format!("NeedToBeLearn = {}", v.to_ascii_lowercase())),
            Some((k, v)) if k == "tooltip" => props.push(format!("Tooltip = {}", v)),
            Some((k, v)) if k == "prop1" => prop_flags.push((v.to_string(), 1)),
            Some((k, v)) if k == "prop2" => prop_flags.push((v.to_string(), 2)),
            Some((k, _)) if k == "animnode" => {
                has_anim = true;
                kept.push(st.clone());
            }
            Some(_) => kept.push(st.clone()),
            None => match ingredient(&st, own) {
                Some(line) => inputs.push(line),
                None => kept.push(st.clone()),
            },
        }
    }
    // Prop1/Prop2: the tool the hands hold, now a flag on its input line.
    for (value, n) in prop_flags {
        let flag = format!("Prop{}", n);
        let target = if let Some(idx) = value.strip_prefix("Source=").and_then(|i| i.trim().parse::<usize>().ok()) {
            idx.checked_sub(1).filter(|i| *i < inputs.len())
        } else {
            let want = qualify(&value, own).to_ascii_lowercase();
            inputs.iter().position(|l| l.to_ascii_lowercase().contains(&want))
        };
        match target {
            Some(i) => {
                let line = &mut inputs[i];
                if let Some(p) = line.find("flags[") {
                    line.insert_str(p + 6, &format!("{};", flag));
                } else {
                    line.push_str(&format!(" flags[{}]", flag));
                }
            }
            None => kept.push(format!("Prop{}:{}", n, value)),
        }
    }
    if !has_anim {
        // The game's own default for a recipe made in the hands.
    }
    props.insert(if props.iter().any(|p| p.starts_with("time")) { 1 } else { 0 }, "timedAction = Making".to_string());
    if outputs.is_empty() {
        review.push("no Result: the recipe makes nothing the writer could see".to_string());
    }
    for k in &kept {
        review.push(k.clone());
    }
    let inner = format!("{}    ", indent);
    let mut text = String::new();
    text.push_str(&format!("{}craftRecipe {}\n{}{{\n", indent, id, indent));
    text.push_str(&format!("{}/* From the Build 41 recipe \"{}\" (PZ Mod Studio); the B41: lines are what has no Build 42 shape here: look at them. */\n", inner, name));
    for p in &props {
        text.push_str(&format!("{}{},\n", inner, p));
    }
    for k in &kept {
        text.push_str(&format!("{}/* B41: {} */\n", inner, k));
    }
    text.push_str(&format!("{}inputs\n{}{{\n", inner, inner));
    for l in &inputs {
        text.push_str(&format!("{}    {},\n", inner, l));
    }
    text.push_str(&format!("{}}}\n{}outputs\n{}{{\n", inner, inner, inner));
    for l in &outputs {
        text.push_str(&format!("{}    {},\n", inner, l));
    }
    text.push_str(&format!("{}}}\n{}}}", inner, indent));
    Converted { id, name: name.to_string(), text, review }
}

/// Every `recipe` block of a script file rewritten as a `craftRecipe`, the rest untouched.
/// None when the file has no `recipe` block.
pub fn convert_file(text: &str, own: &OwnItems) -> Option<(String, Vec<Converted>)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut converted = Vec::new();
    let mut header = String::new();
    let mut header_start: Option<usize> = None;
    let mut copied = 0usize; // chars before this index are already in `out`
    let mut i = 0;
    let mut depth = 0usize;
    while i < chars.len() {
        let c = chars[i];
        // comments are not structure
        if c == '/' && chars.get(i + 1) == Some(&'*') {
            let mut j = i + 2;
            while j + 1 < chars.len() && !(chars[j] == '*' && chars[j + 1] == '/') {
                j += 1;
            }
            i = (j + 2).min(chars.len());
            header.clear();
            header_start = None;
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            header.clear();
            header_start = None;
            continue;
        }
        match c {
            '{' => {
                let head: Vec<&str> = header.split_whitespace().collect();
                if head.first() == Some(&"recipe") && head.len() > 1 {
                    // the matching brace, comments skipped
                    let mut d = 1usize;
                    let mut j = i + 1;
                    while j < chars.len() && d > 0 {
                        if chars[j] == '/' && chars.get(j + 1) == Some(&'*') {
                            j += 2;
                            while j + 1 < chars.len() && !(chars[j] == '*' && chars[j + 1] == '/') {
                                j += 1;
                            }
                            j += 2;
                            continue;
                        }
                        match chars[j] {
                            '{' => d += 1,
                            '}' => d -= 1,
                            _ => {}
                        }
                        j += 1;
                    }
                    if d == 0 {
                        let start = header_start.unwrap_or(i);
                        let line_start = chars[..start].iter().rposition(|&x| x == '\n').map(|p| p + 1).unwrap_or(0);
                        let indent: String = chars[line_start..start].iter().take_while(|x| x.is_whitespace()).collect();
                        let name = head[1..].join(" ");
                        let body: String = chars[i + 1..j - 1].iter().collect();
                        let block = convert_block(&name, &body, own, &indent);
                        out.extend(chars[copied..line_start].iter());
                        out.push_str(&block.text);
                        converted.push(block);
                        copied = j;
                        i = j;
                        header.clear();
                        header_start = None;
                        continue;
                    }
                }
                depth += 1;
                header.clear();
                header_start = None;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                header.clear();
                header_start = None;
            }
            ',' | ';' => {
                header.clear();
                header_start = None;
            }
            _ => {
                if !c.is_whitespace() && header_start.is_none() {
                    header_start = Some(i);
                }
                if c == '\n' && header.trim().is_empty() {
                    header_start = None;
                }
                header.push(c);
            }
        }
        i += 1;
    }
    if converted.is_empty() {
        return None;
    }
    out.extend(chars[copied..].iter());
    Some((out, converted))
}

/// `TeachedRecipes = Make Axe;Make Spear` -> `LearnedRecipes = MakeAxe;MakeSpear`, line by line.
pub fn convert_learned(text: &str) -> (String, usize) {
    let mut n = 0;
    let out: Vec<String> = text
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            let Some(rest) = trimmed.strip_prefix("TeachedRecipes") else { return line.to_string() };
            let Some((_, value)) = rest.split_once('=') else { return line.to_string() };
            let indent = &line[..line.len() - trimmed.len()];
            let value = value.trim().trim_end_matches(',');
            let ids: Vec<String> = value.split(';').map(|r| recipe_id(r.trim())).filter(|r| !r.is_empty()).collect();
            n += 1;
            format!("{}LearnedRecipes = {},", indent, ids.join(";"))
        })
        .collect();
    let mut joined = out.join("\n");
    if text.ends_with('\n') {
        joined.push('\n');
    }
    (joined, n)
}

/// The words for each new id, into `Translate/EN/Recipes.json` (made when missing): the Build
/// 41 translation of the name when the copy has one (`Recipe_Make_Stone_Axe`, already turned
/// to JSON before this runs), else the name as written.
fn write_names(root: &Path, names: &BTreeMap<String, String>) -> Option<usize> {
    if names.is_empty() {
        return None;
    }
    let en = root.join("media").join("lua").join("shared").join("Translate").join("EN");
    let json = en.join("Recipes.json");
    let existing = std::fs::read_to_string(&json).ok();
    let present: BTreeMap<String, String> = existing.as_deref().map(crate::translations::parse_entries).unwrap_or_default().into_iter().collect();
    let mut new: Vec<(String, String)> = Vec::new();
    for (id, name) in names {
        if present.contains_key(id) {
            continue;
        }
        let underscored = name.replace(' ', "_");
        let words = present
            .get(&format!("Recipe_{}", underscored))
            .or_else(|| present.get(&underscored))
            .cloned()
            .unwrap_or_else(|| name.clone());
        new.push((id.clone(), words));
    }
    if new.is_empty() {
        return Some(0);
    }
    let lines: Vec<String> = new.iter().map(|(k, v)| format!("    {}: {}", serde_json::to_string(k).unwrap_or_default(), serde_json::to_string(v).unwrap_or_default())).collect();
    let text = match existing {
        Some(old) => {
            let cut = old.trim_end().rfind('}')?;
            let head = old[..cut].trim_end();
            format!("{}{}{}\n}}\n", head, if head.ends_with('{') { "\n" } else { ",\n" }, lines.join(",\n"))
        }
        None => format!("{{\n{}\n}}\n", lines.join(",\n")),
    };
    std::fs::create_dir_all(&en).ok()?;
    std::fs::write(&json, text).ok()?;
    Some(new.len())
}

/// The items the mod declares across its script files: name -> module.
fn own_items(paths: &[PathBuf]) -> OwnItems {
    let mut own = OwnItems::new();
    for path in paths {
        let Ok(text) = std::fs::read_to_string(path) else { continue };
        for def in crate::scripts::parse_script_defs(&text) {
            if def.kind == "item" {
                own.insert(def.name, if def.module.is_empty() { "Base".to_string() } else { def.module });
            }
        }
    }
    own
}

/// The compatible copy's step: every script file's `recipe` blocks rewritten, `TeachedRecipes`
/// renamed, the names written. Returns (changes, warnings) in the author's words.
pub fn convert_recipes(roots: &[PathBuf]) -> (Vec<String>, Vec<String>) {
    let mut changes = Vec::new();
    let mut warnings = Vec::new();
    for root in roots {
        let mut names: BTreeMap<String, String> = BTreeMap::new();
        let mut files = 0usize;
        let mut recipes = 0usize;
        let mut learned = 0usize;
        let scripts = root.join("media").join("scripts");
        let mut paths: Vec<PathBuf> = walkdir::WalkDir::new(&scripts)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("txt")))
            .map(|e| e.into_path())
            .collect();
        paths.sort();
        let own = own_items(&paths);
        for path in paths {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let rel = path.strip_prefix(root).unwrap_or(&path).display().to_string();
            let mut text = text;
            let mut touched = false;
            if let Some((rewritten, blocks)) = convert_file(&text, &own) {
                files += 1;
                recipes += blocks.len();
                for b in &blocks {
                    names.insert(b.id.clone(), b.name.clone());
                    if !b.review.is_empty() {
                        warnings.push(format!("{}: recipe \"{}\" ({}) has lines with no Build 42 shape, kept as B41: comments: {}", rel, b.name, b.id, b.review.join(" | ")));
                    }
                }
                text = rewritten;
                touched = true;
            }
            let (with_learned, n) = convert_learned(&text);
            if n > 0 {
                learned += n;
                text = with_learned;
                touched = true;
            }
            if touched {
                let _ = std::fs::write(&path, text);
            }
        }
        if recipes > 0 {
            changes.push(format!(
                "Wrote {} Build 41 recipe(s) in {} file(s) as Build 42 craftRecipe blocks (ids without spaces: \"Make Stone Axe\" is MakeStoneAxe); Build 42 does not read `recipe` at all.",
                recipes, files
            ));
        }
        if learned > 0 {
            changes.push(format!("Renamed {} TeachedRecipes line(s) to LearnedRecipes with the new ids.", learned));
        }
        if let Some(n) = write_names(root, &names) {
            if n > 0 {
                changes.push(format!("Added {} recipe name(s) to Translate/EN/Recipes.json, so the game shows the words and not the id.", n));
            }
        }
    }
    (changes, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const B41: &str = r#"module MyMod
{
    imports { Base }

    item StoneAxe
    {
        Type = Weapon,
        TeachedRecipes = Make Stone Axe;Make Spear,
    }

    /* the recipe */
    recipe Make Stone Axe
    {
        Stone,
        TreeBranch=2,
        keep Hammer/Screwdriver,
        [Recipe.GetItemTypes.Saw],
        Glue;2,
        Water=10,
        Result:StoneAxe=2,
        Time:120.0,
        Category:Carpentry,
        SkillRequired:Woodwork=2;Carpentry=1,
        OnGiveXP:Recipe.OnGiveXP.WoodWork3,
        OnCreate:MyMod.onCreate,
        NeedToBeLearn:true,
        Prop1:Source=3,
        Sound:PutItemInBag,
        CanBeDoneFromFloor:true,
    }

    recipe 2 Planks
    {
        Log,
        keep Saw,
        Result:Plank=2,
        Time:50,
    }
}
"#;

    #[test]
    fn a_recipe_becomes_a_craft_recipe_with_the_games_shapes() {
        let own: OwnItems = [("StoneAxe".to_string(), "MyMod".to_string())].into_iter().collect();
        let (out, blocks) = convert_file(B41, &own).unwrap();
        assert_eq!(blocks.len(), 2);
        let axe = &blocks[0];
        assert_eq!(axe.id, "MakeStoneAxe");
        assert_eq!(axe.name, "Make Stone Axe");
        for want in [
            "craftRecipe MakeStoneAxe",
            "time = 120,",
            "timedAction = Making,",
            "category = Carpentry,",
            "SkillRequired = Woodwork:2;Woodwork:1,",
            "xpAward = Woodwork:3,",
            "OnCreate = MyMod.onCreate,",
            "NeedToBeLearn = true,",
            "item 1 Base.Stone,",
            "item 2 Base.TreeBranch,",
            "item 1 [Base.Hammer;Base.Screwdriver] mode:keep flags[Prop1],",
            "item 1 tags[base:saw],",
            "item 2 Base.Glue,",
            "item 2 MyMod.StoneAxe,",
            "/* B41: Water=10 */",
            "/* B41: Sound:PutItemInBag */",
            "/* B41: CanBeDoneFromFloor:true */",
        ] {
            assert!(axe.text.contains(want), "missing {:?} in\n{}", want, axe.text);
        }
        assert_eq!(axe.review.len(), 3);
        assert_eq!(blocks[1].id, "R2Planks");
        assert!(blocks[1].text.contains("item 1 Base.Saw mode:keep,"));
        assert!(blocks[1].text.contains("item 2 Base.Plank,"));
        // the rest of the file is as it was, and no `recipe` block is left
        assert!(out.contains("item StoneAxe") && out.contains("/* the recipe */") && out.contains("imports { Base }"));
        assert!(!out.contains("\n    recipe "));
        assert!(out.starts_with("module MyMod\n{"));
        assert!(out.trim_end().ends_with('}'));
        // indentation follows the header's
        assert!(out.contains("\n    craftRecipe MakeStoneAxe\n    {\n        /* From the Build 41 recipe"), "{}", out);
        assert!(convert_file("module X { item A { Type = Normal, } }", &own).is_none());
    }

    #[test]
    fn taught_recipes_take_the_new_ids() {
        let (out, n) = convert_learned(B41);
        assert_eq!(n, 1);
        assert!(out.contains("        LearnedRecipes = MakeStoneAxe;MakeSpear,"));
        assert!(!out.contains("TeachedRecipes"));
    }

    #[test]
    fn a_mod_folder_is_converted_in_place_with_its_names() {
        let dir = std::env::temp_dir().join(format!("pzms-recipes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let scripts = dir.join("media").join("scripts");
        std::fs::create_dir_all(&scripts).unwrap();
        std::fs::write(scripts.join("mymod.txt"), B41).unwrap();
        let en = dir.join("media").join("lua").join("shared").join("Translate").join("EN");
        std::fs::create_dir_all(&en).unwrap();
        std::fs::write(en.join("Recipes.json"), "{\n    \"Recipe_Make_Stone_Axe\": \"Make a Stone Axe\"\n}\n").unwrap();
        let (changes, warnings) = convert_recipes(&[dir.clone()]);
        assert_eq!(changes.len(), 3, "{:?}", changes);
        assert_eq!(warnings.len(), 1);
        let text = std::fs::read_to_string(scripts.join("mymod.txt")).unwrap();
        assert!(text.contains("craftRecipe MakeStoneAxe") && text.contains("LearnedRecipes = MakeStoneAxe;MakeSpear"));
        let names = crate::translations::parse_entries(&std::fs::read_to_string(en.join("Recipes.json")).unwrap());
        assert!(names.iter().any(|(k, v)| k == "MakeStoneAxe" && v == "Make a Stone Axe"), "{:?}", names);
        assert!(names.iter().any(|(k, v)| k == "R2Planks" && v == "2 Planks"));
        // a second run changes nothing more
        let (again, _) = convert_recipes(&[dir.clone()]);
        assert!(again.is_empty(), "{:?}", again);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
