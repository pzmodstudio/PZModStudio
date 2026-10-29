// Signs a release's SHA256SUMS.txt with the project's ed25519 key (PKCS#8 PEM in the
// RELEASE_SIGNING_KEY environment variable, a secret of the repository), writing
// SHA256SUMS.txt.sig beside it (base64). The public half is keys/release-signing.pub.pem.
// node scripts/sign-release.cjs publish/SHA256SUMS.txt
const crypto = require('crypto');
const fs = require('fs');

const file = process.argv[2];
const pem = process.env.RELEASE_SIGNING_KEY;
if (!file || !fs.existsSync(file)) {
  console.error('Usage: node scripts/sign-release.cjs <SHA256SUMS.txt>');
  process.exit(1);
}
if (!pem) {
  console.error('RELEASE_SIGNING_KEY is not set.');
  process.exit(1);
}
const key = crypto.createPrivateKey(pem);
if (key.asymmetricKeyType !== 'ed25519') {
  console.error(`RELEASE_SIGNING_KEY is a ${key.asymmetricKeyType} key, not ed25519.`);
  process.exit(1);
}
const signature = crypto.sign(null, fs.readFileSync(file), key);
fs.writeFileSync(`${file}.sig`, signature.toString('base64') + '\n');
console.log(`Signed ${file} -> ${file}.sig`);
