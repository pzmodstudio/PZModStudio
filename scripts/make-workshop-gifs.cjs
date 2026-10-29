// Turns screen recordings into the GIFs the Link's Workshop page shows:
//   workshop/Z_StudioLink/recordings/<name>.mp4  ->  workshop/Z_StudioLink/media/<name>.gif
// A recording named like a GIF the page uses (01-mods-screen.mp4, ...) becomes that GIF; see
// workshop/Z_StudioLink/SHOTS.md. GIFs are kept under 5 MB so the Workshop page loads: a big
// one is made again smaller (fewer frames, then narrower).
// Optional trim in the name: 03-requirements@2-11.mp4 keeps seconds 2 to 11.
// Usage: node scripts/make-workshop-gifs.cjs [only-this-name]
const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

const root = path.resolve(__dirname, '..');
const inDir = path.join(root, 'workshop', 'Z_StudioLink', 'recordings');
const outDir = path.join(root, 'workshop', 'Z_StudioLink', 'media');
const only = process.argv[2];
const MAX = 5 * 1024 * 1024;
// Tried in order until one fits: [frames per second, width]. 600 px is about the width of the
// Workshop page's description column, so a wider GIF only costs bytes.
const SIZES = [[12, 600], [10, 600], [10, 520], [8, 480]];

fs.mkdirSync(outDir, { recursive: true });
const files = fs.existsSync(inDir) ? fs.readdirSync(inDir).filter(f => /\.(mp4|mkv|mov|webm)$/i.test(f)) : [];
if (!files.length) {
  console.error(`No recordings in ${inDir}`);
  process.exit(1);
}
for (const file of files) {
  const m = path.parse(file).name.match(/^(.*?)(?:@([\d.]+)-([\d.]+))?$/);
  const name = m[1];
  // Only the page's takes (01-..., 02-...): the video in the same folder is not one.
  if (!/^\d\d-/.test(name) || (only && name !== only)) continue;
  const trim = m[2] ? ['-ss', m[2], '-to', m[3]] : [];
  const out = path.join(outDir, `${name}.gif`);
  for (const [fps, width] of SIZES) {
    // One pass: a palette made from the clip itself, so the game's dark colours stay clean.
    const filter = `fps=${fps},scale=${width}:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=192:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle`;
    execFileSync('ffmpeg', ['-v', 'error', '-y', ...trim, '-i', path.join(inDir, file), '-vf', filter, '-loop', '0', out], { stdio: 'inherit' });
    const size = fs.statSync(out).size;
    if (size <= MAX) {
      console.log(`${name}.gif  ${(size / 1048576).toFixed(1)} MB  (${fps} fps, ${width}px)`);
      break;
    }
    if (fps === SIZES[SIZES.length - 1][0]) console.log(`${name}.gif  ${(size / 1048576).toFixed(1)} MB  still over 5 MB: record it shorter`);
  }
}
