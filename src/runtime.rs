//! An Elm-style (Model / Message / update) slice for **add-on state**.
//!
//! This is the shape a full `Stredio-Heart` runtime would grow into: one state
//! box ([`Model`]), a closed set of things that can happen ([`Msg`]), a single
//! **pure** [`update`] that folds a message into the model, and [`Effect`]s —
//! *descriptions* of side-effects the shell performs, feeding results back in as
//! new messages. Data flows one way: `Msg -> update -> (Model, [Effect])`.
//!
//! The core never does I/O. Fetching the CDN, writing local storage, calling the
//! server, and repainting are all the shell's job — it just runs the returned
//! [`Effect`]s and dispatches the resulting [`Msg`]s. That is what keeps this
//! identical on every device and trivially testable (see the tests below).

use crate::collection::merge_official;
use crate::state::{apply_install_map, install_map, reconcile, InstallMap, SyncDecision};
use crate::types::{AddonCollection, AddonDescriptor, CollectionManifest};

/// Where the CDN official-collection load is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum LoadStatus {
    #[default]
    Idle,
    Loading,
    /// Merged successfully (or the CDN was understood but had nothing new).
    Loaded,
    /// Fetch/parse/schema failure — the inline defaults stand.
    Failed,
}

/// The whole add-on state, in one place.
#[derive(Debug, Clone, Default)]
pub struct Model {
    /// Every add-on card. Seeded with the inline defaults at construction; the
    /// CDN merge refines/appends but never drops the protected ids.
    pub addons: Vec<AddonDescriptor>,
    /// Timestamp (ms) of the last local install-state write (for last-write-wins).
    pub install_at: u64,
    /// Account that owns the local install state (guards shared-browser clobber).
    pub owner: Option<String>,
    /// CDN official-collection load status.
    pub status: LoadStatus,
    /// `version` string from the loaded collection, if any.
    pub collection_version: Option<String>,
}

impl Model {
    /// Start from the inline default add-ons (the offline/boot source of truth).
    pub fn new(inline: Vec<AddonDescriptor>) -> Self {
        Model {
            addons: inline,
            ..Default::default()
        }
    }

    /// Is this add-on currently installed?
    pub fn is_installed(&self, id: &str) -> bool {
        self.addons
            .iter()
            .find(|a| a.id == id)
            .and_then(|a| a.installed)
            .unwrap_or(false)
    }

    /// The current install map (id -> installed), skipping locked entries.
    pub fn install_map(&self) -> InstallMap {
        install_map(&self.addons)
    }
}

/// Everything that can happen to the add-on state. The shell dispatches these;
/// results of [`Effect`]s come back as more of these.
#[derive(Debug, Clone)]
pub enum Msg {
    /// Boot: begin loading the official collection from the CDN.
    LoadOfficial,
    /// A persisted local install map was read at boot (the localStorage overlay).
    LocalStateLoaded { map: InstallMap, at: u64 },
    /// The manifest (`index.json`) came back (`None` = fetch/parse failed).
    OfficialManifestFetched(Option<CollectionManifest>),
    /// The payload (`addons.json`) came back (`None` = fetch/parse failed).
    OfficialPayloadFetched(Option<AddonCollection>),
    /// User flipped a card's install toggle. `now` is the shell-supplied clock (ms)
    /// — the core has no clock, so time enters through the message.
    ToggleAddon { id: String, now: u64 },
    /// Server install state arrived (from `GET /api/addon-state`).
    InstallStatePulled {
        map: InstallMap,
        at: u64,
        owner_changed: bool,
    },
}

/// A side-effect to perform. The core returns these; the shell executes them and
/// dispatches the follow-up [`Msg`]. Effects are plain data — no I/O in here.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum Effect {
    /// `GET <cdn>/index.json`, then dispatch [`Msg::OfficialManifestFetched`].
    FetchOfficialManifest,
    /// `GET <cdn>/<file>`, then dispatch [`Msg::OfficialPayloadFetched`].
    FetchOfficialPayload { file: String },
    /// Write the install map to local persistence (e.g. localStorage).
    PersistInstallState { map: InstallMap, at: u64 },
    /// Upload the install map to the server (`PUT /api/addon-state`).
    PushInstallState { map: InstallMap, at: u64 },
    /// Something visible changed — repaint the add-ons grid.
    Repaint,
}

