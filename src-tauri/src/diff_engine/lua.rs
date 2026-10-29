use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuaSyntaxCheckResult {
    pub is_valid: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeChunkResult {
    pub merged_text: String,
    pub has_conflict: bool,
    pub conflict_line_start: Option<usize>,
    pub conflict_line_end: Option<usize>,
}

/// Validates Lua code syntax using full_moon AST parser.
/// Rewrites what Kahlua (the game's Lua) accepts but standard Lua 5.1 parsers reject, so syntax
/// checks agree with the game. Used for validation only, never for files written to disk:
/// - lone `\r` line endings (old Mac style; vanilla EditVehicleState.lua uses them),
/// - Java-style number suffixes like `0.0f` / `1d` (Kahlua parses numbers with Java's
///   Double.parseDouble, which accepts them; vanilla ISFluidEmptyAction.lua uses `0.0f`).
fn normalize_for_kahlua(code: &str) -> String {
    let code = code.replace("\r\n", "\n").replace('\r', "\n");
    let chars: Vec<char> = code.chars().collect();
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut out = String::with_capacity(code.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let starts_number = c.is_ascii_digit() && (i == 0 || !is_ident(chars[i - 1]));
        if !starts_number {
            out.push(c);
            i += 1;
            continue;
        }
        let start = i;
        if c == '0' && chars.get(i + 1).is_some_and(|x| *x == 'x' || *x == 'X') {
            // Hex literal: copied as is (f/d are hex digits there).
            i += 2;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
        } else {
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
        }
        out.extend(&chars[start..i]);
        let suffix = chars.get(i).is_some_and(|x| matches!(x, 'f' | 'F' | 'd' | 'D'));
        if suffix && !chars.get(i + 1).is_some_and(|x| is_ident(*x)) {
            i += 1; // drop the Java suffix
        }
    }
    out
}

pub fn validate_lua_syntax(code: &str) -> LuaSyntaxCheckResult {
    // full_moon parses recursively; deeply nested real-world files (huge tables, long
    // concatenation chains) overflow a normal thread stack, and a stack overflow aborts the
    // whole app. Parse on a thread with a large stack instead.
    const PARSER_STACK: usize = 256 * 1024 * 1024;
    let owned = normalize_for_kahlua(code);
    let parsed = std::thread::Builder::new()
        .name("lua-parse".into())
        .stack_size(PARSER_STACK)
        .spawn(move || full_moon::parse(&owned).map(|_| ()))
        .map_err(|e| format!("could not start the Lua parser: {}", e))
        .and_then(|handle| handle.join().map_err(|_| "the Lua parser crashed".to_string()));
    let parsed = match parsed {
        Ok(result) => result,
        Err(msg) => {
            return LuaSyntaxCheckResult { is_valid: false, error_message: Some(msg) };
        }
    };
    match parsed {
        Ok(_) => LuaSyntaxCheckResult {
            is_valid: true,
            error_message: None,
        },
        Err(err) => LuaSyntaxCheckResult {
            is_valid: false,
            error_message: Some(format!(
                "Lua AST Syntax Error: {}",
                err.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; ")
            )),
        },
    }
}

/// One side's change relative to the base: base lines `start..end` become `lines`
/// (an insertion has `start == end`, a deletion has no `lines`).
#[derive(Debug, Clone)]
struct Hunk<'a> {
    start: usize,
    end: usize,
    lines: Vec<&'a str>,
}

/// Changes needed to turn `base` into `other`, as non-overlapping hunks sorted by position.
fn diff_hunks<'a>(base: &[&'a str], other: &[&'a str]) -> Vec<Hunk<'a>> {
    let mut hunks: Vec<Hunk<'a>> = Vec::new();
    // Bounded: a huge file (vanilla has some with 300k+ lines) gets a coarser but still correct
    // diff instead of minutes of work.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    for op in similar::capture_diff_slices_deadline(similar::Algorithm::Patience, base, other, Some(deadline)) {
        let (tag, old, new) = op.as_tag_tuple();
        if tag == similar::DiffTag::Equal {
            continue;
        }
        match hunks.last_mut() {
            // A delete directly followed by an insert is one replacement.
            Some(last) if last.end == old.start => {
                last.end = old.end;
                last.lines.extend_from_slice(&other[new]);
            }
            _ => hunks.push(Hunk { start: old.start, end: old.end, lines: other[new].to_vec() }),
        }
    }
    hunks
}

