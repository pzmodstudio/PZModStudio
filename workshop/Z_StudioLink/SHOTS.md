# Images of the Workshop page

The description ([workshop.txt](workshop.txt)) shows only the banners in `banners/`, loaded from
`raw.githubusercontent.com` (Steam does not follow the redirect of a `github.com/.../raw/...`
link, so those never load). The screens go in the carousel at the top of the page: the files in
`slides/`, added by hand in "Add/edit images & videos" on the item's page, in their numbered
order. `node scripts/make-workshop-art.cjs` draws both (`banners` or `slides` for one of them):
`art/key-art.jpg` is the project's key art: the hero banner, the cover slide, the 256x256
`preview.png` (a square of its full height, so no text is cut) and the round badge on the slides
(`art/badge.png`) come from it, and the banners use its teal and red. The slides take the app's screens from `docs/forum/images`, the game's crops from `media/` and
the animated ones from `recordings/`.

The screens in `media/` are each one of the game or the app (Link 1.13.1, B42.21), no zoom, cut to the part that
matters:

| File | What it shows | How |
| --- | --- | --- |
| `01-mods-screen.gif` | Main menu → MODS → the Link's screen, a mod's details | whole window, app closed |
| `02-search-filters.gif` | search, On / Off / Maps / All, groups | top-left of the screen, app closed |
| `03-requirements.gif` | turning on 3D Equipment Lockers brings 3D Gun Racks; turning Viewpoint off lists who needs it (Back, nothing applied) | list and dialogs, app closed |
| `04-load-order.gif` | sort by load order, drag Blood FX Fixes up: it stops under Blood FX; Undo | list, app closed |
| `05-many-mods.gif` | Ctrl+click five mods → Turn all on (with Gun Racks), Undo; Blood FX's options window | list and details, app closed |
| `06-error-notice.png` | the notice for a Lua error, from a throwaway example mod (not active, so the game calls it "Unnamed Mod"); no real mod is shown failing | crop of the main menu, app open |
| `07-profiles.gif` | the app switches to a small profile, the game's MODS screen shows it, then all installed | app, then game |
| `08-save-check.png` | what changed in a save since its last session (a mod taken out, the order) | crop of the main menu |
| `app-mod-list.png`, `app-mod-health.png`, `app-monitor.png`, `app-browse.png` | the app's Mod List, Mod Health, Monitor and Browse | `docs/forum/images` at 720 px |
| `09-profile-in-game.png` | the Mods screen with the app open: profile's mods, a player's note on a mod | crop of the screen, opened from Lua |

"App closed" was the Link told in memory that PZ Mod Studio is not running, so the screen is
what a subscriber without the app sees. Nothing was accepted on the Mods screen during the
recordings; the profiles, mod list and app language were put back afterwards.

## Making them again

1. Record each take (OBS, the Xbox Game Bar with `Win+Alt+R`, or ffmpeg's `gdigrab`) into
   `workshop/Z_StudioLink/recordings/<name>.mp4`; that folder stays out of git.
   `name@2-11.mp4` keeps seconds 2 to 11.
2. `node scripts/make-workshop-gifs.cjs` writes `media/<name>.gif`, each under 5 MB.
3. Commit and push: the page shows the new images, with no new upload of the mod.

## The video

`recordings/pz-studio-link.mp4` (about 2 minutes, 1080p): the same takes, captioned. For the
Workshop page it goes on YouTube, then "Add/edit images & videos" on the item's page.
