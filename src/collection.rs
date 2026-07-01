//! Merging a CDN-served official collection over the inline defaults.
//!
//! This is the portable heart of the "official add-on list" upgrade layer: the
//! inline defaults are the source of truth for behaviour-critical ids and their
//! install state; the CDN may only **refine display metadata** of known ids and
//! **append** brand-new curated official cards. It can never drop, lock,
//! re-section, or flip the toggle of a known id, and it can never introduce a
//! stream source (neutral-conduit stance).
//!
//! Ported verbatim in behaviour from the browser loader, with three hardenings
//! baked in: an `icon_cls` allow-list (it lands in a class attribute), exclusion
//! of behaviour-gating fields (`no_config`/`preview`) from the override
//! whitelist, and precise change detection so identical data is a true no-op.

use crate::types::{AddonDescriptor, Section};

/// What a merge did — lets a caller repaint only when something actually changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeReport {
    /// `true` iff any field was refined or any new card appended.
    pub changed: bool,
    /// Ids of brand-new cards appended.
    pub added: Vec<String>,
    /// Ids of known cards whose display metadata was refined.
    pub refined: Vec<String>,
    /// Ids (or `<empty-id>`) rejected by a guard (non-official, stream-bearing, id-less).
    pub skipped: Vec<String>,
}

/// Max length + charset for a CDN-supplied `icon_cls`. It is emitted into an HTML
/// `class` attribute by UI shells, so anything outside `[A-Za-z0-9 _-]` is dropped.
const ICON_MAX: usize = 40;

/// Returns the value only if it is a safe class token (else `None`).
pub fn safe_icon(v: &str) -> Option<String> {
    if v.len() <= ICON_MAX
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_' || c == '-')
    {
        Some(v.to_string())
    } else {
        None
    }
}

/// Does this descriptor carry a stream source? Curated official cards must not.
pub fn has_stream(a: &AddonDescriptor) -> bool {
    a.transport_url.is_some() || a.resources.iter().any(|r| r == "stream")
}

/// UI version string: prefer explicit `ver`, else derive `v{version}`.
fn ver_of(raw: &AddonDescriptor) -> Option<String> {
    if let Some(v) = &raw.ver {
        return Some(v.clone());
    }
    raw.version.as_ref().map(|v| format!("v{v}"))
}

/// Refine an existing (known) descriptor with display-only fields. Returns `true`
/// iff a field actually changed. `id`, `section`, `installed`, `locked`,
/// `no_config`, and `preview` are never copied onto a known entry.
fn upsert_known(cur: &mut AddonDescriptor, raw: &AddonDescriptor) -> bool {
    let mut changed = false;

    if !raw.name.is_empty() && raw.name != cur.name {
        cur.name = raw.name.clone();
        changed = true;
    }
    if let Some(v) = ver_of(raw) {
        if Some(&v) != cur.ver.as_ref() {
            cur.ver = Some(v);
            changed = true;
        }
    }
    if let Some(raw_icon) = raw.icon_cls.as_deref() {
        if let Some(ic) = safe_icon(raw_icon) {
            if Some(&ic) != cur.icon_cls.as_ref() {
                cur.icon_cls = Some(ic);
                changed = true;
            }
        }
    }
    if let Some(g) = &raw.glyph {
        if Some(g) != cur.glyph.as_ref() {
            cur.glyph = Some(g.clone());
            changed = true;
        }
    }
    if let Some(img) = &raw.img {
        if Some(img) != cur.img.as_ref() {
            cur.img = Some(img.clone());
            changed = true;
        }
    }
    // Only replace tags when the CDN actually supplies some — never wipe to empty.
    if !raw.tags.is_empty() && raw.tags != cur.tags {
        cur.tags = raw.tags.clone();
        changed = true;
    }

    changed
}