/// The text one side has for base lines `start..end`, given that side's hunks inside the range.
fn side_text<'a>(base: &[&'a str], hunks: &[Hunk<'a>], start: usize, end: usize) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut pos = start;
    for h in hunks {
        out.extend_from_slice(&base[pos..h.start]);
        out.extend_from_slice(&h.lines);
        pos = h.end;
    }
    out.extend_from_slice(&base[pos..end]);
    out
}

/// Longest run of `=` that would close a `--[==[ ]==]` comment early inside `lines`.
fn block_comment_level(lines: &[&str]) -> String {
    let mut level = 1;
    while lines.iter().any(|l| l.contains(&format!("]{}]", "=".repeat(level)))) {
        level += 1;
    }
    "=".repeat(level)
}

/// Real three-way merge (diff3) of two edited versions of a Lua file against the vanilla base.
///
/// Each side's edits are computed as a diff against the base, so inserted or removed lines no
/// longer shift everything below them. Edits to different parts of the file are all applied;
/// identical edits are applied once. Only edits that touch the same base lines differently are
/// conflicts: the merged text then keeps mod A's version active and includes mod B's version in
/// a Lua block comment right below it, so the output is still valid Lua and nothing is lost.
pub fn three_way_merge_lua(base: &str, target_a: &str, target_b: &str) -> MergeChunkResult {
    three_way_merge(base, target_a, target_b, CommentStyle::Lua, "mod A", "mod B")
}

/// How conflict notes are written so the merged file stays valid for the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentStyle {
    /// `--` and `--[==[ ]==]`: Lua files and translation tables under media/lua.
    Lua,
    /// `/* */`: PZ script files (items, recipes, tile definitions, sandbox options...).
    Block,
}

/// Lua comments for files the game reads as Lua (media/lua, .lua); `/* */` for script files.
pub fn comment_style_for(rel_path: &str) -> CommentStyle {
    let p = rel_path.replace('\\', "/").to_lowercase();
    if p.ends_with(".lua") || p.starts_with("media/lua/") {
        CommentStyle::Lua
    } else {
        CommentStyle::Block
    }
}

fn note(style: CommentStyle, text: &str) -> String {
    match style {
        CommentStyle::Lua => format!("-- [[ {} ]]", text),
        CommentStyle::Block => format!("/* [[ {} ]] */", text.replace("*/", "* /")),
    }
}

