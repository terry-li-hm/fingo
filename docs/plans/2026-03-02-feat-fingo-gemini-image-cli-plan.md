---
title: "feat: Build fingo — Gemini image generation and editing CLI"
type: feat
status: active
date: 2026-03-02
---

# feat: Build fingo — Gemini image generation and editing CLI

## Overview

`fingo` (Latin: *to shape, form, fashion*) is a Rust CLI wrapping the Google Gemini / Nano Banana image API. It provides four subcommands for AI image work directly from the terminal: text-to-image generation, image editing, object/text removal, and model listing.

Reserved on crates.io at v0.1.0 (placeholder). This plan covers the full v0.1 implementation.

## Proposed Solution

A single Rust binary with a `clap`-based subcommand structure, sync HTTP via `ureq`, and macOS Keychain credential management — following the same conventions as `stips` (the existing API CLI in this setup).

### Subcommands

```
fingo gen "<prompt>" [-m model] [-o output.jpg]
fingo edit <image> "<prompt>" [-m model] [-o output.jpg]
fingo remove <image> "<prompt>" [-m model] [-o output.jpg]
fingo models
fingo key save <api-key>
fingo key show
```

### Default model routing

| Subcommand | Default model |
|---|---|
| `gen` | `gemini-2.0-flash-exp-image-generation` |
| `edit` | `nano-banana-pro-preview` |
| `remove` | `nano-banana-pro-preview` |

User can override with `-m / --model <model>`.

## Technical Approach

### Dependencies (`Cargo.toml`)

```toml
[dependencies]
clap       = { version = "4", features = ["derive"] }
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
ureq       = { version = "3", features = ["json"] }
base64     = "0.22"

[profile.release]
opt-level     = "z"
lto           = true
codegen-units = 1
panic         = "abort"
strip         = true
```

No async runtime (`tokio`), no `anyhow`/`thiserror` — matches workspace convention.

### File structure

```
src/
  main.rs       — CLI entrypoint, Cli/Commands structs, run()
  api.rs        — Gemini API call: build payload, POST, extract image
  keychain.rs   — read/write API key via `security` CLI
  models.rs     — list available models via GET /v1beta/models
  error.rs      — AppError enum
```

### Error handling

Custom `AppError` enum matching `stips` pattern:

```rust
enum AppError {
    ApiKeyMissing,
    Silent,
    Message(String),
}
```

`main()` delegates to `run() -> Result<(), AppError>`, prints errors to stderr, exits 1.

### Keychain

Service name: `"gemini-api-key-secrets"` (already registered in keychain).

Shell out to `security find-generic-password -s "gemini-api-key-secrets" -w` — same as `stips`. No Rust keychain crate.

`fingo key save <key>` → `security add-generic-password ... -U`
`fingo key show` → print masked key (`AIza...Xl8`)

### API call pattern

```
POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}
Content-Type: application/json

{
  "contents": [{"parts": [
    {"text": "<prompt>"},
    // only for edit/remove:
    {"inline_data": {"mime_type": "image/jpeg", "data": "<base64>"}}
  ]}],
  "generationConfig": {"responseModalities": ["image", "text"]}
}
```

Response extraction (critical — camelCase):

```rust
// response.candidates[0].content.parts[n].inlineData.data
// Also check inline_data (snake_case) — API is inconsistent
```

**Always check `finishReason` before accessing `content.parts`** — RECITATION or other blocks return `finishReason` with an empty `content`, causing a panic on `.parts` access.

### Output

- Default output path: `./fingo-out.jpg`
- `-o / --output <path>` overrides
- Print saved path to stdout: `Saved: ./fingo-out.jpg`
- Detect MIME type from response and use correct extension (`.jpg` / `.png` / `.webp`)

## Acceptance Criteria

- [ ] `fingo gen "a neon cyberpunk cat"` generates and saves an image
- [ ] `fingo edit photo.jpg "make it look like watercolor"` produces edited image
- [ ] `fingo remove photo.jpg "remove the FOCUS text"` returns clean image
- [ ] `fingo models` lists all available Gemini image models
- [ ] `fingo key save <key>` stores key; `fingo key show` prints masked key
- [ ] Missing API key gives clear error: `Error: Gemini API key not found. Run: fingo key save <key>`
- [ ] `-o output.png` saves to specified path
- [ ] `-m nano-banana-pro-preview` overrides default model
- [ ] `finishReason` errors surface cleanly (not a panic)
- [ ] `cargo clippy` passes with no warnings
- [ ] Release binary is <5MB stripped

## Gotchas (from institutional knowledge)

1. **`inlineData` is camelCase** in the response — `serde` field aliasing or manual extraction needed. Check both `inlineData` and `inline_data`.
2. **`responseModalities: ["image", "text"]` is required** — without it, no image output.
3. **Check `finishReason` before `.content.parts`** — RECITATION returns empty content, causes panic if you skip this check.
4. **Codex sandbox has no network** — write source in Codex, then `cargo build --release` in a normal shell. Use `cargo clean -p fingo` if build seems stale.
5. **`ureq` v3 POST with JSON**: `ureq::post(url).send_json(&payload)` — not `.call()`.
6. **Timeout**: set 60s timeout for image requests — generation can be slow.

## Implementation Order

1. `error.rs` — AppError enum
2. `keychain.rs` — read/write key via `security`
3. `main.rs` — Cli/Commands with clap (stub `run()` branches)
4. `api.rs` — POST to Gemini, base64 encode input, decode output image
5. `models.rs` — GET /v1beta/models, filter image-capable ones
6. Wire everything in `main.rs`, handle all error cases
7. `cargo clippy && cargo test`

## Verification

```bash
cd ~/code/fingo
cargo build --release 2>&1 | tail -3
./target/release/fingo key show
./target/release/fingo models
./target/release/fingo remove /tmp/yt_thumb.jpg "remove the FOCUS text" -o /tmp/fingo-test.jpg
open /tmp/fingo-test.jpg
```

## Sources & References

- `~/docs/solutions/gemini-image-editing-inpainting.md` — working API reference + model list
- `~/docs/solutions/rust-gotchas.md` — Rust patterns and Codex delegation gotchas
- `~/docs/solutions/rust-toolchain-setup.md` — release profile, pre-publish checklist
- `~/docs/solutions/credential-isolation-keychain.md` — keychain service name + shell-out pattern
- `~/docs/solutions/gemini-recitation-filter.md` — finishReason handling
- `~/code/stips/src/` — reference implementation for clap, ureq, AppError, keychain pattern
