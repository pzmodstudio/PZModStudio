// Turns a forum post written in Markdown into the page it is copied from:
//   docs/forum/<name>.md  ->  docs/forum/<name>.html
// Open the page, Ctrl+A, Ctrl+C, then Ctrl+V in the forum's editor: it keeps the headings, lists,
// links and images (which load from GitHub, so push them first).
// Usage: node scripts/make-forum-html.cjs docs/forum/forum-update-1.2.md
const fs = require('fs');
const path = require('path');
const { marked } = require('marked');

const src = process.argv[2];
if (!src) {
  console.error('Usage: node scripts/make-forum-html.cjs <post.md>');
  process.exit(1);
}
const md = fs.readFileSync(src, 'utf8');
const title = (md.match(/^# (.*)$/m) || [, 'Forum post'])[1];
const html = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>${title.replace(/</g, '&lt;')}</title>
<style>
  body { font-family: Segoe UI, Arial, sans-serif; max-width: 980px; margin: 24px auto; padding: 0 16px; line-height: 1.55; color: #1b1f24; background: #fff; }
  img { max-width: 100%; border: 1px solid #ccd; border-radius: 6px; margin: 8px 0 4px; }
  h1 { font-size: 26px; } h2 { font-size: 21px; margin-top: 28px; border-bottom: 1px solid #e3e6ea; padding-bottom: 4px; }
  code { background: #f1f3f5; padding: 1px 4px; border-radius: 3px; font-size: 90%; }
  hr { border: 0; border-top: 1px solid #e3e6ea; margin: 24px 0; }
  .howto { background: #fff8e1; border: 1px solid #f0d27a; padding: 10px 14px; border-radius: 6px; font-size: 14px; }
</style></head><body>
<p class="howto"><b>How to post this:</b> press Ctrl+A, Ctrl+C here, then Ctrl+V in the forum editor. Images load from GitHub once the repository has them.</p>
${marked.parse(md)}
</body></html>
`;
const out = src.replace(/\.md$/, '.html');
fs.writeFileSync(out, html);
console.log(`Written: ${path.relative(process.cwd(), out)}`);