/// The diff3 merge with conflict notes in `style`; `label_a` is kept active on conflicts.
pub fn three_way_merge(base: &str, target_a: &str, target_b: &str, style: CommentStyle, label_a: &str, label_b: &str) -> MergeChunkResult {
    // Compare lines regardless of line-ending style (\r\n, \n, or old-Mac lone \r).
    let unify = |s: &str| if s.contains('\r') { s.replace("\r\n", "\n").replace('\r', "\n") } else { s.to_string() };
    let (base, target_a, target_b) = (unify(base), unify(target_a), unify(target_b));
    let (base, target_a, target_b) = (base.as_str(), target_a.as_str(), target_b.as_str());
    let base_lines: Vec<&str> = base.lines().collect();
    let a_lines: Vec<&str> = target_a.lines().collect();
    let b_lines: Vec<&str> = target_b.lines().collect();
    let a_hunks = diff_hunks(&base_lines, &a_lines);
    let b_hunks = diff_hunks(&base_lines, &b_lines);

    let mut merged: Vec<String> = Vec::new();
    let mut conflict_line_start = None;
    let mut conflict_line_end = None;
    let (mut ia, mut ib, mut pos) = (0usize, 0usize, 0usize);

    loop {
        let next_a = a_hunks.get(ia);
        let next_b = b_hunks.get(ib);
        // Earliest hunk; at the same position an insertion goes first (it sits before the line).
        let start = match (next_a, next_b) {
            (None, None) => break,
            (Some(a), None) => a.start,
            (None, Some(b)) => b.start,
            (Some(a), Some(b)) => a.start.min(b.start),
        };
        merged.extend(base_lines[pos..start].iter().map(|s| s.to_string()));

        // Build one group of hunks that touch the same base lines. Touching edits are merged by
        // diff_hunks, so each side has at most one hunk starting at `start`.
        let (a_from, b_from) = (ia, ib);
        let group_start = start;
        let is_insert_at = |h: Option<&Hunk>| h.is_some_and(|h| h.start == start && h.end == start);
        let mut group_end = start;
        if is_insert_at(a_hunks.get(ia)) || is_insert_at(b_hunks.get(ib)) {
            // Insertions before line `start`: they only collide with another insertion there.
            // An edit of the lines that follow is handled as its own group.
            if is_insert_at(a_hunks.get(ia)) {
                ia += 1;
            }
            if is_insert_at(b_hunks.get(ib)) {
                ib += 1;
            }
        } else {
            // Edits of base lines: absorb every hunk (from either side) starting inside the range.
            loop {
                if let Some(h) = a_hunks.get(ia).filter(|h| h.start == start || h.start < group_end) {
                    group_end = group_end.max(h.end);
                    ia += 1;
                } else if let Some(h) = b_hunks.get(ib).filter(|h| h.start == start || h.start < group_end) {
                    group_end = group_end.max(h.end);
                    ib += 1;
                } else {
                    break;
                }
            }
        }

        let a_group = &a_hunks[a_from..ia];
        let b_group = &b_hunks[b_from..ib];
        let a_text = side_text(&base_lines, a_group, group_start, group_end);
        let b_text = side_text(&base_lines, b_group, group_start, group_end);

        if b_group.is_empty() || a_text == b_text {
            merged.extend(a_text.iter().map(|s| s.to_string()));
        } else if a_group.is_empty() {
            merged.extend(b_text.iter().map(|s| s.to_string()));
        } else {
            if conflict_line_start.is_none() {
                conflict_line_start = Some(merged.len() + 1);
            }
            merged.push(note(style, &format!(
                "PZ Mod Studio CONFLICT (vanilla lines {}-{}): {}'s version is active, {}'s is in the comment below",
                group_start + 1,
                group_end.max(group_start + 1),
                label_a,
                label_b
            )));
            merged.extend(a_text.iter().map(|s| s.to_string()));
            match style {
                CommentStyle::Lua => {
                    let eq = block_comment_level(&b_text);
                    merged.push(format!("--[{}[ PZ Mod Studio: {}'s version", eq, label_b));
                    merged.extend(b_text.iter().map(|s| s.to_string()));
                    merged.push(format!("]{}]", eq));
                }
                CommentStyle::Block => {
                    merged.push(format!("/* PZ Mod Studio: {}'s version", label_b.replace("*/", "* /")));
                    // Script comments do not nest: neutralise any "*/" inside the commented copy.
                    merged.extend(b_text.iter().map(|s| s.replace("*/", "* /")));
                    merged.push("*/".to_string());
                }
            }
            merged.push(note(style, "END CONFLICT"));
            conflict_line_end = Some(merged.len());
        }
        pos = group_end;
    }
    merged.extend(base_lines[pos..].iter().map(|s| s.to_string()));

    let mut merged_text = merged.join("\n");
    if target_a.ends_with('\n') || (target_a.is_empty() && base.ends_with('\n')) {
        merged_text.push('\n');
    }
    MergeChunkResult {
        merged_text,
        has_conflict: conflict_line_start.is_some(),
        conflict_line_start,
        conflict_line_end,
    }
}

/// One change a mod makes to the vanilla file (1-based, inclusive line ranges; for a pure
/// insertion `base_end` = `base_start` - 1, i.e. it goes before `base_start`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRegion {
    /// "added", "removed" or "changed".
    pub kind: String,
    pub base_start: usize,
    pub base_end: usize,
    pub mod_start: usize,
    pub mod_end: usize,
}

/// What one mod's version of the file changes compared to vanilla.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModChangeSummary {
    pub mod_id: String,
    pub mod_name: String,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub regions: Vec<ChangeRegion>,
}

/// Vanilla lines that two or more mods change differently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionRegion {
    pub base_start: usize,
    pub base_end: usize,
    pub mods: Vec<String>,
}

fn lines_of(s: &str) -> Vec<&str> {
    s.lines().collect()
}

/// The changes `other` makes to `base`.
pub fn change_regions(base: &str, other: &str) -> Vec<ChangeRegion> {
    let unify = |s: &str| s.replace("\r\n", "\n").replace('\r', "\n");
    let (base, other) = (unify(base), unify(other));
    let (b, o) = (lines_of(&base), lines_of(&other));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut regions: Vec<ChangeRegion> = Vec::new();
    for op in similar::capture_diff_slices_deadline(similar::Algorithm::Patience, &b, &o, Some(deadline)) {
        let (tag, old, new) = op.as_tag_tuple();
        if tag == similar::DiffTag::Equal {
            continue;
        }
        // Merge a delete directly followed by an insert into one "changed" region.
        if let Some(last) = regions.last_mut() {
            if last.base_end == old.start && last.mod_end == new.start {
                last.base_end = old.end;
                last.mod_end = new.end;
                continue;
            }
        }
        regions.push(ChangeRegion { kind: String::new(), base_start: old.start, base_end: old.end, mod_start: new.start, mod_end: new.end });
    }
    for r in &mut regions {
        r.kind = match (r.base_end > r.base_start, r.mod_end > r.mod_start) {
            (false, true) => "added",
            (true, false) => "removed",
            _ => "changed",
        }
        .to_string();
        // to 1-based inclusive
        r.base_start += 1;
        r.mod_start += 1;
    }
    regions
}

