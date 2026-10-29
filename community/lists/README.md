# Players' lists

One file per list, `<id>.json`, shared from PZ Mod Studio (Browse > My lists > Share with the
community) or written by hand:

```json
{
  "id": "vehicle-pack-b42",
  "name": "Vehicle pack (B42)",
  "author": "your name",
  "description": "What the list is for, what it was played with.",
  "build": "42",
  "updated": "2026-10-21",
  "items": [
    { "workshop_id": "2392709985", "name": "Autotsar Trailers" },
    { "mod_id": "SomeLocalMod", "name": "A mod outside the Workshop" }
  ]
}
```

`node scripts/community-lists.js check` runs on every pull request; on main `lists.json` (what the
Studio downloads) is rebuilt from these files. A list is a suggestion: copying one makes it the
player's own, and nothing is installed until they press Subscribe all.
