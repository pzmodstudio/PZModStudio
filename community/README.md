# The Studio's knowledge

What `mod.info` does not say, kept in this folder and built into PZ Mod Studio when it is
compiled (`include_str!`). The app downloads none of it and sends nothing back: each version
carries what was known when it was built. To change it, edit the files here, run
`node scripts/community-index.js build`, and build the app.

- `rules.json`: one mod has to load after another, or two mods cannot be on together.
  **Auto-Sort** follows the load-after rules as if the mod had written `loadModAfter` itself;
  **Diagnostics** says when the active list breaks one, names its source, and offers the move that
  mends it. The player's own rules (Mod List → right-click → Load-order rules…) win over these.
- `index/`: one registry per mod, below.

On each PC the player can mark a known issue as **No longer happens**
(`Zomboid/PZModStudio_LocalIndex.json`), and Mod Health leaves out by itself an issue the mod's
files show is gone: an issue whose `mod_versions` does not name the installed version, a `require_path` whose file no longer names the old module, or an issue with
a `log` line when the mod changed after the issue's `date` and the line is not in the session.

## A rule in `rules.json`

```json
{
  "load_after": [
    { "mod_id": "SomeOverhaul", "after": "SomeLibrary", "note": "recipes vanish otherwise", "source": "https://steamcommunity.com/workshop/filedetails/discussion/..." }
  ],
  "incompatible": [
    { "a": "ModOne", "b": "ModTwo", "note": "both replace ISInventoryPane and the second wins silently", "source": "..." }
  ]
}
```

- `mod_id`, `after`, `a`, `b` are the `id=` line of the mod's `mod.info`, exactly (case matters
  to the game).
- `note` is one sentence a player can act on: what breaks, how it shows.
- `source` is a link to where it was reported or discussed, or `"tested on 42.21"`. A rule
  nobody can check does not go in.
- A rule the mod's own `mod.info` already states (`require`, `loadModAfter`, `incompatible`)
  does not go here: the Studio reads those from the mod.

Keep `updated` as the date of the change.

## The mod index (`index/`)

One file per mod, `index/<mod id>.json` (a `/` in the id is written `__`), and `index.json` built
from them by `node scripts/community-index.js build` — that is what the app carries. A registry says, each entry with the build it was seen on, a date, a source and whether it
is **verified** (the Studio checked it: mod.info, the log) or **reported** (a person said so):

```json
{
  "id": "FancyHandworkB42_19",
  "name": "Fancy Handwork",
  "workshop_id": "3771638611",
  "builds": ["42"],
  "requires": [],
  "incompatible": [{ "id": "OtherMod", "note": "what breaks", "build": "42.21", "date": "2026-10-18", "source": "...", "status": "reported" }],
  "load_after": [],
  "successor": { "id": "SameModB42", "workshop_id": "123", "note": "the author's Build 42 version" },
  "issues": [
    {
      "title": "getKey(nil) when the keybinds load",
      "note": "one sentence a player can act on",
      "file": "media/lua/shared/aFancyHandwork.lua", "line": 126,
      "log": "the log line, as evidence",
      "fix": { "kind": "shim_patch", "name": "fancy_handwork" },
      "build": "42.21", "date": "2026-10-18", "source": "...", "status": "verified"
    }
  ]
}
```

`fix.kind` is what the Studio can do in one click: `shim_patch` (one of the shim's per-mod
patches, by its switch name), `require_path` (`from`/`to`: a module Build 42 moved — a compatible
copy rewrites it, Fix the path in the Monitor Center publishes the one file), `compatible_copy`,
`disable_mod` (switch it off until its author updates it), `replace_mod` (`with`: the mod to use in
its place), or `none` (nothing to do: a harmless line, or no fix known yet). `mod_versions` lists
the `modversion=` values the issue was seen with; once the mod is updated past them the issue is
no longer shown. With `file` set, the Studio knows the issue is
already handled on a machine whose package carries that file.

Only what was seen goes in: a log line, the mod's own files, or the game checked by hand. A
registry that only repeats the mod's `mod.info` does not, since the Studio reads that itself.

What the Studio does with it: Auto-Sort follows `load_after`; Diagnostics and the Mod List row say
when two mods of an `incompatible` pair are both on, when a known issue is not yet handled (with
its fix beside it), and when a mod this build cannot load has a `successor`; the Browse page shows
the registry before anything is installed. The player's own rules win over the index.