/// Per-mod summaries and the vanilla line ranges where mods collide. Empty without a vanilla base.
pub fn summarize_changes(base: &str, files: &[crate::vfs::CompetingModFileRaw]) -> (Vec<ModChangeSummary>, Vec<CollisionRegion>) {
    let summaries: Vec<ModChangeSummary> = files
        .iter()
        .map(|f| {
            let regions = change_regions(base, &f.content);
            ModChangeSummary {
                mod_id: f.mod_id.clone(),
                mod_name: f.mod_name.clone(),
                lines_added: regions.iter().map(|r| r.mod_end + 1 - r.mod_start).sum(),
                lines_removed: regions.iter().map(|r| r.base_end + 1 - r.base_start).sum(),
                regions,
            }
        })
        .collect();

    // Two mods collide where their changed vanilla ranges overlap (insertions at the same spot too).
    let mut collisions: Vec<CollisionRegion> = Vec::new();
    for (i, a) in summaries.iter().enumerate() {
        for b in &summaries[i + 1..] {
            for ra in &a.regions {
                for rb in &b.regions {
                    let (a0, a1) = (ra.base_start, ra.base_end.max(ra.base_start));
                    let (b0, b1) = (rb.base_start, rb.base_end.max(rb.base_start));
                    if a0 <= b1 && b0 <= a1 {
                        // The same edit is the same vanilla lines replaced by the same text: two
                        // deletions of different (overlapping) ranges both add nothing, and are not.
                        let same = (ra.base_start, ra.base_end) == (rb.base_start, rb.base_end)
                            && files[i].content.lines().skip(ra.mod_start - 1).take(ra.mod_end + 1 - ra.mod_start).eq(
                            files.iter().find(|f| f.mod_id == b.mod_id).map(|f| f.content.lines().skip(rb.mod_start - 1).take(rb.mod_end + 1 - rb.mod_start)).into_iter().flatten(),
                        );
                        if same {
                            continue; // identical edit: no conflict
                        }
                        let (start, end) = (a0.min(b0), a1.max(b1));
                        match collisions.iter_mut().find(|c| c.base_start <= end && start <= c.base_end) {
                            Some(c) => {
                                c.base_start = c.base_start.min(start);
                                c.base_end = c.base_end.max(end);
                                for m in [&a.mod_name, &b.mod_name] {
                                    if !c.mods.contains(m) {
                                        c.mods.push(m.clone());
                                    }
                                }
                            }
                            None => collisions.push(CollisionRegion { base_start: start, base_end: end, mods: vec![a.mod_name.clone(), b.mod_name.clone()] }),
                        }
                    }
                }
            }
        }
    }
    collisions.sort_by_key(|c| c.base_start);
    (summaries, collisions)
}

/// True when the automatic merge needs no decision: identical copies, edits to different lines
/// of a vanilla file, or a file vanilla lacks whose versions can all take effect.
pub fn can_auto_merge(rel_path: &str, base: &str, files: &[crate::vfs::CompetingModFileRaw], collisions: usize) -> bool {
    if files.len() < 2 {
        return true;
    }
    let first = files[0].content.trim();
    if files[1..].iter().all(|f| f.content.trim() == first) {
        return true;
    }
    let has_base = !base.starts_with("-- vanilla file not present") && !base.starts_with("-- vanilla file unreadable");
    if has_base {
        return collisions == 0;
    }
    match comment_style_for(rel_path) {
        CommentStyle::Lua => files.iter().all(|f| lua_runs_standalone(&f.content)),
        CommentStyle::Block => true,
    }
}