/// Build a brand-new curated official card from a CDN record, sanitising it.
fn coerce_new(raw: &AddonDescriptor) -> AddonDescriptor {
    let installed = raw.default_installed == Some(true) || raw.installed == Some(true);
    AddonDescriptor {
        id: raw.id.clone(),
        section: Section::Official,
        name: if raw.name.is_empty() {
            raw.id.clone()
        } else {
            raw.name.clone()
        },
        version: raw.version.clone(),
        ver: Some(ver_of(raw).unwrap_or_default()),
        icon_cls: Some(
            raw.icon_cls
                .as_deref()
                .and_then(safe_icon)
                .unwrap_or_else(|| "puzzle".to_string()),
        ),
        glyph: Some(raw.glyph.clone().unwrap_or_default()),
        img: raw.img.clone(),
        tags: raw.tags.clone(),
        default_installed: raw.default_installed,
        installed: Some(installed),
        no_config: Some(raw.no_config == Some(true)),
        preview: Some(raw.preview == Some(true)),
        locked: Some(raw.locked == Some(true)),
        kind: raw.kind.clone(),
        types: raw.types.clone(),
        resources: raw.resources.clone(),
        // Never carry a transport into a curated UI card (neutral-conduit).
        transport_url: None,
        config_ref: raw.config_ref.clone(),
        flags: raw.flags.clone(),
    }
}

