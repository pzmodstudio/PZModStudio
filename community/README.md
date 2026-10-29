# Community rules

`rules.json` holds what `mod.info` does not say: that one mod has to load after another or
its recipes vanish, that two mods cannot be on together. PZ Mod Studio downloads it once a day
(and keeps the last copy for when it is offline), and then:

- **Auto-Sort** places the mods as if the earlier one had written `loadModAfter` itself.
- **Mod Health** says when the active list breaks a rule, names the rule's source, and offers
  the move that mends it.

The player's own rules (Mod List → right-click → Load-order rules…) win over these; a player can
also switch the download off in Settings.

## Adding a rule

Open a pull request that edits `rules.json`. Every rule needs where it was seen:

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
- `source` is a link to where it was reported or discussed, or `"tested on 42.21"` with your
  name. A rule nobody can check does not go in.
- A rule the mod's own `mod.info` already states (`require`, `loadModAfter`, `incompatible`)
  does not go here: the Studio reads those from the mod.

Keep `updated` as the date of the change.

## The mod index (`index/`)

One file per mod, `index/<mod id>.json` (a `/` in the id is written `__`), and `index.json` built
from them by `node scripts/community-index.js build` — that is what the Studio downloads once a
day. A registry says, each entry with the build it was seen on, a date, a source and whether it
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
or `none` (a harmless line, or no fix known yet). With `file` set, the Studio knows the issue is
already handled on a machine whose package carries that file.

What the Studio does with it: Auto-Sort follows `load_after`; Mod Health and the Mod List row say
when two mods of an `incompatible` pair are both on, when a known issue is not yet handled (with
its fix beside it), and when a mod this build cannot load has a `successor`; the Browse page shows
the registry before anything is installed. The player's own rules win over the index.

**Proposing one:** Mod List → right-click a mod → *Propose to the community index…* opens
GitHub's new-file page with the registry the Studio can fill in (mod.info, build) already there;
add what you tested and press *Propose changes* — GitHub forks and opens the pull request. With a
personal GitHub token saved in Settings, the Studio does that itself: it forks this repository on
your account once, writes the file on a branch and opens the pull request, then shows it to you to
add what you tested. A GitHub account is needed to propose, none to receive. The repository's check
(`.github/workflows/community.yml`) refuses a registry that is malformed, names a shim patch that
does not exist, or has an entry with neither a source nor a build, so a person reviews only what
a machine could not.

## The web view

Live at https://pichu6.github.io/-Project-Zomboid-Mod-Studio-PZ-Mod-Studio-/ — `site/index.html` shows the same data in a browser: search, a build filter, a card per mod, the
detail with its issues and fixes, the rules table, and links to propose. `.github/workflows/pages.yml`
publishes it as GitHub Pages with `index.json` and `rules.json` beside it, on every push that touches
them; it needs Pages enabled once in the repository settings (Settings → Pages → Source: GitHub
Actions). Locally: serve the repository root (`python -m http.server 8765`) and open `/site/`.
Reports without git go through the issue form (`.github/ISSUE_TEMPLATE/mod-report.yml`).
