// Draws the Link's Workshop art with HTML and Edge's headless screenshot:
//   workshop/Z_StudioLink/banners/*.png  the banners in the description (loaded from GitHub)
//   workshop/Z_StudioLink/slides/*       the images and GIFs for the carousel at the top of the
//                                        page ("Add/edit images & videos" on the item's page)
// Screens come from docs/forum/images (the app), workshop/Z_StudioLink/media (game crops) and
// workshop/Z_StudioLink/recordings (the game, for the animated slides; not in git).
// Usage: node scripts/make-workshop-art.cjs [banners|slides]
const fs = require('fs');
const os = require('os');
const path = require('path');
const { execFileSync } = require('child_process');

const root = path.resolve(__dirname, '..');
const ws = path.join(root, 'workshop', 'Z_StudioLink');
const bannerDir = path.join(ws, 'banners');
const slideDir = path.join(ws, 'slides');
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'pz-art-'));
const only = process.argv[2];
const EDGE = ['C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', 'C:/Program Files/Microsoft/Edge/Application/msedge.exe'].find(p => fs.existsSync(p));
if (!EDGE) {
  console.error('Microsoft Edge not found');
  process.exit(1);
}
const fileUrl = p => 'file:///' + p.replace(/\\/g, '/').replace(/ /g, '%20');

