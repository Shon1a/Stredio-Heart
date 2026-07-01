//! WebAssembly binding — lets a browser shell (e.g. `index.html`) drive the pure
//! core directly, instead of hand-writing the same state logic in JS.
//!
//! Enabled by the `wasm` feature. Each runtime is exposed as a JS class; its
//! methods take/return **JSON strings** so no hand-written bindings are needed.
//! Every method returns the effects to run as a JSON array (e.g.
//! `["Repaint"]`, `[{"FetchRow":{"cat":"top_movie"}}]`) — the shell performs the
//! I/O and feeds results back by calling the next method.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown --features wasm`
//! then run `wasm-bindgen` on the resulting `.wasm` to emit the JS glue.

use wasm_bindgen::prelude::*;

use crate::types::{AddonDescriptor, LibraryItem, MetaItem, Progress};
use crate::{catalog, library, runtime};
use std::collections::BTreeMap;

fn to_json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string())
}

// ---------------------------------------------------------------------------
// Stateless helpers (the JSON ffi, re-exported to JS).
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn merge_official_json(inline_json: &str, cdn_json: &str) -> String {
    crate::ffi::merge_official_json(inline_json, cdn_json)
}

#[wasm_bindgen]
pub fn official_payload_file(manifest_json: &str) -> String {
    crate::ffi::official_payload_file(manifest_json)
}

#[wasm_bindgen]
pub fn collection_addons_json(payload_json: &str) -> String {
    crate::ffi::collection_addons_json(payload_json)
}

// ---------------------------------------------------------------------------
// Add-on state runtime.
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub struct AddonRuntime {
    model: runtime::Model,
}

#[wasm_bindgen]
impl AddonRuntime {
    /// `inline_json` is the inline default add-on array (the boot/fallback set).
    #[wasm_bindgen(constructor)]
    pub fn new(inline_json: &str) -> AddonRuntime {
        let inline: Vec<AddonDescriptor> = serde_json::from_str(inline_json).unwrap_or_default();
        AddonRuntime {
            model: runtime::Model::new(inline),
        }
    }

    pub fn load_official(&mut self) -> String {
        to_json(&runtime::update(
            &mut self.model,
            runtime::Msg::LoadOfficial,
        ))
    }

    pub fn official_manifest_fetched(&mut self, json: Option<String>) -> String {
        let man = json.and_then(|s| serde_json::from_str(&s).ok());
        to_json(&runtime::update(
            &mut self.model,
            runtime::Msg::OfficialManifestFetched(man),
        ))
    }

    pub fn official_payload_fetched(&mut self, json: Option<String>) -> String {
        let col = json.and_then(|s| serde_json::from_str(&s).ok());
        to_json(&runtime::update(
            &mut self.model,
            runtime::Msg::OfficialPayloadFetched(col),
        ))
    }

    pub fn toggle_addon(&mut self, id: String, now: f64) -> String {
        to_json(&runtime::update(
            &mut self.model,
            runtime::Msg::ToggleAddon {
                id,
                now: now as u64,
            },
        ))
    }

    pub fn install_state_pulled(&mut self, map_json: &str, at: f64, owner_changed: bool) -> String {
        let map = serde_json::from_str(map_json).unwrap_or_default();
        to_json(&runtime::update(
            &mut self.model,
            runtime::Msg::InstallStatePulled {
                map,
                at: at as u64,
                owner_changed,
            },
        ))
    }

    /// Current add-on list (array of descriptors) as JSON — what the shell renders.
    pub fn addons_json(&self) -> String {
        to_json(&self.model.addons)
    }

    /// Current install map (id -> installed) as JSON.
    pub fn install_map_json(&self) -> String {
        to_json(&self.model.install_map())
    }

    /// Load status (`"Idle" | "Loading" | "Loaded" | "Failed"`).
    pub fn status(&self) -> String {
        to_json(&self.model.status)
    }
}

// ---------------------------------------------------------------------------
// Home-rows (catalog) runtime.
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub struct CatalogRuntime {
    model: catalog::Catalog,
}

