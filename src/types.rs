//! Core domain types — the shared data model every STREDIO client speaks.
//!
//! These mirror the JSON shapes used across the platform (the official add-on
//! collection, an add-on's own `manifest.json`, and per-account install state) so
//! the same bytes flow from CDN → core → any shell with zero transformation.

use serde::{Deserialize, Serialize};

/// Which shelf a card belongs to. `official` cards ship curated; `community`
/// cards are added by the user (by URL) at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Section {
    #[default]
    Official,
    Community,
}

/// Capability/marker flags carried alongside a descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Flags {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub official: Option<bool>,
    /// `true` marks a behaviour-critical entry a merge must never drop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected: Option<bool>,
}

/// A single official/curated add-on card.
///
/// Dual-purpose by design: it carries a canonical core (`id`, `version`,
/// `types`, `resources`, `flags`, `kind`) that a future device shell can consume
/// natively, **plus** the flat UI hints today's clients read directly (`ver`,
/// `icon_cls`, `glyph`, `tags`, `default_installed`, ...). Absent fields stay
/// `None`/empty so a partial CDN record never wipes an inline default.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AddonDescriptor {
    pub id: String,
    #[serde(default)]
    pub section: Section,
    #[serde(default)]
    pub name: String,

    /// Canonical semantic version, no leading `v` (e.g. `1.3.0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// UI-facing version string, with leading `v` (e.g. `v1.3.0`). Derived from
    /// `version` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ver: Option<String>,

    #[serde(rename = "iconCls", default, skip_serializing_if = "Option::is_none")]
    pub icon_cls: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glyph: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub img: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,

    /// Default install state — a *hint* only, honoured for genuinely new ids.
    /// Real per-account state lives in [`crate::state`], not here.
    #[serde(
        rename = "defaultInstalled",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_installed: Option<bool>,
    /// Legacy/runtime install flag (tolerated on input).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed: Option<bool>,

    #[serde(rename = "noConfig", default, skip_serializing_if = "Option::is_none")]
    pub no_config: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,

    /// `"discovery"` marks a metadata-only card (no stream sources).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub resources: Vec<String>,

    /// Present only if a card is (or becomes) a real network add-on. Its presence
    /// means "carries a transport", which the neutral-conduit merge rejects for
    /// curated official cards.
    #[serde(
        rename = "transportUrl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub transport_url: Option<String>,
    #[serde(rename = "configRef", default, skip_serializing_if = "Option::is_none")]
    pub config_ref: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<Flags>,
}

/// One entry in the manifest's `collections[]` — points at a payload file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionRef {
    pub id: String,
    #[serde(default)]
    pub section: Section,
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(rename = "protectedIds", default)]
    pub protected_ids: Vec<String>,
}

/// The small, eager-loaded manifest (`index.json`). Lists the payload files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionManifest {
    /// Schema major. Consumers ignore any value other than `1` and fall back to
    /// their inline defaults (version negotiation).
    pub schema: u32,
    #[serde(default)]
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdn: Option<String>,
    #[serde(default)]
    pub collections: Vec<CollectionRef>,
}

impl CollectionManifest {
    /// The `official` collection reference, if the manifest is understood.
    pub fn official(&self) -> Option<&CollectionRef> {
        if self.schema != 1 {
            return None;
        }
        self.collections
            .iter()
            .find(|c| c.section == Section::Official && !c.file.is_empty())
    }
}

/// A collection payload file (`addons.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddonCollection {
    pub schema: u32,
    #[serde(default)]
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(default)]
    pub section: Section,
    #[serde(default)]
    pub addons: Vec<AddonDescriptor>,
}

/// A `resources` entry in a real add-on `manifest.json` — either a short string
/// (`"stream"`) or the full form (`{ "name": "stream", "types": [...] }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Resource {
    Short(String),
    Full {
        name: String,
        #[serde(default)]
        types: Vec<String>,
        #[serde(rename = "idPrefixes", default)]
        id_prefixes: Vec<String>,
    },
}

impl Resource {
    pub fn name(&self) -> &str {
        match self {
            Resource::Short(s) => s,
            Resource::Full { name, .. } => name,
        }
    }
    pub fn types(&self) -> &[String] {
        match self {
            Resource::Short(_) => &[],
            Resource::Full { types, .. } => types,
        }
    }
}

/// A real third-party add-on's own `manifest.json` (the thing a user installs by
/// URL). Only the fields the core needs are modelled.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AddonManifest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default)]
    pub resources: Vec<Resource>,
    #[serde(default)]
    pub types: Vec<String>,
}

/// A poster/catalog item — the shape a home row or search result renders.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MetaItem {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rating: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ep: Option<String>,
}

/// A library / history entry — a [`MetaItem`] plus per-user watch bookkeeping.
/// Matches the flat shape persisted per account (id/title/poster/... + at/key/season/episode).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LibraryItem {
    #[serde(flatten)]
    pub meta: MetaItem,
    /// Stable render key some rows use (usually equal to `meta.id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Last-touched timestamp (ms) — drives recency ordering and merge.
    #[serde(default)]
    pub at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode: Option<u32>,
}

impl LibraryItem {
    pub fn id(&self) -> &str {
        &self.meta.id
    }
}

/// Resume position for a title (seconds), with the time it was recorded (ms).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Progress {
    pub pos: f64,
    pub dur: f64,
    pub at: u64,
}

impl Progress {
    /// Fraction watched in `[0, 1]` (0 when the duration is unknown).
    pub fn fraction(&self) -> f64 {
        if self.dur > 0.0 {
            (self.pos / self.dur).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
    /// Heuristic "basically finished" cutoff — used to hide finished titles from
    /// the Continue-Watching row.
    pub fn is_finished(&self) -> bool {
        self.fraction() >= 0.9
    }
}