/// One sentence saying how the merged output was built, for the UI.
pub fn describe_merge_strategy(rel_path: &str, base: &str, files: &[crate::vfs::CompetingModFileRaw], collisions: usize) -> String {
    let Some(winner) = files.last() else { return String::new() };
    let first = files[0].content.trim();
    if files[1..].iter().all(|f| f.content.trim() == first) {
        return "All mods ship the same file: nothing to merge.".to_string();
    }
    let has_base = !base.starts_with("-- vanilla file not present") && !base.starts_with("-- vanilla file unreadable");
    if has_base {
        return if collisions == 0 {
            format!("Each mod's changes to the vanilla file were merged: they touch different lines, so all of them apply. Without the merge the game would only use {}'s copy.", winner.mod_name)
        } else {
            format!("Changes to different lines were merged automatically. {} place(s) where mods change the same vanilla lines differently: {}'s version is active there (it loads last) and the other version is kept in a comment for you to choose.", collisions, winner.mod_name)
        };
    }
    match comment_style_for(rel_path) {
        CommentStyle::Lua if files.iter().all(|f| lua_runs_standalone(&f.content)) => {
            "The base game has no such file. Every mod's version is kept and runs once (each in its own do ... end block), instead of the game running only the last one.".to_string()
        }
        CommentStyle::Lua => format!(
            "The base game has no such file and at least one version is a module (ends in return), so they cannot run together: {}'s version is used, as the game would. Decide by hand which one to keep.",
            winner.mod_name
        ),
        CommentStyle::Block => "The base game has no such file. All versions are combined, so every mod's items/recipes are defined (the game merges blocks by name).".to_string(),
    }
}

/// Combines the versions of one file from several mods, given in load order (the game uses
/// the one that loads last). The last-loaded version is the starting point and stays active on
/// conflicts, so with no manual edits the result behaves like the game would; edits from the
/// other mods are merged in where they do not collide.
pub fn combine_n_way_lua_or_script(
    rel_path: &str,
    base: &str,
    competing_files: &[crate::vfs::CompetingModFileRaw],
) -> String {
    if competing_files.is_empty() {
        return base.to_string();
    }
    if competing_files.len() == 1 {
        return competing_files[0].content.clone();
    }

    // Special case 1: registries.lua (Aggregate all registration calls and tables cleanly)
    if rel_path.ends_with("registries.lua") || rel_path.ends_with("registries.LUA") {
        let mut out = String::new();
        out.push_str("-- ============================================================================\n");
        out.push_str("-- PZ Mod Studio Master Patch: Unified Registries (media/registries.lua)\n");
        out.push_str(&format!("-- Auto-fused across {} active mods to prevent registry clobbering in B42\n", competing_files.len()));
        out.push_str("-- ============================================================================\n\n");

        for file in competing_files {
            out.push_str(&format!("-- [Mod: {}] ({})\n", file.mod_name, file.mod_id));
            let clean_content = file.content.trim();
            out.push_str(clean_content);
            out.push_str("\n\n");
        }
        return out.trim_end().to_string() + "\n";
    }

    // Special case 2: Check if all competing files are identical
    let first_trimmed = competing_files[0].content.trim();
    if competing_files[1..].iter().all(|f| f.content.trim() == first_trimmed) {
        return competing_files[0].content.clone();
    }

    // Special case 3: If a vanilla base exists and is valid, perform iterative 3-way merge
    let has_real_base = !base.starts_with("-- vanilla file not present") && !base.starts_with("-- vanilla file unreadable");
    if has_real_base {
        let style = comment_style_for(rel_path);
        let (winner, others) = competing_files.split_last().expect("at least two files");
        let mut current_merged = winner.content.clone();
        let mut current_label = winner.mod_name.clone();
        for file in others.iter().rev() {
            let res = three_way_merge(base, &current_merged, &file.content, style, &current_label, &file.mod_name);
            current_merged = res.merged_text;
            current_label = format!("{} (merged)", winner.mod_name);
        }
        return current_merged;
    }

    // Special case 4: a file the base game does not have, shipped by several mods. The game would
    // load only the last mod's copy, so the others' code never runs (e.g. two SaucedCarts addons
    // built from the same template both ship MyCartAddon/init.lua, and one cart is never
    // registered). Combining makes every mod's version take effect:
    // - Lua: each version in its own `do ... end` (locals cannot clash), so each runs once. Not
    //   possible for module files that end in a top-level `return` (loaded with require).
    // - Scripts: versions one after the other; the game merges item/recipe blocks by name.
    let style = comment_style_for(rel_path);
    let (winner, others) = competing_files.split_last().expect("at least two files");
    let combinable = match style {
        CommentStyle::Lua => competing_files.iter().all(|f| lua_runs_standalone(&f.content)),
        CommentStyle::Block => true,
    };
    if !combinable {
        let names: Vec<String> = others.iter().map(|f| format!("{} ({})", f.mod_name, f.mod_id)).collect();
        let header = note(
            style,
            &format!(
                "PZ Mod Studio CONFLICT: {} mods ship {} and at least one is a module (ends in return), so they cannot run together automatically. Active: {} ({}), which the game uses because it loads last. Also shipped by: {}",
                competing_files.len(),
                rel_path,
                winner.mod_name,
                winner.mod_id,
                names.join(", ")
            ),
        );
        return format!("{}\n{}", header, winner.content);
    }

    let mut out = String::new();
    out.push_str(&note(style, &format!("PZ Mod Studio: {} combined from {} mods so every version takes effect (in load order)", rel_path, competing_files.len())));
    out.push('\n');
    for file in competing_files {
        out.push('\n');
        out.push_str(&note(style, &format!("From {} ({})", file.mod_name, file.mod_id)));
        out.push('\n');
        match style {
            CommentStyle::Lua => {
                out.push_str("do\n");
                out.push_str(file.content.trim_end());
                out.push_str("\nend\n");
            }
            CommentStyle::Block => {
                out.push_str(file.content.trim_end());
                out.push('\n');
            }
        }
    }
    out
}

