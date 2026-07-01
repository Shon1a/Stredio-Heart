//! Pure helpers for working with a real (user-installed) add-on and its manifest:
//! URL normalisation, base-URL resolution, manifest validation, and capability
//! checks. All string-in / string-out — no I/O, so identical on every device.

use crate::types::AddonManifest;

/// STREDIO's own add-on deep-link scheme. A pasted `stredio://host/...` link is
/// treated as `https://host/...`. (Add extra aliases in the shell if needed.)
const ADDON_SCHEME: &str = "stredio://";

/// The base URL (directory) that an add-on's resource paths are relative to —
/// i.e. the manifest URL with its last path segment removed.
///
/// `https://a.co/x/manifest.json` → `https://a.co/x/`
pub fn addon_base_url(manifest_url: &str) -> String {
    match manifest_url.rfind('/') {
        Some(i) => manifest_url[..=i].to_string(),
        None => String::new(),
    }
}

/// Normalise a raw, user-pasted string into a canonical `.../manifest.json` URL,
/// or `None` if it is not a usable http(s) URL.
///
/// The URL is kept **byte-for-byte** (configured add-ons pack options into the
/// path, so re-encoding would corrupt them). A path already ending in `.json` is
/// returned as-is; otherwise `/manifest.json` is appended (preserving any query).
pub fn normalize_manifest_url(raw: &str) -> Option<String> {
    let mut url = raw.trim().to_string();
    if url.is_empty() {
        return None;
    }
    if let Some(rest) = url.strip_prefix(ADDON_SCHEME) {
        url = format!("https://{rest}");
    }

    let low = url.to_ascii_lowercase();
    let scheme_len = if low.starts_with("https://") {
        "https://".len()
    } else if low.starts_with("http://") {
        "http://".len()
    } else {
        return None;
    };
    // Require a host (something after `scheme://`).
    if url[scheme_len..].is_empty() {
        return None;
    }

    // Split off the query string; test the path portion for a `.json` suffix.
    let (path, qs) = match url.find('?') {
        Some(i) => (&url[..i], &url[i..]),
        None => (url.as_str(), ""),
    };
    if path.to_ascii_lowercase().ends_with(".json") {
        return Some(url.clone());
    }
    let trimmed = path.trim_end_matches('/');
    Some(format!("{trimmed}/manifest.json{qs}"))
}

/// Validate the shape of an add-on's `manifest.json`. `Ok(())` if usable, else a
/// human-readable reason.
pub fn validate_manifest(m: &AddonManifest) -> Result<(), String> {
    if m.id.is_empty() || !valid_id(&m.id) {
        return Err("Manifest \"id\" is missing or malformed".to_string());
    }
    if m.name.trim().is_empty() || m.name.len() > 200 {
        return Err("Manifest \"name\" is missing or too long".to_string());
    }
    if m.resources.is_empty() {
        return Err("Manifest missing \"resources\"".to_string());
    }
    if m.types.is_empty() {
        return Err("Manifest missing \"types\"".to_string());
    }
    Ok(())
}

/// `^[A-Za-z0-9][A-Za-z0-9._-]{0,200}$`
fn valid_id(id: &str) -> bool {
    let b = id.as_bytes();
    if b.is_empty() || b.len() > 201 {
        return false;
    }
    if !b[0].is_ascii_alphanumeric() {
        return false;
    }
    b[1..]
        .iter()
        .all(|&c| c.is_ascii_alphanumeric() || c == b'.' || c == b'_' || c == b'-')
}

/// Does the manifest advertise a `stream` resource?
pub fn manifest_provides_stream(m: &AddonManifest) -> bool {
    m.resources.iter().any(|r| r.name() == "stream")
}

/// Does the manifest advertise `resource`, optionally for `typ`?
pub fn manifest_has_resource(m: &AddonManifest, resource: &str, typ: Option<&str>) -> bool {
    let has = m.resources.iter().any(|r| r.name() == resource);
    has && typ.map_or(true, |t| m.types.iter().any(|x| x == t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_strips_last_segment() {
        assert_eq!(
            addon_base_url("https://a.co/x/manifest.json"),
            "https://a.co/x/"
        );
        assert_eq!(addon_base_url("https://a.co/"), "https://a.co/");
        assert_eq!(addon_base_url("noslash"), "");
    }

    #[test]
    fn normalize_variants() {
        assert_eq!(
            normalize_manifest_url("https://a.co/x/manifest.json").as_deref(),
            Some("https://a.co/x/manifest.json")
        );
        assert_eq!(
            normalize_manifest_url("https://a.co/x").as_deref(),
            Some("https://a.co/x/manifest.json")
        );
        assert_eq!(
            normalize_manifest_url("https://a.co/x/").as_deref(),
            Some("https://a.co/x/manifest.json")
        );
        // own scheme maps to https
        assert_eq!(
            normalize_manifest_url("stredio://a.co/x").as_deref(),
            Some("https://a.co/x/manifest.json")
        );
        // query preserved, config chars untouched
        assert_eq!(
            normalize_manifest_url("https://a.co/cfg=a|b,c/manifest.json?x=1").as_deref(),
            Some("https://a.co/cfg=a|b,c/manifest.json?x=1")
        );
        assert_eq!(
            normalize_manifest_url("https://a.co/addon?x=1").as_deref(),
            Some("https://a.co/addon/manifest.json?x=1")
        );
        // rejects
        assert_eq!(normalize_manifest_url(""), None);
        assert_eq!(normalize_manifest_url("   "), None);
        assert_eq!(normalize_manifest_url("ftp://a.co/x"), None);
        assert_eq!(normalize_manifest_url("https://"), None);
    }

    #[test]
    fn validate_manifest_cases() {
        let ok: AddonManifest = serde_json::from_str(
            r#"{"id":"com.x.addon","name":"X","resources":["catalog"],"types":["movie"]}"#,
        )
        .unwrap();
        assert!(validate_manifest(&ok).is_ok());

        let no_res: AddonManifest =
            serde_json::from_str(r#"{"id":"x","name":"X","resources":[],"types":["movie"]}"#)
                .unwrap();
        assert!(validate_manifest(&no_res).is_err());

        let bad_id: AddonManifest = serde_json::from_str(
            r#"{"id":"-bad","name":"X","resources":["catalog"],"types":["movie"]}"#,
        )
        .unwrap();
        assert!(validate_manifest(&bad_id).is_err());
    }

    #[test]
    fn resource_checks_short_and_full() {
        let m: AddonManifest = serde_json::from_str(
            r#"{"id":"x","name":"X","types":["movie","series"],
                "resources":["catalog",{"name":"stream","types":["movie"]}]}"#,
        )
        .unwrap();
        assert!(manifest_provides_stream(&m));
        assert!(manifest_has_resource(&m, "catalog", Some("movie")));
        assert!(!manifest_has_resource(&m, "meta", None));
        assert!(!manifest_has_resource(&m, "catalog", Some("channel")));
    }
}