impl Default for CatalogRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl CatalogRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new() -> CatalogRuntime {
        CatalogRuntime {
            model: catalog::Catalog::default(),
        }
    }

    pub fn set_gating(&mut self, catalog: bool, providers: bool, studios: bool) -> String {
        to_json(&catalog::update(
            &mut self.model,
            catalog::CatMsg::SetGating {
                catalog,
                providers,
                studios,
            },
        ))
    }

    pub fn hydrate_row_config(&mut self, cfg_json: &str) -> String {
        let cfg: BTreeMap<String, bool> = serde_json::from_str(cfg_json).unwrap_or_default();
        to_json(&catalog::update(
            &mut self.model,
            catalog::CatMsg::HydrateRowConfig(cfg),
        ))
    }

    pub fn toggle_row(&mut self, cat: String, on: bool) -> String {
        to_json(&catalog::update(
            &mut self.model,
            catalog::CatMsg::ToggleRow { cat, on },
        ))
    }

    pub fn load_home(&mut self) -> String {
        to_json(&catalog::update(&mut self.model, catalog::CatMsg::LoadHome))
    }

    pub fn row_fetched(&mut self, cat: String, items_json: Option<String>) -> String {
        let items = items_json.and_then(|s| serde_json::from_str::<Vec<MetaItem>>(&s).ok());
        to_json(&catalog::update(
            &mut self.model,
            catalog::CatMsg::RowFetched { cat, items },
        ))
    }

    pub fn hero_fetched(&mut self, item_json: Option<String>) -> String {
        let item = item_json.and_then(|s| serde_json::from_str::<MetaItem>(&s).ok());
        to_json(&catalog::update(
            &mut self.model,
            catalog::CatMsg::HeroFetched(item),
        ))
    }

    /// The whole home-rows model as JSON (gating, config, loaded rows, hero).
    pub fn snapshot_json(&self) -> String {
        to_json(&self.model)
    }

    /// The ordered list of visible row categories, as a JSON array of strings.
    pub fn visible_rows_json(&self) -> String {
        let cats: Vec<&str> = self.model.visible_rows().iter().map(|r| r.cat).collect();
        to_json(&cats)
    }
}

// ---------------------------------------------------------------------------
// Library / watch-progress runtime.
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub struct LibraryRuntime {
    model: library::Library,
}

impl Default for LibraryRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl LibraryRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new() -> LibraryRuntime {
        LibraryRuntime {
            model: library::Library::default(),
        }
    }

    /// Load a persisted `Library` (`{history, progress, removed}`) at boot.
    pub fn hydrate(&mut self, library_json: &str) -> String {
        let lib = serde_json::from_str(library_json).unwrap_or_default();
        to_json(&library::update(
            &mut self.model,
            library::LibMsg::Hydrate(lib),
        ))
    }

    /// Record a watched/opened title (`item_json` is a `LibraryItem`).
    pub fn record_watch(&mut self, item_json: &str) -> String {
        match serde_json::from_str::<LibraryItem>(item_json) {
            Ok(item) => to_json(&library::update(
                &mut self.model,
                library::LibMsg::RecordWatch(item),
            )),
            Err(_) => "[]".to_string(),
        }
    }

    pub fn set_progress(&mut self, id: String, pos: f64, dur: f64, now: f64) -> String {
        to_json(&library::update(
            &mut self.model,
            library::LibMsg::SetProgress {
                id,
                pos,
                dur,
                now: now as u64,
            },
        ))
    }

    pub fn remove(&mut self, id: String, now: f64) -> String {
        to_json(&library::update(
            &mut self.model,
            library::LibMsg::Remove {
                id,
                now: now as u64,
            },
        ))
    }

    /// Merge server state (`GET /api/library-state`) by recency.
    pub fn pulled(
        &mut self,
        history_json: &str,
        progress_json: &str,
        removed_json: &str,
        now: f64,
    ) -> String {
        let history: Vec<LibraryItem> = serde_json::from_str(history_json).unwrap_or_default();
        let progress: BTreeMap<String, Progress> =
            serde_json::from_str(progress_json).unwrap_or_default();
        let removed: library::Tombstones = serde_json::from_str(removed_json).unwrap_or_default();
        to_json(&library::update(
            &mut self.model,
            library::LibMsg::Pulled {
                history,
                progress,
                removed,
                now: now as u64,
            },
        ))
    }

    /// The whole library (`{history, progress, removed}`) as JSON — for local
    /// persistence and the server push body.
    pub fn snapshot_json(&self) -> String {
        to_json(&self.model)
    }

    /// The Continue-Watching items (unfinished), newest first, as JSON.
    pub fn continue_watching_json(&self) -> String {
        to_json(&self.model.continue_watching())
    }
}