/// Merge a CDN-served official collection into `inline` (the boot/fallback set).
///
/// Mutates `inline` in place and reports what happened. Guards skip records with
/// no id, non-official records, and any record that carries a stream source.
pub fn merge_official(inline: &mut Vec<AddonDescriptor>, cdn: &[AddonDescriptor]) -> MergeReport {
    let mut report = MergeReport::default();
    for raw in cdn {
        if raw.id.is_empty() {
            report.skipped.push("<empty-id>".to_string());
            continue;
        }
        if raw.section != Section::Official {
            report.skipped.push(raw.id.clone());
            continue;
        }
        if has_stream(raw) {
            report.skipped.push(raw.id.clone());
            continue;
        }
        match inline.iter().position(|x| x.id == raw.id) {
            Some(pos) => {
                if upsert_known(&mut inline[pos], raw) {
                    report.changed = true;
                    report.refined.push(raw.id.clone());
                }
            }
            None => {
                inline.push(coerce_new(raw));
                report.changed = true;
                report.added.push(raw.id.clone());
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desc(json: &str) -> AddonDescriptor {
        serde_json::from_str(json).unwrap()
    }

    fn four() -> Vec<AddonDescriptor> {
        serde_json::from_str(
            r#"[
          {"id":"upcoming","section":"official","name":"Upcoming","ver":"v1.3.0","iconCls":"puzzle","glyph":"A","tags":["catalog","metadata"],"installed":true,"noConfig":true,"preview":true},
          {"id":"studios","section":"official","name":"Studios","ver":"v1.0.0","iconCls":"puzzle","glyph":"B","tags":["catalog","metadata"],"installed":true,"noConfig":true,"preview":true},
          {"id":"catalog","section":"official","name":"Catalog","ver":"v1.0.0","iconCls":"puzzle","glyph":"C","tags":["catalog","metadata"],"installed":true},
          {"id":"providers","section":"official","name":"Providers","ver":"v1.0.0","iconCls":"puzzle","glyph":"D","tags":["catalog","metadata"],"installed":false}
        ]"#,
        )
        .unwrap()
    }

    #[test]
    fn identical_display_is_noop() {
        let mut inline = four();
        // CDN payload uses defaultInstalled + version instead of installed, same display.
        let cdn: Vec<AddonDescriptor> = serde_json::from_str(
            r#"[
          {"id":"upcoming","section":"official","name":"Upcoming","version":"1.3.0","ver":"v1.3.0","iconCls":"puzzle","glyph":"A","tags":["catalog","metadata"],"defaultInstalled":true,"noConfig":true,"preview":true,"kind":"discovery"},
          {"id":"studios","section":"official","name":"Studios","version":"1.0.0","ver":"v1.0.0","iconCls":"puzzle","glyph":"B","tags":["catalog","metadata"],"defaultInstalled":true,"noConfig":true,"preview":true,"kind":"discovery"},
          {"id":"catalog","section":"official","name":"Catalog","version":"1.0.0","ver":"v1.0.0","iconCls":"puzzle","glyph":"C","tags":["catalog","metadata"],"defaultInstalled":true,"kind":"discovery"},
          {"id":"providers","section":"official","name":"Providers","version":"1.0.0","ver":"v1.0.0","iconCls":"puzzle","glyph":"D","tags":["catalog","metadata"],"defaultInstalled":false,"kind":"discovery"}
        ]"#,
        )
        .unwrap();
        let report = merge_official(&mut inline, &cdn);
        assert!(!report.changed, "identical CDN data must be a no-op");
        assert_eq!(inline.len(), 4);
    }

    #[test]
    fn refines_display_but_never_behaviour() {
        let mut inline = four();
        let cdn = vec![desc(
            r#"{"id":"catalog","section":"official","name":"Trending & Top","ver":"v2.0.0","installed":false,"locked":true,"noConfig":true}"#,
        )];
        let report = merge_official(&mut inline, &cdn);
        let cat = inline.iter().find(|a| a.id == "catalog").unwrap();
        assert!(report.changed);
        assert_eq!(cat.name, "Trending & Top"); // display refined
        assert_eq!(cat.ver.as_deref(), Some("v2.0.0"));
        assert_eq!(cat.installed, Some(true)); // behaviour untouched
        assert_eq!(cat.locked, None); // never locked by CDN
        assert_eq!(cat.no_config, None); // Configure button preserved
    }

    #[test]
    fn rejects_xss_iconcls_on_known_id() {
        let mut inline = four();
        let cdn = vec![desc(
            r#"{"id":"catalog","section":"official","iconCls":"x\"><img src=y onerror=alert(1)>"}"#,
        )];
        merge_official(&mut inline, &cdn);
        let cat = inline.iter().find(|a| a.id == "catalog").unwrap();
        assert_eq!(cat.icon_cls.as_deref(), Some("puzzle")); // malicious value dropped
    }

    #[test]
    fn skips_stream_sources() {
        let mut inline = four();
        let cdn: Vec<AddonDescriptor> = serde_json::from_str(
            r#"[
          {"id":"pirate","section":"official","name":"P","transportUrl":"http://x/manifest.json"},
          {"id":"pirate2","section":"official","name":"P2","resources":["stream"]}
        ]"#,
        )
        .unwrap();
        let report = merge_official(&mut inline, &cdn);
        assert!(!inline.iter().any(|a| a.id == "pirate" || a.id == "pirate2"));
        assert_eq!(report.skipped, vec!["pirate", "pirate2"]);
        assert!(!report.changed);
    }

    #[test]
    fn appends_new_curated_card_and_derives_ver() {
        let mut inline = four();
        let cdn = vec![desc(
            r#"{"id":"nebula","section":"official","name":"Nebula","version":"2.1.0","tags":["catalog"],"defaultInstalled":true}"#,
        )];
        let report = merge_official(&mut inline, &cdn);
        let n = inline.iter().find(|a| a.id == "nebula").unwrap();
        assert_eq!(report.added, vec!["nebula"]);
        assert_eq!(n.ver.as_deref(), Some("v2.1.0"));
        assert_eq!(n.installed, Some(true));
        assert_eq!(n.transport_url, None);
    }

    #[test]
    fn skips_community_and_idless() {
        let mut inline = four();
        let cdn: Vec<AddonDescriptor> = serde_json::from_str(
            r#"[
          {"id":"","section":"official","name":"x"},
          {"id":"c1","section":"community","name":"community card"}
        ]"#,
        )
        .unwrap();
        let report = merge_official(&mut inline, &cdn);
        assert_eq!(inline.len(), 4);
        assert!(!report.changed);
        assert_eq!(report.skipped, vec!["<empty-id>", "c1"]);
    }
}