/// True when a Lua chunk can be wrapped in `do ... end` and run next to others: it parses and has
/// no top-level `return` (a module loaded with require must stay a file of its own).
fn lua_runs_standalone(code: &str) -> bool {
    let normalized = normalize_for_kahlua(code);
    let parsed = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || full_moon::parse(&normalized).map(|ast| ast.nodes().last_stmt().is_some_and(|s| matches!(s, full_moon::ast::LastStmt::Return(_)))))
        .ok()
        .and_then(|h| h.join().ok());
    matches!(parsed, Some(Ok(false)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lua(lines: &[&str]) -> String {
        lines.iter().map(|l| format!("{}\n", l)).collect()
    }

    fn assert_valid_lua(text: &str) {
        assert!(validate_lua_syntax(text).is_valid, "merged output is not valid Lua:\n{}", text);
    }

    const BASE: &[&str] = &[
        "local M = {}",
        "function M.a() return 1 end",
        "function M.b() return 2 end",
        "function M.c() return 3 end",
        "return M",
    ];

    #[test]
    fn insertion_in_one_mod_does_not_shift_the_other_mods_edit() {
        let mut a = BASE.to_vec();
        a.insert(1, "local extra = true"); // A adds a line near the top
        let mut b = BASE.to_vec();
        b[3] = "function M.c() return 30 end"; // B edits a line further down
        let r = three_way_merge_lua(&lua(BASE), &lua(&a), &lua(&b));
        assert!(!r.has_conflict, "{}", r.merged_text);
        assert!(r.merged_text.contains("local extra = true"));
        assert!(r.merged_text.contains("return 30"));
        assert!(!r.merged_text.contains("return 3 end"));
        assert_valid_lua(&r.merged_text);
    }

    #[test]
    fn edits_to_different_lines_and_deletions_are_all_applied() {
        let mut a = BASE.to_vec();
        a[1] = "function M.a() return 10 end";
        let mut b = BASE.to_vec();
        b.remove(2); // B deletes M.b
        let r = three_way_merge_lua(&lua(BASE), &lua(&a), &lua(&b));
        assert!(!r.has_conflict, "{}", r.merged_text);
        assert!(r.merged_text.contains("return 10"));
        assert!(!r.merged_text.contains("M.b"));
        assert_valid_lua(&r.merged_text);
    }

    #[test]
    fn identical_edits_are_applied_once() {
        let mut a = BASE.to_vec();
        a[2] = "function M.b() return 22 end";
        let r = three_way_merge_lua(&lua(BASE), &lua(&a), &lua(&a));
        assert!(!r.has_conflict);
        assert_eq!(r.merged_text, lua(&a));
    }

    #[test]
    fn conflicting_edits_keep_both_versions_and_stay_valid_lua() {
        let mut a = BASE.to_vec();
        a[2] = "function M.b() return \"from A\" end";
        let mut b = BASE.to_vec();
        b[2] = "function M.b() return \"from B\" ]] end";
        b[4] = "return M -- B touched the end too";
        let r = three_way_merge_lua(&lua(BASE), &lua(&a), &lua(&b));
        assert!(r.has_conflict);
        assert!(r.merged_text.contains("from A"));
        assert!(r.merged_text.contains("from B"), "mod B's version must not be lost");
        assert!(r.merged_text.contains("B touched the end too"), "non-conflicting edit still applied");
        let start = r.conflict_line_start.unwrap();
        let end = r.conflict_line_end.unwrap();
        let lines: Vec<&str> = r.merged_text.lines().collect();
        assert!(lines[start - 1].contains("CONFLICT"));
        assert!(lines[end - 1].contains("END CONFLICT"));
        assert_valid_lua(&r.merged_text);
    }

    #[test]
    fn insertions_at_the_same_place_conflict_but_an_insertion_before_an_edit_does_not() {
        let mut a = BASE.to_vec();
        a.insert(2, "local fromA = 1");
        let mut b = BASE.to_vec();
        b.insert(2, "local fromB = 2");
        let r = three_way_merge_lua(&lua(BASE), &lua(&a), &lua(&b));
        assert!(r.has_conflict);
        assert_valid_lua(&r.merged_text);

        let mut b2 = BASE.to_vec();
        b2[2] = "function M.b() return 200 end";
        let r2 = three_way_merge_lua(&lua(BASE), &lua(&a), &lua(&b2));
        assert!(!r2.has_conflict, "{}", r2.merged_text);
        let fa = r2.merged_text.find("fromA").unwrap();
        let fb = r2.merged_text.find("return 200").unwrap();
        assert!(fa < fb);
    }

    #[test]
    fn syntax_check_accepts_what_kahlua_accepts() {
        assert!(validate_lua_syntax("local r = x > 0.0f and 1d or 2.5F").is_valid);
        assert!(validate_lua_syntax("--\tcomment\rlocal a = 1\rreturn a").is_valid, "lone CR line endings");
        assert_eq!(normalize_for_kahlua("a = 0xff + x0f + 1.5e-3f"), "a = 0xff + x0f + 1.5e-3");
        assert!(!validate_lua_syntax("local = 1").is_valid);
        // Deep nesting must not crash the app (stack overflow aborts the process).
        let deep = format!("local x = {}1{}", "(".repeat(3000), ")".repeat(3000));
        let _ = validate_lua_syntax(&deep);
    }

    fn competing(mod_name: &str, content: &str) -> crate::vfs::CompetingModFileRaw {
        crate::vfs::CompetingModFileRaw {
            mod_id: mod_name.to_string(),
            mod_name: mod_name.to_string(),
            absolute_path: String::new(),
            content: content.to_string(),
        }
    }

    #[test]
    fn script_files_get_block_comments_and_the_last_loaded_mod_wins() {
        let base = "tileGeometry\n{\n    VERSION = 2,\n    name = vanilla_01,\n}\n";
        let early = "tileGeometry\n{\n    VERSION = 2,\n    name = early_01,\n}\n";
        let late = "tileGeometry\n{\n    VERSION = 2,\n    name = late_01, */ tricky\n}\n";
        let out = combine_n_way_lua_or_script("media/tileGeometry.txt", base, &[competing("Early", early), competing("Late", late)]);
        assert!(!out.contains("--"), "no Lua comments in a script file:\n{}", out);
        assert!(out.contains("/* [[ PZ Mod Studio CONFLICT"), "{}", out);
        // Active (uncommented) line is the last-loaded mod's.
        let active: Vec<&str> = out.lines().filter(|l| l.trim_start().starts_with("name =")).collect();
        assert_eq!(active[0].trim(), "name = late_01, */ tricky");
        assert!(out.contains("Late's version is active, Early's is in the comment below"));
        // Comments do not nest: the commented copy cannot close the comment early.
        let commented_start = out.find("/* PZ Mod Studio: Early's version").unwrap();
        let closing = out[commented_start..].find("\n*/").unwrap();
        assert!(!out[commented_start + 2..commented_start + closing].contains("*/"));

        // Lua files keep Lua comments and stay valid.
        let lua_out = combine_n_way_lua_or_script(
            "media/lua/client/A.lua",
            "local x = 1\nreturn x\n",
            &[competing("Early", "local x = 2\nreturn x\n"), competing("Late", "local x = 3\nreturn x\n")],
        );
        assert!(lua_out.contains("local x = 3"));
        assert_valid_lua(&lua_out);

        // A file vanilla does not have (like the two SaucedCarts addons' MyCartAddon/init.lua):
        // self-contained versions all run, each in its own block so locals do not clash.
        let cart = |name: &str| format!("local function registerCarts()\n  print('{}')\nend\nregisterCarts()\n", name);
        let no_base = combine_n_way_lua_or_script(
            "media/lua/shared/MyCartAddon/init.lua",
            "-- vanilla file not present for this path",
            &[competing("Trolleys", &cart("trolley")), competing("Carro", &cart("carro"))],
        );
        assert!(no_base.contains("print('trolley')") && no_base.contains("print('carro')"), "{}", no_base);
        assert_eq!(no_base.matches("\ndo\n").count(), 2, "{}", no_base);
        assert_valid_lua(&no_base);

        // Module files (ending in return) cannot run together: the last-loaded one, flagged.
        let modules = combine_n_way_lua_or_script(
            "media/lua/shared/Lib.lua",
            "-- vanilla file not present for this path",
            &[competing("Early", "local M = {}\nreturn M\n"), competing("Late", "local M = { late = true }\nreturn M\n")],
        );
        assert!(modules.contains("late = true") && !modules.contains("local M = {}\n"), "{}", modules);
        assert!(modules.contains("CONFLICT"));
        assert_valid_lua(&modules);

        // Script files are combined (the game merges blocks by name).
        let scripts = combine_n_way_lua_or_script(
            "media/scripts/items_shared.txt",
            "-- vanilla file not present for this path",
            &[competing("A", "module Base { item A1 { } }"), competing("B", "module Base { item B1 { } }")],
        );
        assert!(scripts.contains("item A1") && scripts.contains("item B1") && !scripts.contains("--"), "{}", scripts);
    }

    #[test]
    fn summarizes_what_each_mod_changes_and_where_they_collide() {
        let base = lua(BASE);
        let mut a = BASE.to_vec();
        a.insert(1, "local extra = true"); // added before line 2
        a[3] = "function M.b() return 'A' end"; // changes vanilla line 3
        let mut b = BASE.to_vec();
        b[2] = "function M.b() return 'B' end"; // changes vanilla line 3 too
        b.remove(4); // removes vanilla line 5
        let files = [competing("ModA", &lua(&a)), competing("ModB", &lua(&b))];
        let (summaries, collisions) = summarize_changes(&base, &files);

        let kinds = |i: usize| summaries[i].regions.iter().map(|r| (r.kind.as_str(), r.base_start, r.base_end)).collect::<Vec<_>>();
        assert_eq!(kinds(0), vec![("added", 2, 1), ("changed", 3, 3)]);
        assert_eq!(kinds(1), vec![("changed", 3, 3), ("removed", 5, 5)]);
        assert_eq!(collisions.len(), 1);
        assert_eq!((collisions[0].base_start, collisions[0].base_end), (3, 3));
        assert_eq!(collisions[0].mods, vec!["ModA", "ModB"]);
        assert!(describe_merge_strategy("media/lua/client/x.lua", &base, &files, collisions.len()).contains("1 place(s)"));

        // Identical edits are not a collision.
        let same = [competing("ModA", &lua(&b)), competing("ModB", &lua(&b))];
        assert!(summarize_changes(&base, &same).1.is_empty());

        // Two deletions of overlapping, different ranges both add nothing, and still collide.
        let lines: Vec<String> = (1..=7).map(|n| format!("local l{} = {}", n, n)).collect();
        let base7 = lines.join("\n") + "\n";
        let del = |from: usize, to: usize| {
            lines.iter().enumerate().filter(|(i, _)| *i + 1 < from || *i + 1 > to).map(|(_, l)| l.clone()).collect::<Vec<_>>().join("\n") + "\n"
        };
        let deletions = [competing("ModA", &del(3, 5)), competing("ModB", &del(4, 6))];
        assert_eq!(summarize_changes(&base7, &deletions).1.len(), 1);
    }

    /// Read-only check that the parser accepts real game/mod Lua (the game itself runs it).
    /// PZMS_LUA_DIRS="<ProjectZomboid/media/lua>;<workshop/content/108600>" cargo test lua_corpus -- --ignored --nocapture
    #[test]
    #[ignore]
    fn lua_corpus_parses() {
        let Ok(dirs) = std::env::var("PZMS_LUA_DIRS") else { return };
        let (mut ok, mut failed) = (0, Vec::new());
        for dir in dirs.split(';') {
            for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().extension().is_some_and(|e| e.eq_ignore_ascii_case("lua")) {
                    let Ok(bytes) = std::fs::read(entry.path()) else { continue };
                    let text = String::from_utf8_lossy(&bytes);
                    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
                    let result = validate_lua_syntax(text);
                    if result.is_valid {
                        ok += 1;
                    } else {
                        let msg: String = result.error_message.unwrap_or_default().chars().take(200).collect();
                        failed.push(format!("{} -> {}", entry.path().display(), msg));
                    }
                }
            }
        }
        println!("parsed: {ok}, rejected: {}", failed.len());
        for f in failed.iter().take(10) {
            println!("  {f}");
        }
    }

    #[test]
    fn unchanged_sides_return_the_other_side() {
        let mut b = BASE.to_vec();
        b.push("print('B')");
        let r = three_way_merge_lua(&lua(BASE), &lua(BASE), &lua(&b));
        assert!(!r.has_conflict);
        assert_eq!(r.merged_text, lua(&b));
    }
}