/// The one pure reducer: fold `msg` into `model`, returning the effects to run.
///
/// Mirrors the shipped browser loader's rules exactly — inline stays canonical
/// for known ids, the CDN only refines/appends, identical data is a no-op, and
/// nothing here touches the network or the clock.
pub fn update(model: &mut Model, msg: Msg) -> Vec<Effect> {
    match msg {
        Msg::LoadOfficial => {
            model.status = LoadStatus::Loading;
            vec![Effect::FetchOfficialManifest]
        }

        Msg::LocalStateLoaded { map, at } => {
            apply_install_map(&mut model.addons, &map);
            model.install_at = at;
            vec![Effect::Repaint]
        }

        Msg::OfficialManifestFetched(Some(man)) => match man.official() {
            Some(c) => vec![Effect::FetchOfficialPayload {
                file: c.file.clone(),
            }],
            // schema mismatch or no official collection -> keep inline defaults.
            None => {
                model.status = LoadStatus::Failed;
                vec![]
            }
        },
        Msg::OfficialManifestFetched(None) => {
            model.status = LoadStatus::Failed;
            vec![]
        }

        Msg::OfficialPayloadFetched(Some(col)) if col.schema == 1 => {
            let report = merge_official(&mut model.addons, &col.addons);
            model.status = LoadStatus::Loaded;
            model.collection_version = Some(col.version.clone());
            // Change detection: identical CDN data repaints nothing.
            if report.changed {
                vec![Effect::Repaint]
            } else {
                vec![]
            }
        }
        Msg::OfficialPayloadFetched(_) => {
            model.status = LoadStatus::Failed;
            vec![]
        }

        Msg::ToggleAddon { id, now } => {
            let Some(a) = model.addons.iter_mut().find(|a| a.id == id) else {
                return vec![];
            };
            if a.locked == Some(true) {
                return vec![]; // protected/default cards never toggle
            }
            a.installed = Some(!a.installed.unwrap_or(false));
            model.install_at = now;
            let map = model.install_map();
            vec![
                Effect::PersistInstallState {
                    map: map.clone(),
                    at: now,
                },
                Effect::PushInstallState { map, at: now },
                Effect::Repaint,
            ]
        }

        Msg::InstallStatePulled {
            map,
            at,
            owner_changed,
        } => match reconcile(model.install_at, !map.is_empty(), at, owner_changed) {
            SyncDecision::AdoptRemote => {
                apply_install_map(&mut model.addons, &map);
                model.install_at = at;
                vec![
                    Effect::PersistInstallState {
                        map: model.install_map(),
                        at,
                    },
                    Effect::Repaint,
                ]
            }
            SyncDecision::UploadLocal => vec![Effect::PushInstallState {
                map: model.install_map(),
                at: model.install_at,
            }],
            SyncDecision::Noop => vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn four() -> Vec<AddonDescriptor> {
        serde_json::from_str(
            r#"[
              {"id":"upcoming","section":"official","name":"Upcoming","ver":"v1.3.0","installed":true},
              {"id":"studios","section":"official","name":"Studios","ver":"v1.0.0","installed":true},
              {"id":"catalog","section":"official","name":"Catalog","ver":"v1.0.0","installed":true},
              {"id":"providers","section":"official","name":"Providers","ver":"v1.0.0","installed":false}
            ]"#,
        )
        .unwrap()
    }

    #[test]
    fn load_flow_manifest_then_payload() {
        let mut m = Model::new(four());

        // 1. boot
        assert_eq!(
            update(&mut m, Msg::LoadOfficial),
            vec![Effect::FetchOfficialManifest]
        );
        assert_eq!(m.status, LoadStatus::Loading);

        // 2. manifest -> asks for the payload file
        let man: CollectionManifest = serde_json::from_str(
            r#"{"schema":1,"version":"1","collections":[{"id":"official","section":"official","file":"addons.json"}]}"#,
        )
        .unwrap();
        assert_eq!(
            update(&mut m, Msg::OfficialManifestFetched(Some(man))),
            vec![Effect::FetchOfficialPayload {
                file: "addons.json".into()
            }]
        );

        // 3. payload identical to inline -> Loaded, but NO repaint (no-op)
        let col: AddonCollection = serde_json::from_str(
            r#"{"schema":1,"version":"2026.07.01","section":"official","addons":[
                 {"id":"upcoming","section":"official","name":"Upcoming","ver":"v1.3.0","defaultInstalled":true},
                 {"id":"studios","section":"official","name":"Studios","ver":"v1.0.0","defaultInstalled":true},
                 {"id":"catalog","section":"official","name":"Catalog","ver":"v1.0.0","defaultInstalled":true},
                 {"id":"providers","section":"official","name":"Providers","ver":"v1.0.0","defaultInstalled":false}
               ]}"#,
        )
        .unwrap();
        let fx = update(&mut m, Msg::OfficialPayloadFetched(Some(col)));
        assert_eq!(fx, vec![]); // identical -> no repaint
        assert_eq!(m.status, LoadStatus::Loaded);
        assert_eq!(m.collection_version.as_deref(), Some("2026.07.01"));
        assert_eq!(m.addons.len(), 4);
    }

    #[test]
    fn payload_with_new_card_repaints_and_appends() {
        let mut m = Model::new(four());
        let col: AddonCollection = serde_json::from_str(
            r#"{"schema":1,"version":"1","section":"official","addons":[
                 {"id":"nebula","section":"official","name":"Nebula","version":"2.0.0"}
               ]}"#,
        )
        .unwrap();
        let fx = update(&mut m, Msg::OfficialPayloadFetched(Some(col)));
        assert_eq!(fx, vec![Effect::Repaint]);
        assert!(m.addons.iter().any(|a| a.id == "nebula"));
    }

    #[test]
    fn bad_manifest_keeps_inline() {
        let mut m = Model::new(four());
        // schema 2 -> official() returns None -> Failed, inline stands
        let man: CollectionManifest =
            serde_json::from_str(r#"{"schema":2,"version":"1","collections":[]}"#).unwrap();
        assert_eq!(
            update(&mut m, Msg::OfficialManifestFetched(Some(man))),
            vec![]
        );
        assert_eq!(m.status, LoadStatus::Failed);
        assert_eq!(m.addons.len(), 4);

        assert_eq!(update(&mut m, Msg::OfficialManifestFetched(None)), vec![]);
    }

    #[test]
    fn toggle_flips_and_emits_persist_push_repaint() {
        let mut m = Model::new(four());
        assert!(m.is_installed("upcoming"));
        let fx = update(
            &mut m,
            Msg::ToggleAddon {
                id: "upcoming".into(),
                now: 1000,
            },
        );
        assert!(!m.is_installed("upcoming"));
        assert_eq!(m.install_at, 1000);
        assert_eq!(fx.len(), 3);
        assert!(matches!(
            fx[0],
            Effect::PersistInstallState { at: 1000, .. }
        ));
        assert!(matches!(fx[1], Effect::PushInstallState { at: 1000, .. }));
        assert_eq!(fx[2], Effect::Repaint);
    }

    #[test]
    fn toggle_unknown_or_locked_is_inert() {
        let mut m = Model::new(four());
        // unknown id
        assert_eq!(
            update(
                &mut m,
                Msg::ToggleAddon {
                    id: "nope".into(),
                    now: 1
                }
            ),
            vec![]
        );
        // locked card
        m.addons.push(serde_json::from_str(r#"{"id":"cinemeta","section":"official","name":"C","locked":true,"installed":true}"#).unwrap());
        assert_eq!(
            update(
                &mut m,
                Msg::ToggleAddon {
                    id: "cinemeta".into(),
                    now: 5
                }
            ),
            vec![]
        );
        assert!(m.is_installed("cinemeta")); // unchanged
    }

    #[test]
    fn pull_adopts_newer_remote() {
        let mut m = Model::new(four());
        m.install_at = 10;
        let mut map = InstallMap::new();
        map.insert("upcoming".into(), false); // remote turned it off
        let fx = update(
            &mut m,
            Msg::InstallStatePulled {
                map,
                at: 20,
                owner_changed: false,
            },
        );
        assert!(!m.is_installed("upcoming")); // adopted
        assert_eq!(m.install_at, 20);
        assert!(fx.contains(&Effect::Repaint));
        assert!(fx
            .iter()
            .any(|e| matches!(e, Effect::PersistInstallState { at: 20, .. })));
    }

    #[test]
    fn pull_uploads_when_local_newer() {
        let mut m = Model::new(four());
        m.install_at = 30;
        let mut map = InstallMap::new();
        map.insert("upcoming".into(), false);
        let fx = update(
            &mut m,
            Msg::InstallStatePulled {
                map,
                at: 20,
                owner_changed: false,
            },
        );
        // local is newer -> we keep ours and push it up; nothing adopted
        assert!(m.is_installed("upcoming"));
        assert_eq!(fx.len(), 1);
        assert!(matches!(fx[0], Effect::PushInstallState { at: 30, .. }));
    }
}