// Lucide icons, read from the app's own dependency (the same icons the app shows).
function icon(name, size = 24, color = 'currentColor', stroke = 2) {
  const src = fs.readFileSync(path.join(root, 'node_modules', 'lucide-react', 'dist', 'esm', 'icons', `${name}.mjs`), 'utf8');
  const nodes = Function(`return ${src.match(/const __iconNode = (\[[\s\S]*?\]);\n/)[1]}`)();
  const inner = nodes.map(([tag, attrs]) => `<${tag} ${Object.entries(attrs).filter(([k]) => k !== 'key').map(([k, v]) => `${k}="${v}"`).join(' ')}/>`).join('');
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="${color}" stroke-width="${stroke}" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`;
}

// The mark: the round badge of the key art (art/key-art.jpg), cut out as art/badge.png.
const keyArt = path.join(ws, 'art', 'key-art.jpg');
const badge = path.join(ws, 'art', 'badge.png');
const LOGO = size => `<img src="${fileUrl(badge)}" style="width:${size}px;height:${size}px;border-radius:50%;flex:none;box-shadow:0 0 18px #14b8a640">`;

// Lua in the background, faint, the way the app's editor looks.
const CODE = `local Link = require "PZModStudioLink"
Events.OnMainMenuEnter.Add(function()
  local profile = Link.activeProfile()
  if profile then Link.card:show(profile) end
end)
function Link.checkSave(save)
  local changed = Link.diffMods(save)
  if #changed > 0 then
    return Link.ask(save, changed)
  end
  return true
end
for i, mod in ipairs(getActivatedMods()) do
  Link.health(mod):mark()
end`;

const CSS = `
@font-face { font-family: Bahn; src: url('${fileUrl('C:/Windows/Fonts/bahnschrift.ttf')}'); font-weight: 100 900; }
* { box-sizing: border-box; margin: 0; padding: 0; }
html, body { background: #070b0f; color: #e6edf3; font-family: 'Segoe UI', system-ui, sans-serif; overflow: hidden; }
.bahn { font-family: Bahn, 'Segoe UI', sans-serif; }
.mono { font-family: 'Cascadia Mono', Consolas, monospace; }
.code { position: absolute; white-space: pre; font: 13px/1.55 'Cascadia Mono', Consolas, monospace; color: #14b8a6; opacity: .045; pointer-events: none; }
.grid { position: absolute; inset: 0; background-image: linear-gradient(#14b8a610 1px, transparent 1px), linear-gradient(90deg, #14b8a610 1px, transparent 1px); background-size: 28px 28px; mask-image: linear-gradient(90deg, transparent, #000 60%); }
.glow { position: absolute; border-radius: 50%; background: radial-gradient(closest-side, #14b8a640, transparent); }
.chip { display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px; border: 1px solid #14b8a655; background: #14b8a614; color: #99f6e4; border-radius: 999px; font-size: 12px; font-weight: 600; letter-spacing: .02em; }
`;

const sleep = ms => Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
function shot(html, out, w, h, scale = 1, clear = false) {
  const page = path.join(tmp, path.basename(out) + '.html');
  const raw = path.join(tmp, path.basename(out) + '.raw.png');
  fs.writeFileSync(page, `<!doctype html><html><head><meta charset="utf-8"><style>${CSS}</style></head><body style="width:${w}px;height:${h}px;position:relative">${html}</body></html>`
    .replace('</style>', clear ? 'html,body{background:transparent}</style>' : '</style>'));
  // A profile of its own each time: an Edge already open would otherwise take the call. Headless
  // Edge will not open a window shorter than about 200 px, so the window is taller and cut after.
  execFileSync(EDGE, ['--headless=new', '--disable-gpu', `--user-data-dir=${path.join(tmp, 'edge-' + path.basename(out))}`, '--hide-scrollbars',
    '--allow-file-access-from-files', `--force-device-scale-factor=${scale}`, `--window-size=${w},${Math.max(h, 240)}`,
    ...(clear ? ['--default-background-color=00000000'] : []), `--screenshot=${raw}`, fileUrl(page)], { stdio: 'ignore' });
  // msedge.exe hands the work to another process and returns first: wait until the file is there
  // and has stopped growing.
  let last = -1;
  for (let i = 0; i < 300; i++) {
    sleep(150);
    const now = fs.existsSync(raw) ? fs.statSync(raw).size : -1;
    if (now > 0 && now === last) break;
    last = now;
  }
  execFileSync('ffmpeg', ['-v', 'error', '-y', '-i', raw, '-vf', `crop=${w * scale}:${h * scale}:0:0`, out]);
}

// ---------- Banners (636 px wide, the description column; drawn at 2x) ----------

const W = 636;
function section(n, title, sub, ic) {
  return `<div style="position:absolute;inset:0;background:linear-gradient(90deg,#0c1a1b,#070b0f 70%);border-left:4px solid #14b8a6;border-radius:4px 12px 12px 4px;overflow:hidden">
    <div class="grid"></div><div class="code" style="right:-20px;top:-40px">${CODE}</div>
    <div class="glow" style="left:-40px;top:-90px;width:260px;height:260px"></div>
    <div style="position:absolute;left:24px;right:90px;top:0;bottom:0;display:flex;align-items:center;gap:18px">
      <div class="bahn" style="font-size:44px;font-weight:700;color:#c62828">${n}</div>
      <div><div class="bahn" style="font-size:30px;font-weight:700;letter-spacing:.01em;text-transform:uppercase">${title}</div>
      <div style="font-size:14px;color:#8fa1b0;margin-top:2px">${sub}</div></div>
    </div>
    <div style="position:absolute;right:26px;top:0;bottom:0;display:flex;align-items:center;opacity:.9">${icon(ic, 40, '#2dd4bf', 1.6)}</div></div>`;
}
function feature(title, tag, ic) {
  return `<div style="position:absolute;inset:0;display:flex;align-items:center;gap:14px;padding:0 16px 0 10px;background:linear-gradient(90deg,#0e1a1b,#0a0f14 80%);border:1px solid #1d2a36;border-radius:10px;overflow:hidden">
    <div style="width:42px;height:42px;border-radius:9px;background:#14b8a61f;border:1px solid #14b8a666;display:flex;align-items:center;justify-content:center;flex:none">${icon(ic, 22, '#2dd4bf')}</div>
    <div class="bahn" style="font-size:21px;font-weight:600;flex:1">${title}</div>
    <div class="mono" style="font-size:10px;letter-spacing:.16em;color:#2dd4bfaa">${tag}</div></div>`;
}
function button() {
  return `<div style="position:absolute;inset:6px 0;display:flex;align-items:center;gap:16px;padding:0 22px;background:linear-gradient(90deg,#0d9488,#14b8a6);border-radius:12px;box-shadow:0 0 0 1px #5eead4 inset,0 6px 18px #14b8a644">
    ${icon('download', 30, '#ffffff', 2.4)}
    <div style="flex:1"><div class="bahn" style="font-size:26px;font-weight:700;color:#fff">Download PZ Mod Studio</div>
    <div style="font-size:13px;color:#ccfbf1">Free · open source · Windows · github.com</div></div>
    ${icon('arrow-right', 28, '#ffffff', 2.4)}</div>`;
}

const BANNERS = [
  ['download', button(), 74],
  ['s1-app', section('01', 'What the app does', 'PZ Mod Studio, the program for Windows', 'monitor'), 92],
  ['s2-game', section('02', 'What the Link adds', 'In the game: the main menu, the Mods screen, before a save loads', 'gamepad-2'), 92],
  ['s3-safe', section('03', 'Safe by design', 'Plain Lua, open source, nothing sent anywhere', 'shield-check'), 92],
  ['f-profiles', feature('Profiles: a whole mod setup in one click', 'PROFILES', 'layers'), 56],
  ['f-order', feature("Load order, sorted by the game's own rules", 'LOAD ORDER', 'list-ordered'), 56],
  ['f-health', feature('Mod Health: problems found before you play', 'MOD HEALTH', 'heart-pulse'), 56],
  ['f-monitor', feature('Monitor: which mod caused that error', 'MONITOR', 'activity'), 56],
  ['f-merger', feature('Mod Merger: two mods that change the same file', 'MERGER', 'git-merge'), 56],
  ['f-saves', feature('Your saves, backed up', 'BACKUPS', 'archive'), 56],
  ['f-more', feature('And much more', 'EXTRAS', 'sparkles'), 56],
];

// ---------- Carousel slides (1280x720) ----------

const SW = 1280, SH = 720;
// The caption along the bottom, over the screen.
function caption(kicker, title, line) {
  return `<div style="position:absolute;left:0;right:0;bottom:0;height:210px;background:linear-gradient(180deg,#070b0f00,#070b0fd9 45%,#070b0ff5)"></div>
    <div style="position:absolute;left:44px;right:44px;bottom:30px;display:flex;align-items:center;gap:20px">${LOGO(78)}
      <div><div class="mono" style="font-size:14px;letter-spacing:.2em;color:#2dd4bf">${kicker}</div>
      <div class="bahn" style="font-size:44px;font-weight:700;line-height:1.1;text-shadow:0 2px 12px #000">${title}</div>
      <div style="font-size:20px;color:#c3ced8;text-shadow:0 1px 8px #000">${line}</div></div></div>`;
}
// A screen of the app: the whole slide, from the top.
function full(img) {
  return `<img src="${fileUrl(img)}" style="position:absolute;inset:0;width:100%;height:100%;object-fit:cover;object-position:top">`;
}
// A small crop of the game: on the dark background, larger, with a border; the caption on top.
const BOX = { x: 70, y: 150, w: 1140, h: 360 };
function fitted(img, iw, ih) {
  const s = Math.min(BOX.w / iw, BOX.h / ih, 1.8);
  const w = Math.round(iw * s), h = Math.round(ih * s);
  const x = BOX.x + Math.round((BOX.w - w) / 2), y = BOX.y + Math.round((BOX.h - h) / 2);
  return `<div style="position:absolute;inset:0;background:linear-gradient(160deg,#0a1617,#070b0f 45%,#0a1016)"><div class="grid"></div>
    <div class="glow" style="left:${x - 200}px;top:${y - 200}px;width:${w + 400}px;height:${h + 400}px"></div></div>
    <img src="${fileUrl(img)}" style="position:absolute;left:${x}px;top:${y}px;width:${w}px;height:${h}px;border-radius:10px;border:1px solid #14b8a666;box-shadow:0 18px 50px #000c,0 0 0 6px #14b8a612">`;
}
function size(file) {
  return execFileSync('ffprobe', ['-v', 'error', '-select_streams', 'v:0', '-show_entries', 'stream=width,height', '-of', 'csv=p=0', file]).toString().trim().split(',').map(Number);
}

const forum = n => path.join(root, 'docs', 'forum', 'images', n);
const media = n => path.join(ws, 'media', n);
const rec = n => path.join(ws, 'recordings', n);
// [file, kicker, title, line, source, (for a video) seconds to keep]
const SLIDES = [
  ['00-cover', null],
  ['01-profiles', 'THE APP + THE LINK', 'Profiles, straight into the game', 'Switch the whole mod setup in the app; the Mods screen shows it.', rec('07-profiles.mp4'), [0, 25]],
  ['02-mod-list', 'THE APP', "Load order by the game's own rules", 'Each mod after what it requires. One click, the old order kept.', forum('02-mod-list.png')],
  ['03-mod-health', 'THE APP', 'Mod Health', 'Problems found before you start a game, each with its fix.', forum('04-mod-health.png')],
  ['04-monitor', 'THE APP', 'Monitor', 'Which mod caused that error, in plain words.', forum('10-monitor.png')],
  ['05-merger', 'THE APP', 'Mod Merger', 'Two mods that change the same file, merged into a package.', forum('09-merger-package.png')],
  ['06-browse', 'THE APP', 'Browse the Workshop and Nexus', 'Subscribe to a whole collection in one click.', forum('05-browse.png')],
  ['07-mods-screen', 'THE LINK, IN THE GAME', 'A clearer Mods screen', 'Search, filters, details. Works even without the app.', rec('01-mods-screen.mp4'), [0, 18]],
  ['08-requirements', 'THE LINK, IN THE GAME', 'Mods come on with what they need', 'And it says first what turning one off would break.', rec('03-requirements.mp4'), [0, 22]],
  ['09-save-check', 'THE LINK, IN THE GAME', 'A check before every save loads', "What changed in the save's mods, before it is too late.", media('08-save-check.png')],
  ['10-error-notice', 'THE LINK, IN THE GAME', 'Errors, with the mod named', 'Copy report puts the details on the clipboard for its author.', media('06-error-notice.png')],
];

const kb = f => Math.round(fs.statSync(f).size / 1024) + ' KB';
// The badge (also the Workshop preview, which the game's uploader wants at 256x256), the hero
// banner and the cover slide all come straight from the key art.
fs.mkdirSync(path.dirname(badge), { recursive: true });
const BADGE_CROP = 'crop=560:560:425:26';
execFileSync('ffmpeg', ['-v', 'error', '-y', '-i', keyArt, '-vf', `${BADGE_CROP},scale=512:512:flags=lanczos`, badge]);
// The preview keeps every line of text: a square the key art's full height, centred.
execFileSync('ffmpeg', ['-v', 'error', '-y', '-i', keyArt, '-vf', 'crop=ih:ih,scale=256:256:flags=lanczos', path.join(ws, 'preview.png')]);
if (only !== 'slides') {
  fs.mkdirSync(bannerDir, { recursive: true });
  execFileSync('ffmpeg', ['-v', 'error', '-y', '-i', keyArt, '-vf', 'scale=1272:-2:flags=lanczos', '-q:v', '3', path.join(bannerDir, 'hero.jpg')]);
  console.log(`banners/hero.jpg  ${kb(path.join(bannerDir, 'hero.jpg'))}`);
  for (const [name, html, h] of BANNERS) {
    const out = path.join(bannerDir, `${name}.png`);
    // On a clear page: the rounded corners sit on the Workshop page's own background.
    shot(html, out, W, h, 2, true);
    console.log(`banners/${name}.png  ${kb(out)}`);
  }
}
if (only !== 'banners') {
  fs.mkdirSync(slideDir, { recursive: true });
  for (const [name, kicker, title, line, src, cut] of SLIDES) {
    const png = path.join(tmp, `${name}.png`);
    if (src && !fs.existsSync(src)) {
      console.log(`slides/${name}: no ${src}, skipped`);
      continue;
    }
    if (cut) {
      // A video: the clip fills the slide and the caption, drawn on a clear page, goes over it.
      shot(caption(kicker, title, line), png, SW, SH, 1, true);
      // Steam refuses a preview image over 2 MB: narrower and fewer frames until it is under 1.9 MB.
      const out = path.join(slideDir, `${name}.gif`);
      for (const [width, fps] of [[960, 10], [800, 10], [800, 8], [720, 8], [640, 8], [640, 6]]) {
        const filter = `[1:v]fps=${fps},scale=${SW}:${SH}:force_original_aspect_ratio=increase:flags=lanczos,crop=${SW}:${SH}:0:0[v];[v][0:v]overlay=0:0:shortest=1,scale=${width}:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=160:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle`;
        execFileSync('ffmpeg', ['-v', 'error', '-y', '-loop', '1', '-framerate', String(fps), '-i', png, '-ss', String(cut[0]), '-t', String(cut[1] - cut[0]), '-i', src, '-filter_complex', filter, '-loop', '0', out], { stdio: 'inherit' });
        if (fs.statSync(out).size <= 1900000) break;
      }
      console.log(`slides/${name}.gif  ${kb(out)}`);
      continue;
    }
    if (!kicker) {
      // 16:9 out of the key art's wider frame, centred.
      const out = path.join(slideDir, `${name}.jpg`);
      execFileSync('ffmpeg', ['-v', 'error', '-y', '-i', keyArt, '-vf', `crop=ih*16/9:ih,scale=${SW}:${SH}:flags=lanczos`, '-q:v', '2', out]);
      console.log(`slides/${name}.jpg  ${kb(out)}`);
      continue;
    }
    if (src.startsWith(forum(''))) shot(full(src) + caption(kicker, title, line), png, SW, SH);
    else shot(fitted(src, ...size(src)) + caption(kicker, title, line), png, SW, SH);
    const out = path.join(slideDir, `${name}.jpg`);
    execFileSync('ffmpeg', ['-v', 'error', '-y', '-i', png, '-q:v', '2', out]);
    console.log(`slides/${name}.jpg  ${kb(out)}`);
  }
}
fs.rmSync(tmp, { recursive: true, force: true });
