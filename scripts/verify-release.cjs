// Checks a downloaded PZ Mod Studio release before you run it:
//   1. SHA256SUMS.txt is signed by the project key (keys/release-signing.pub.pem), and
//   2. every file of the release in the folder has the SHA-256 the sums list.
// node scripts/verify-release.cjs <folder with the downloaded files>
// node scripts/verify-release.cjs --self-test   (signs and checks with a throwaway key)
const crypto = require('crypto');
const fs = require('fs');
const os = require('os');
const path = require('path');

const PUBLIC_KEY = path.join(__dirname, '..', 'keys', 'release-signing.pub.pem');

/** Returns a list of problems; empty when everything checks out. */
function verify(folder, publicKeyPem) {
  const problems = [];
  const sums = path.join(folder, 'SHA256SUMS.txt');
  if (!fs.existsSync(sums)) return [`No SHA256SUMS.txt in ${folder}.`];
  const text = fs.readFileSync(sums);
  const sigFile = `${sums}.sig`;
  if (!fs.existsSync(sigFile)) {
    problems.push('SHA256SUMS.txt.sig is missing: the sums are not signed.');
  } else {
    const signature = Buffer.from(fs.readFileSync(sigFile, 'utf8').trim(), 'base64');
    if (!crypto.verify(null, text, crypto.createPublicKey(publicKeyPem), signature)) {
      problems.push('The signature of SHA256SUMS.txt does not match the project key: do not run these files.');
    }
  }
  let checked = 0;
  for (const line of text.toString('utf8').split('\n')) {
    const m = line.match(/^([0-9a-f]{64})\s+\*?(.+)$/i);
    if (!m) continue;
    const name = m[2].trim();
    if (name.includes('/') || name.includes('\\') || name.startsWith('.')) {
      problems.push(`SHA256SUMS.txt names an unexpected path: ${name}`);
      continue;
    }
    const file = path.join(folder, name);
    if (!fs.existsSync(file)) continue; // not downloaded: nothing to check
    const actual = crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
    if (actual !== m[1].toLowerCase()) problems.push(`${name}: its SHA-256 is not the one listed.`);
    else console.log(`ok  ${name}`);
    checked++;
  }
  if (checked === 0) problems.push('None of the files SHA256SUMS.txt lists are in the folder.');
  return problems;
}

function selfTest() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'pzms-verify-'));
  const { privateKey, publicKey } = crypto.generateKeyPairSync('ed25519');
  fs.writeFileSync(path.join(dir, 'app.zip'), 'the app');
  const hash = crypto.createHash('sha256').update('the app').digest('hex');
  const sums = `${hash}  app.zip\n`;
  fs.writeFileSync(path.join(dir, 'SHA256SUMS.txt'), sums);
  fs.writeFileSync(path.join(dir, 'SHA256SUMS.txt.sig'), crypto.sign(null, Buffer.from(sums), privateKey).toString('base64'));
  const pub = publicKey.export({ type: 'spki', format: 'pem' });
  const expect = (what, ok) => {
    if (!ok) {
      console.error(`self-test failed: ${what}`);
      process.exit(1);
    }
    console.log(`self-test: ${what}`);
  };
  expect('a good release passes', verify(dir, pub).length === 0);
  fs.writeFileSync(path.join(dir, 'app.zip'), 'the app, changed');
  expect('a changed file is caught', verify(dir, pub).some((p) => p.includes('app.zip')));
  fs.writeFileSync(path.join(dir, 'app.zip'), 'the app');
  fs.writeFileSync(path.join(dir, 'SHA256SUMS.txt'), sums.replace('app.zip', 'app.zip ')); // sums edited
  expect('edited sums are caught', verify(dir, pub).some((p) => p.includes('signature')));
  const other = crypto.generateKeyPairSync('ed25519').publicKey.export({ type: 'spki', format: 'pem' });
  fs.writeFileSync(path.join(dir, 'SHA256SUMS.txt'), sums);
  expect('another key is refused', verify(dir, other).some((p) => p.includes('signature')));
  fs.rmSync(dir, { recursive: true, force: true });
  expect('the project key is in the repository', fs.existsSync(PUBLIC_KEY) && crypto.createPublicKey(fs.readFileSync(PUBLIC_KEY)).asymmetricKeyType === 'ed25519');
}

if (process.argv[2] === '--self-test') {
  selfTest();
} else {
  const folder = process.argv[2];
  if (!folder) {
    console.error('Usage: node scripts/verify-release.cjs <folder>');
    process.exit(1);
  }
  const problems = verify(folder, fs.readFileSync(PUBLIC_KEY));
  if (problems.length > 0) {
    for (const p of problems) console.error(`PROBLEM  ${p}`);
    process.exit(1);
  }
  console.log('The release is the project\'s, and every file matches.');
}
