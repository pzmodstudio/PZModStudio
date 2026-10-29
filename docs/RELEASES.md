# Releases: how they are built, and how to check one

A release is built by `.github/workflows/release.yml` when a `v*` tag is pushed. The workflow:

1. **Checks the version.** It stops unless the tag is the version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` (`scripts/check-release-version.cjs`).
2. **Runs the tests.** It type-checks and builds the frontend, then runs the frontend tests and `cargo test --lib`. Nothing is built for release if any of them fails.
3. **Builds from the tagged sources only.** There is no build cache in this job, so nothing another run left can end up in what is published.
4. **Writes `SHA256SUMS.txt`** with the SHA-256 of every published file: the portable zip, the NSIS installer and the MSI.
5. **Signs the sums with the project key** (ed25519), as `SHA256SUMS.txt.sig`. The public key is `keys/release-signing.pub.pem` in this repository.
6. **Asks GitHub to attest every file** (Sigstore). This is a signed record of which workflow, commit and runner built it, and it needs no key of ours.
7. **Publishes** the files, the sums and the signature.

Every Action in every workflow is pinned to a commit, with the tag it was on in a comment, because a tag can be moved and a commit cannot. Each workflow reads only, unless a job needs more: the release job can write the release and the attestations, and the community index job commits `index.json` on `main` only, after its check passed. Dependabot proposes newer versions of the pinned Actions every week.

## Checking a download

With Node.js, put the files you downloaded in one folder together with `SHA256SUMS.txt` and `SHA256SUMS.txt.sig`, then run:

```
node scripts/verify-release.cjs <folder>
```

It says whether the sums are signed by the project key and whether every file matches its sum.

With the GitHub CLI, for any one file:

```
gh attestation verify <file> --repo pzmodstudio/PZModStudio
```

## The signing key (maintainer)

- **Private key.** It never goes in the repository, and `.gitignore` refuses `*.pem` except `keys/*.pub.pem`. The release job reads it from the repository secret `RELEASE_SIGNING_KEY`: the whole PEM text, including the `BEGIN`/`END` lines. Add it under *Settings → Secrets and variables → Actions → New repository secret*.
- **Without the secret,** a release still goes out with its sums and GitHub's attestations. The run shows a warning that the sums are not signed with the project key.
- **To replace the key,** generate a new ed25519 pair, put the public half in `keys/release-signing.pub.pem` and the private half in the secret. Releases made before the change still verify against the old public key, which remains in the repository history.

## Not done yet

- **The Windows build is not Authenticode code-signed.** That needs a code-signing certificate, which costs money, so SmartScreen may still warn about the installer.
- **There is no in-app updater.** One would download a release, check `SHA256SUMS.txt.sig` against the public key built into the app, check the file's sum, and only then replace the exe.
