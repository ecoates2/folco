I'll do #1 fully now, and for #2 wire up the pipeline + plugin scaffolding safely (without anything that breaks your currently-building app), leaving key generation for later. Let me verify the exact Tauri v2 updater requirements and read your current app entry point first so I don't break the build.     



Fetched https://v2.tauri.app/plugin/updater/

Read [](file:///home/ethan/work/folco/gui/src-tauri/capabilities/default.json)

Read [](file:///home/ethan/work/folco/gui/src-tauri/src/lib.rs)

Now I have the definitive requirements. Key safety facts:
- The updater **cannot be enabled without signing keys** — `createUpdaterArtifacts: true` or a real `pubkey` would break your build. So I'll wire all the inert plumbing now (plugin, capability, CI signing env) and leave *only* key-generation + flipping `createUpdaterArtifacts`/`pubkey` for you later.
- Adding the Rust dep changes Cargo.lock, and your CI uses `--locked`, so I'll update the lockfile via `cargo add`.

Let me start with #1 (the release.yml polish) and the safe #2 plumbing.   

Replacing 24 lines with 36 lines in [](file:///home/ethan/work/folco/.github/workflows/release.yml)


```

```
Now add the updater plugin dependency (this also updates Cargo.lock to keep `--locked` CI happy).   

Ran terminal command: cargo add tauri-plugin-updater --package folco-gui 2>&1 | tail -20

Now register the plugin (desktop-only, the documented inert pattern) and add the capability permission.   

Replacing 14 lines with 22 lines in [](file:///home/ethan/work/folco/gui/src-tauri/src/lib.rs), [](file:///home/ethan/work/folco/gui/src-tauri/capabilities/default.json)


```

```

```

```
Let me validate the lockfile is consistent and confirm the dependency entry.   

Ran terminal command: cargo metadata --locked --format-version 1 >/dev/null && echo "lockfile OK"; echo "--- dep line ---"; grep tauri-plugin-updater gui/src-tauri/Cargo.toml

Lockfile passes `--locked` and the plugin resolved (v2.10.1). Let me relax the version to `"2"` to match your other Tauri plugin deps.   

Read [](file:///home/ethan/work/folco/gui/src-tauri/Cargo.toml#17-17), lines 17 to 30

Replacing 4 lines with 4 lines in [](file:///home/ethan/work/folco/gui/src-tauri/Cargo.toml)


```

```


Ran terminal command: cargo metadata --locked --format-version 1 >/dev/null && echo "lockfile still OK"

Both done. Here's what changed and the one manual step left for later.

## #1 — Tauri-native release polish (active now)

In release.yml:
- Bumped `tauri-action` `@v0` → **`@v1`**.
- Added a **macOS Intel** matrix row (`x86_64-apple-darwin`) alongside Apple Silicon; each row now builds for its explicit `--target`, which is required to cross-build Intel on the Apple-Silicon runner.
- Added **`releaseAssetNamePattern: "folco-[version]-[platform]-[arch][ext]"`** for consistent installer names.

## #2 — Updater plumbing (wired, inert, safe to build)

Everything that *can't* break your current build is in place:
- Cargo.toml: added `tauri-plugin-updater = "2"` (+ Cargo.lock updated so `--locked` CI stays green).
- lib.rs: registered the plugin with the documented `#[cfg(desktop)]` setup pattern — inert until endpoints/pubkey exist.
- default.json: added `updater:default`.
- release.yml: added `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` env (empty until you add secrets — harmless while updater artifacts are off).

I deliberately **did not** touch tauri.conf.json or add the JS `@tauri-apps/plugin-updater` dep, because enabling `createUpdaterArtifacts`/`pubkey` without signing keys would fail the build, and adding a JS dep without updating bun.lock would break `bun install --frozen-lockfile`.

## What you do later (when the app is ready)

1. `cd gui && bun run tauri signer generate -- -w ~/.tauri/folco.key`
2. Add repo secrets **`TAURI_SIGNING_PRIVATE_KEY`** (the private key content) and **`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`**.
3. In tauri.conf.json add:
   ```json
   "bundle": { "createUpdaterArtifacts": true },
   "plugins": {
     "updater": {
       "pubkey": "<PUBLIC KEY CONTENT>",
       "endpoints": ["https://github.com/ecoates2/folco/releases/latest/download/latest.json"]
     }
   }
   ```
4. Add `@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process` (run `bun add` so bun.lock updates), then add the update-check UI.

Once those land, `tauri-action@v1` automatically signs the bundles and uploads `latest.json` — no further workflow changes needed.

One note: I couldn't fully compile-check `folco-gui` locally (tauri-build needs a built frontend), but the lockfile validates under `--locked` and the plugin registration uses the sanctioned pattern — your `app-ci` workflow will confirm the compile on the next push.

Made changes.