# Stredio-Heart

The reusable **heart** of [STREDIO](https://github.com/Shon1a) — a Rust crate holding the
device- and version-agnostic domain models and pure logic shared across every client
(web, desktop, mobile, TV). **One core, many thin shells.**

The goal is that a rule written once — how the official add-on list merges, how an add-on
URL is normalised, how install state reconciles across devices — behaves *identically*
everywhere, instead of being re-implemented (and drifting) per platform.

## Design principles

- **Pure & I/O-free.** The heart never does network, disk, or UI. Bytes in, bytes out.
  Transport, storage, and rendering belong to the shell. This is what makes it portable
  and trivially testable.
- **`serde`-typed model.** Every type mirrors the JSON already used across the platform,
  so the same bytes flow from CDN → core → shell with zero transformation.
- **Portable to the browser.** The crate builds to a `cdylib`; a web shell can call it via
  WebAssembly. Because the surface is JSON-in/JSON-out (see [`ffi`](src/ffi.rs)), no
  hand-written bindings are needed.
- **Safe by construction.** `#![forbid(unsafe_code)]`. Guard rules (neutral-conduit, the
  `icon_cls` allow-list, "never override behaviour of protected ids") live in the core, so
  no shell can forget them.

## Modules

| Module | Responsibility |
|--------|----------------|
| `types` | The shared data model — add-on descriptors, real add-on manifests, collection manifest + payload, flags, sections. |
| `addon` | Pure helpers for a user-installed add-on: manifest-URL normalisation, base-URL resolution, manifest validation, resource/capability checks. |
| `collection` | Merging a CDN-served **official add-on collection** over the inline defaults — display-only refinement of known ids, additive-only new cards, neutral-conduit + XSS guards, precise change detection. |
| `state` | Per-account install state (`id -> installed`) and last-write-wins cross-device reconciliation. |
| `ffi` | Thin JSON string-in/string-out wrappers for non-Rust shells. |

## The official-addons connection

`collection::merge_official` is the native twin of the browser upgrade-layer that consumes
[`Stredio-official-addons`](https://github.com/Shon1a/Stredio-official-addons). Same rules,
one implementation:

- the inline defaults are the source of truth for behaviour-critical ids and their install state;
- the CDN may only **refine display metadata** of a known id and **append** new curated cards;
- it can never drop, lock, re-section, or flip the toggle of a known id;
- it can never introduce a stream source (metadata/discovery only);
- `icon_cls` is allow-listed (`[A-Za-z0-9 _-]`, ≤40) because shells render it into a class attribute;
- identical data is a true no-op (change detection), so a shell repaints only when something changed.

## Usage (Rust)

```rust,ignore
use stredio_heart::{merge_official, AddonDescriptor};

let mut inline: Vec<AddonDescriptor> = serde_json::from_str(inline_json)?;
let cdn: Vec<AddonDescriptor> = serde_json::from_str(cdn_json)?;
let report = merge_official(&mut inline, &cdn);
if report.changed {
    // repaint the add-ons grid; `report.added` / `report.refined` say what moved
}
```

## Usage (any shell, via JSON)

```rust,ignore
// merged array of descriptors as a JSON string; invalid input degrades to the inline set
let merged: String = stredio_heart::ffi::merge_official_json(inline_json, cdn_json);
```

## Build & test

```bash
cargo test            # unit tests for every module
cargo clippy --all-targets -- -D warnings
cargo build --release # rlib + cdylib
# web shell:  cargo build --release --target wasm32-unknown-unknown
```

## Roadmap

- `catalog` / `meta` item models and paging.
- Subtitle + stream mapping (the native twin of the client's `mapAddonStream`).
- Library model (watch progress, favourites) with the same reconcile discipline as `state`.
- A first-class WASM binding layer behind a `wasm` feature.

## License

MIT — see [`LICENSE`](./LICENSE). Covers code only; ships no media and no stream sources.
