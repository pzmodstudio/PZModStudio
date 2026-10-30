# Site design rules (`site/`)

The rules the public site follows, so every change keeps one look. Read before touching `site/`.

## Identity
- Colours come from the key art (`workshop/Z_StudioLink/art/key-art.jpg`): near-black teal background,
  teal phosphor `--teal` for actions and kickers, rust red `--rust` for emphasis and "something broke",
  bone white `--text`. Tokens live at the top of `site/style.css`; use them, add none ad hoc.
- Type: Barlow Condensed (uppercase, headings, buttons: the game's signage), IBM Plex Sans (body),
  JetBrains Mono (anything the game or the app would print: file names, counters, kickers).
- Square-ish corners (3-6 px), thin 1 px lines, a faint 48 px grid behind everything. No gradients on
  buttons, no glassmorphism, no rounded pill cards, no stock icons.

## Content
- Lead with what only the app + Link do (mod named in each error, save check, profiles the game follows,
  Mod Health, Merger). Never name other mod managers; generic names in illustrations ("Vehicle pack").
- Real screenshots and recordings only, from `docs/forum/images` and `workshop/Z_StudioLink/media|recordings`,
  converted into `site/img/` (webp at most 1440 px; mp4 h264 crf 30, no audio, about 1100 px wide).
- Every claim must match the app's help pages (`docs/app/en/*.md`) and `modules.js`.
- Sections are numbered (`01 · Auto-Sort`); each has a kicker, a condensed headline and one lede.

## Behaviour
- Motion has a job (the Auto-Sort demo, reveals, the ticker) and stops under `prefers-reduced-motion`.
- Videos play only while on screen. No layout breaks at 375 px; no horizontal scroll.
- Check every change in a browser at 1366 px and 375 px before committing.
