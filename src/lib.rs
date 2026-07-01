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
//! - [`ffi`] — thin JSON string-in/string-out wrappers for non-Rust shells (web/WASM, mobile).

pub mod addon;
pub mod collection;
pub mod ffi;
pub mod runtime;
pub mod state;
pub mod types;

// Convenience re-exports.
pub use addon::{
    addon_base_url, manifest_has_resource, manifest_provides_stream, normalize_manifest_url,
    validate_manifest,
};
pub use collection::{has_stream, merge_official, safe_icon, MergeReport};
pub use runtime::{update, Effect, LoadStatus, Model, Msg};
pub use state::{apply_install_map, install_map, reconcile, InstallMap, SyncDecision};
pub use types::{
    AddonCollection, AddonDescriptor, AddonManifest, CollectionManifest, CollectionRef, Flags,
    Resource, Section,
};
