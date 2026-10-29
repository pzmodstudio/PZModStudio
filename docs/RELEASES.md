# Releases: how they are made, and how to check one

PZ Mod Studio is distributed as a **portable zip** only, in [GitHub Releases](https://github.com/pzmodstudio/PZModStudio/releases) (and on itch.io). There is no installer. Releases are made on the maintainer's PC with the scripts in `scripts/`: no CI runs in this repository.

## What a release holds

- `PZ-Mod-Studio-Portable-v<version>.zip`: `Project-Zomboid-Mod-Studio.exe`, `pz-mcp-server.exe`, `README_PORTABLE.txt` and `LICENSE`.
- `SHA256SUMS.txt`: the SHA-256 of the zip.
- `SHA256SUMS.txt.sig`: the project's ed25519 signature of that list (base64). The public key is `keys/release-signing.pub.pem`, and the same key is built into the app.

## How one is made (maintainer)

1. Raise the version in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json` and `package-lock.json`, and check it with `node scripts/check-release-version.cjs v<version>`.
2. `npm run build:portable` builds both executables.
3. `node scripts/sync-public.cjs "message"` checks and rebuilds the community index, mirrors the sources into the public repository and publishes the website branch.
4. `node scripts/publish-release.cjs` packs the zip, writes and signs the sums, checks the signature, and creates the GitHub release (or `--recreate` to make it again from the pushed sources).

## Checking a download

With Node.js, put the zip, `SHA256SUMS.txt` and `SHA256SUMS.txt.sig` in one folder and run:

```
node scripts/verify-release.cjs <folder>
```

It says whether the sums are signed by the project key and whether the file matches its sum. The app's **Update now** does the same before it replaces anything: it takes only what the signed list proves, and stops before touching a file otherwise.

## The signing key (maintainer)

- The private key never goes in the repository (`.gitignore` refuses `*.pem` except `keys/*.pub.pem`). `publish-release.cjs` reads it from `~/pz-release-signing-key-2026.pem`, or from the file named by `RELEASE_SIGNING_KEY_FILE`, and never prints it.
- To replace the key, generate a new ed25519 pair and put the public half in `keys/release-signing.pub.pem`. Apps built before the change keep verifying against the old public key, so they cannot update to a release signed with the new one: make that release by hand (download the zip) and say so in the notes.

## Not done

- **The Windows build is not Authenticode code-signed.** That needs a code-signing certificate, which costs money, so SmartScreen may warn about the exe.
