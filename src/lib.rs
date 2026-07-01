#![forbid(unsafe_code)]
//! # Stredio-Heart
//!
//! The reusable **heart** of STREDIO — device- and version-agnostic domain models
//! and pure logic shared across every client (web, desktop, mobile, TV). One core,
//! many thin shells.
//!
//! Everything here is pure and I/O-free: bytes in, bytes out. Transport, storage,
//! and rendering belong to the shell; the heart owns the model and the rules, so a
//! rule fixed once holds identically on every device.
//!
//! ## Crate layout
//!
//! - [`types`] — the shared data model (add-on descriptors, manifests, collections).
//! - [`addon`] — pure helpers for a real add-on: URL normalisation, validation, capabilities.
//! - [`collection`] — merging a CDN-served official collection over inline defaults.
//! - [`state`] — per-account install state and cross-device reconciliation.
//! - [`runtime`] — an Elm-style Model / Msg / update / Effect slice for add-on state.
//! - [`catalog`] — the same shape for the home rows (catalog / provider / studio rails).
//! - [`library`] — the same shape for watch progress + library history + sync.
//! - [`ffi`] — thin JSON string-in/string-out wrappers for non-Rust shells (web/WASM, mobile).
//! - `wasm` (feature) — JS classes that let a browser shell drive the runtimes directly.

pub mod addon;
pub mod catalog;
pub mod collection;
pub mod ffi;
pub mod library;
pub mod runtime;
pub mod state;
pub mod types;

#[cfg(feature = "wasm")]
pub mod wasm;

// Convenience re-exports.
pub use addon::{
    addon_base_url, manifest_has_resource, manifest_provides_stream, normalize_manifest_url,
    validate_manifest,
};
pub use catalog::{CatEffect, CatMsg, Catalog, RowData, RowDef, RowKind, HOME_ROWS};
pub use collection::{has_stream, merge_official, safe_icon, MergeReport};
pub use library::{LibEffect, LibMsg, Library, Tombstones};
pub use runtime::{Effect, LoadStatus, Model, Msg};
pub use state::{apply_install_map, install_map, reconcile, InstallMap, SyncDecision};
pub use types::{
    AddonCollection, AddonDescriptor, AddonManifest, CollectionManifest, CollectionRef, Flags,
    LibraryItem, MetaItem, Progress, Resource, Section,
};

// Each slice defines its own pure `update`; call them module-qualified to avoid a
// name clash: `runtime::update`, `catalog::update`, `library::update`.
