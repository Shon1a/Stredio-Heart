//! JSON string-in / string-out surface for non-Rust shells.
//!
//! The whole core is `serde`-typed, so the simplest portable boundary is JSON:
//! a web (WASM) or mobile shell passes JSON in and gets JSON back, with no
//! bindings to generate. These wrappers are intentionally infallible — malformed
//! input degrades to the safe default rather than throwing across the boundary.

use crate::collection::merge_official;
use crate::types::{AddonCollection, AddonDescriptor, CollectionManifest};

/// Merge a CDN official collection (`cdn_json`, an array of descriptors) over the
/// inline defaults (`inline_json`, an array of descriptors) and return the merged
/// array as JSON. Invalid input on either side degrades to the inline set.
pub fn merge_official_json(inline_json: &str, cdn_json: &str) -> String {
    let mut inline: Vec<AddonDescriptor> = serde_json::from_str(inline_json).unwrap_or_default();
    let cdn: Vec<AddonDescriptor> = serde_json::from_str(cdn_json).unwrap_or_default();
    merge_official(&mut inline, &cdn);
    serde_json::to_string(&inline).unwrap_or_else(|_| "[]".to_string())
}

/// Given the manifest JSON (`index.json`), return the payload file name for the
/// official collection (e.g. `"addons.json"`), or empty string if the manifest is
/// not understood (schema mismatch, no official collection, parse error).
pub fn official_payload_file(manifest_json: &str) -> String {
    serde_json::from_str::<CollectionManifest>(manifest_json)
        .ok()
        .and_then(|m| m.official().map(|c| c.file.clone()))
        .unwrap_or_default()
}

/// Parse a collection payload (`addons.json`) and return its `addons` array as
/// JSON, or `[]` if the payload is unusable (schema != 1, parse error).
pub fn collection_addons_json(payload_json: &str) -> String {
    match serde_json::from_str::<AddonCollection>(payload_json) {
        Ok(c) if c.schema == 1 => serde_json::to_string(&c.addons).unwrap_or_else(|_| "[]".into()),
        _ => "[]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_json_roundtrip_appends() {
        let inline = r#"[{"id":"catalog","section":"official","name":"Catalog","installed":true}]"#;
        let cdn = r#"[{"id":"nebula","section":"official","name":"Nebula","version":"2.0.0"}]"#;
        let out = merge_official_json(inline, cdn);
        assert!(out.contains("\"nebula\""));
        assert!(out.contains("\"catalog\""));
    }

    #[test]
    fn bad_input_degrades_to_inline() {
        let inline = r#"[{"id":"catalog","section":"official","name":"Catalog"}]"#;
        let out = merge_official_json(inline, "not json");
        assert!(out.contains("\"catalog\""));
    }

    #[test]
    fn manifest_and_payload_helpers() {
        let man = r#"{"schema":1,"version":"1","collections":[{"id":"official","section":"official","file":"addons.json"}]}"#;
        assert_eq!(official_payload_file(man), "addons.json");
        assert_eq!(
            official_payload_file(r#"{"schema":2,"version":"1","collections":[]}"#),
            ""
        );

        let payload = r#"{"schema":1,"version":"1","addons":[{"id":"upcoming","section":"official","name":"U"}]}"#;
        assert!(collection_addons_json(payload).contains("\"upcoming\""));
        assert_eq!(
            collection_addons_json(r#"{"schema":9,"version":"1","addons":[]}"#),
            "[]"
        );
    }
}
