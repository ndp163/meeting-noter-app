# MlxBridge — optional on-device summary runtime

Builds `libMlxBridge.dylib` + `mlx.metallib`: the MLX LLM runtime that powers
the **on-device** summary provider. It is **not** bundled in the `.app` — it is
downloaded on demand (with the model weights) from the model CDN and `dlopen`ed
at runtime, so the base install stays small (~7 MB). Only users who pick the
local provider pay the ~32 MB runtime + ~2.3 GB weights.

See `Sources/MlxBridge/MlxBridge.swift` for the C FFI the Rust backend calls
(`src-tauri/src/mlx/`).

## Why a separate package / xcodebuild

Plain `swift build` **cannot compile MLX's Metal shaders**, so it never produces
`default.metallib` and the runtime crashes with
`Failed to load the default metallib`. `xcodebuild` does compile them. mlx finds
the kernels via `current_binary_dir()/mlx.metallib` (dladdr → the dylib's own
folder), so we ship `mlx.metallib` next to the dylib — no bundle plumbing.

## 1. Build (local, Apple Silicon)

```bash
cd src-tauri/mlx-bridge
xcodebuild -scheme MlxBridge -configuration Release \
  -destination 'platform=OS X,arch=arm64' -derivedDataPath ./dd build
```

Extract the two flat artifacts:

```bash
R=dd/Build/Products/Release
cp "$R/PackageFrameworks/MlxBridge.framework/Versions/A/MlxBridge" ./libMlxBridge.dylib
cp "$R/mlx-swift_Cmlx.bundle/Contents/Resources/default.metallib" ./mlx.metallib
chmod +w ./libMlxBridge.dylib
install_name_tool -id "@rpath/libMlxBridge.dylib" ./libMlxBridge.dylib
```

## 2. Sign (required for other machines)

The app uses hardened runtime + `com.apple.security.cs.disable-library-validation`
(already in `entitlements.plist`), so the downloaded dylib must be signed with
your Developer ID:

```bash
codesign --force --options runtime --timestamp \
  --sign "Developer ID Application: <NAME> (<TEAMID>)" libMlxBridge.dylib
```

`mlx.metallib` is a data resource (not code) — no signing needed. Files we
download via `curl` are not quarantined, so Gatekeeper does not block the
`dlopen`.

## 3. Mirror the model weights

Download `mlx-community/Qwen3-4B-Instruct-2507-4bit` from Hugging Face — every
file in the repo (`config.json`, `*.safetensors`, `tokenizer*.json`, etc.).

## 4. Generate the manifest + upload

```bash
./scripts/make-manifest.sh ./libMlxBridge.dylib ./mlx.metallib <model-dir> > manifest.json
```

Upload to the CDN bucket (CloudFront origin) under the `mlx/v1/` prefix, keeping
the `url` paths in the manifest:

```
mlx/v1/manifest.json
mlx/v1/libMlxBridge.dylib
mlx/v1/mlx.metallib
mlx/v1/model/<weight files…>
```

The backend reads `https://<cdn>/mlx/v1/manifest.json` (see
`src-tauri/src/mlx/mod.rs`), downloads each `url` → local `dest`, verifies the
sha256, and writes a `.installed` marker only when the whole set checks out.
Bump `v1` → `v2` (here and in the bucket) to ship a new mlx/model without
breaking installed clients.
