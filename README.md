# Stredio-Heart

`Stredio-Heart` is a Rust crate that holds the reusable logic shared across every version of [STREDIO](https://github.com/Shon1a/Stredio) — web, desktop, mobile, TV. Write a rule once; every client behaves the same.

The crate is **pure and I/O-free** (`#![forbid(unsafe_code)]`): bytes in, bytes out. Network, storage, and rendering belong to the shell. It builds to a native library and to WebAssembly, so a browser can drive it directly.

## Modules

| Module | Responsibility |
|--------|----------------|
| `types` | Shared data model — add-on descriptors, manifests, collections, catalog/library items, progress. |
| `addon` | Add-on URL normalisation, manifest validation, capability checks. |
| `collection` | Merge the official add-on list over the built-in defaults — display-only refine, additive-only, with input guards. |
| `state` | Per-account install state + last-write-wins cross-device sync. |
| `runtime` · `catalog` · `library` | Elm-style state slices — add-ons, home rows, watch progress. |
| `ffi` · `wasm` | JSON string-in/out wrappers, and the `wasm` feature's JS classes. |

## Elm-style state

`runtime`, `catalog`, and `library` share one shape: a `Model`, a `Msg` enum, and a pure `update(&mut Model, Msg) -> Vec<Effect>`. State flows one way — the shell runs the returned `Effect`s (fetch / persist / repaint) and feeds results back as messages. The core never touches the network or the clock.

## Build

```bash
cargo test                                    # all modules
cargo clippy --all-targets -- -D warnings
cargo build --release                         # native lib + cdylib

# browser (WebAssembly):
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --features wasm
wasm-bindgen target/wasm32-unknown-unknown/release/stredio_heart.wasm --out-dir web --target web
```

Prebuilt WebAssembly glue is committed at [`web/`](web), so a shell can import it with no build step.

## Roadmap

- [x] Add-on, home-rows, and library state slices + `wasm` bindings
- [ ] Stream + subtitle mapping
- [ ] Catalog paging & search
- [ ] More shells driving the core

## License

[MIT](./LICENSE) — code only; no media, no stream sources.
