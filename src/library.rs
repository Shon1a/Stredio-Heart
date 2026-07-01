//! An Elm-style slice for the **library / watch progress**.
//!
//! Holds the recently-watched history, resume positions, and a tombstone map
//! (so a title removed on one device doesn't resurrect from another's older
//! copy). Merge-by-recency and the caps/TTL match the web client's sync exactly,
//! so this can back the same `/api/library-state` PUT/GET — the shell just does
//! the I/O and coalesces pushes; every rule lives here, pure and tested.

use crate::types::{LibraryItem, Progress};
use std::collections::BTreeMap;

/// Most recent history entries kept per account.
pub const HISTORY_CAP: usize = 60;
/// Most resume positions kept per account.
pub const PROGRESS_CAP: usize = 240;
/// Tombstones older than this are pruned on merge (30 days, ms).
pub const TOMB_TTL_MS: u64 = 30 * 24 * 3600 * 1000;

/// Tombstone map: id -> removal timestamp (ms).
pub type Tombstones = BTreeMap<String, u64>;

/// The library / watch state.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Library {
    /// Recently-watched items, newest first, capped at [`HISTORY_CAP`].
    #[serde(default)]
    pub history: Vec<LibraryItem>,
    /// Resume positions, keyed by id.
    #[serde(default)]
    pub progress: BTreeMap<String, Progress>,
    /// Tombstones for removed ids.
    #[serde(default)]
    pub removed: Tombstones,
}

impl Library {
    /// Resume position for a title, if any.
    pub fn resume(&self, id: &str) -> Option<Progress> {
        self.progress.get(id).copied()
    }

    /// Has this id been tombstoned?
    pub fn is_removed(&self, id: &str) -> bool {
        self.removed.contains_key(id)
    }

    /// Items to show in "Continue Watching": history entries that have an
    /// unfinished resume position, newest first (history is already ordered).
    pub fn continue_watching(&self) -> Vec<&LibraryItem> {
        self.history
            .iter()
            .filter(|it| {
                self.progress
                    .get(it.id())
                    .map(|p| !p.is_finished())
                    .unwrap_or(false)
            })
            .collect()
    }

    fn cap_history(&mut self) {
        self.history.sort_by(|a, b| b.at.cmp(&a.at));
        self.history.truncate(HISTORY_CAP);
    }

    fn cap_progress(&mut self) {
        if self.progress.len() <= PROGRESS_CAP {
            return;
        }
        let mut pairs: Vec<(String, Progress)> =
            self.progress.iter().map(|(k, v)| (k.clone(), *v)).collect();
        pairs.sort_by(|a, b| b.1.at.cmp(&a.1.at));
        pairs.truncate(PROGRESS_CAP);
        self.progress = pairs.into_iter().collect();
    }
}

/// Things that can happen to the library.
#[derive(Debug, Clone)]
pub enum LibMsg {
    /// Persisted local state loaded at boot.
    Hydrate(Library),
    /// A title was watched/opened — upsert it into history (its `at` is authoritative).
    RecordWatch(LibraryItem),
    /// Playback progress ticked. `now` is the shell-supplied clock (ms).
    SetProgress {
        id: String,
        pos: f64,
        dur: f64,
        now: u64,
    },
    /// User removed a title from the library.
    Remove { id: String, now: u64 },
    /// Server state arrived (`GET /api/library-state`) — merge by recency. `now`
    /// prunes expired tombstones.
    Pulled {
        history: Vec<LibraryItem>,
        progress: BTreeMap<String, Progress>,
        removed: Tombstones,
        now: u64,
    },
}

/// Side-effects for the shell to run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum LibEffect {
    /// Write the library to local persistence (shell reads the model).
    Persist,
    /// Schedule a coalesced server push (`PUT /api/library-state`).
    SchedulePush,
    /// Repaint the affected rows (Continue Watching / history).
    Repaint,
}

/// Pure reducer for the library.
pub fn update(model: &mut Library, msg: LibMsg) -> Vec<LibEffect> {
    match msg {
        LibMsg::Hydrate(lib) => {
            *model = lib;
            model.cap_history();
            model.cap_progress();
            vec![LibEffect::Repaint]
        }

        LibMsg::RecordWatch(item) => {
            let id = item.id().to_string();
            model.removed.remove(&id); // re-watching clears any tombstone
            model.history.retain(|it| it.id() != id);
            model.history.push(item);
            model.cap_history();
            vec![
                LibEffect::Persist,
                LibEffect::SchedulePush,
                LibEffect::Repaint,
            ]
        }

        LibMsg::SetProgress { id, pos, dur, now } => {
            model.progress.insert(id, Progress { pos, dur, at: now });
            model.cap_progress();
            vec![
                LibEffect::Persist,
                LibEffect::SchedulePush,
                LibEffect::Repaint,
            ]
        }

        LibMsg::Remove { id, now } => {
            model.removed.insert(id.clone(), now);
            model.history.retain(|it| it.id() != id);
            model.progress.remove(&id);
            vec![
                LibEffect::Persist,
                LibEffect::SchedulePush,
                LibEffect::Repaint,
            ]
        }

        LibMsg::Pulled {
            history,
            progress,
            removed,
            now,
        } => {
            let tomb = merge_tombstones(&model.removed, &removed, now);
            model.history = merge_history(&model.history, &history, &tomb);
            model.progress = merge_progress(&model.progress, &progress);
            model.removed = tomb;
            vec![LibEffect::Persist, LibEffect::Repaint]
        }
    }
}

/// Merge two tombstone maps (keep the newest removal per id) and prune entries
/// older than [`TOMB_TTL_MS`] relative to `now`.
pub fn merge_tombstones(a: &Tombstones, b: &Tombstones, now: u64) -> Tombstones {
    let mut out: Tombstones = BTreeMap::new();
    for (id, &at) in a.iter().chain(b.iter()) {
        let e = out.entry(id.clone()).or_insert(0);
        if at > *e {
            *e = at;
        }
    }
    out.retain(|_, &mut at| now.saturating_sub(at) <= TOMB_TTL_MS);
    out
}

/// Merge two history lists: newest entry wins per id, tombstoned ids (where the
/// tombstone is at-or-after the entry) are dropped, newest first, capped.
pub fn merge_history(a: &[LibraryItem], b: &[LibraryItem], tomb: &Tombstones) -> Vec<LibraryItem> {
    let mut by_id: BTreeMap<String, LibraryItem> = BTreeMap::new();
    for it in a.iter().chain(b.iter()) {
        let id = it.id().to_string();
        match by_id.get(&id) {
            Some(prev) if prev.at >= it.at => {}
            _ => {
                by_id.insert(id, it.clone());
            }
        }
    }
    let mut out: Vec<LibraryItem> = by_id
        .into_values()
        .filter(|it| tomb.get(it.id()).map(|&t| t < it.at).unwrap_or(true))
        .collect();
    out.sort_by(|x, y| y.at.cmp(&x.at));
    out.truncate(HISTORY_CAP);
    out
}

/// Merge two progress maps: newer `at` wins per id, capped by recency.
pub fn merge_progress(
    a: &BTreeMap<String, Progress>,
    b: &BTreeMap<String, Progress>,
) -> BTreeMap<String, Progress> {
    let mut out: BTreeMap<String, Progress> = BTreeMap::new();
    for (id, p) in a.iter().chain(b.iter()) {
        match out.get(id) {
            Some(prev) if prev.at >= p.at => {}
            _ => {
                out.insert(id.clone(), *p);
            }
        }
    }
    if out.len() > PROGRESS_CAP {
        let mut pairs: Vec<(String, Progress)> = out.into_iter().collect();
        pairs.sort_by(|x, y| y.1.at.cmp(&x.1.at));
        pairs.truncate(PROGRESS_CAP);
        out = pairs.into_iter().collect();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, at: u64) -> LibraryItem {
        LibraryItem {
            meta: crate::types::MetaItem {
                id: id.to_string(),
                title: id.to_string(),
                ..Default::default()
            },
            at,
            ..Default::default()
        }
    }

    #[test]
    fn record_watch_upserts_and_orders() {
        let mut m = Library::default();
        assert!(update(&mut m, LibMsg::RecordWatch(item("a", 100))).contains(&LibEffect::Persist));
        update(&mut m, LibMsg::RecordWatch(item("b", 200)));
        update(&mut m, LibMsg::RecordWatch(item("a", 300))); // re-watch a -> moves to front, no dup
        let ids: Vec<&str> = m.history.iter().map(|i| i.id()).collect();
        assert_eq!(ids, vec!["a", "b"]);
        assert_eq!(m.history[0].at, 300);
    }

    #[test]
    fn set_progress_and_continue_watching() {
        let mut m = Library::default();
        update(&mut m, LibMsg::RecordWatch(item("a", 100)));
        update(&mut m, LibMsg::RecordWatch(item("b", 90)));
        update(
            &mut m,
            LibMsg::SetProgress {
                id: "a".into(),
                pos: 30.0,
                dur: 100.0,
                now: 110,
            },
        );
        update(
            &mut m,
            LibMsg::SetProgress {
                id: "b".into(),
                pos: 99.0,
                dur: 100.0,
                now: 95,
            },
        ); // finished
        let cw: Vec<&str> = m.continue_watching().iter().map(|i| i.id()).collect();
        assert_eq!(cw, vec!["a"]); // b is finished -> excluded
        assert_eq!(m.resume("a").unwrap().pos, 30.0);
    }

    #[test]
    fn remove_tombstones_and_drops() {
        let mut m = Library::default();
        update(&mut m, LibMsg::RecordWatch(item("a", 100)));
        update(
            &mut m,
            LibMsg::SetProgress {
                id: "a".into(),
                pos: 5.0,
                dur: 100.0,
                now: 100,
            },
        );
        update(
            &mut m,
            LibMsg::Remove {
                id: "a".into(),
                now: 200,
            },
        );
        assert!(m.is_removed("a"));
        assert!(m.history.is_empty());
        assert!(m.resume("a").is_none());
    }

    #[test]
    fn pull_merges_by_recency_and_respects_tombstones() {
        let mut m = Library::default();
        update(&mut m, LibMsg::RecordWatch(item("a", 100)));
        // remote has a newer "a" and a new "b", but we tombstoned "b" later
        update(
            &mut m,
            LibMsg::Remove {
                id: "b".into(),
                now: 500,
            },
        );
        let remote_hist = vec![item("a", 300), item("b", 200)];
        let mut remote_prog = BTreeMap::new();
        remote_prog.insert(
            "a".to_string(),
            Progress {
                pos: 50.0,
                dur: 100.0,
                at: 300,
            },
        );
        let fx = update(
            &mut m,
            LibMsg::Pulled {
                history: remote_hist,
                progress: remote_prog,
                removed: BTreeMap::new(),
                now: 600,
            },
        );
        assert!(fx.contains(&LibEffect::Repaint));
        // "a" adopted at newer at; "b" stays tombstoned (removed at 500 > entry at 200)
        assert_eq!(m.history.len(), 1);
        assert_eq!(m.history[0].id(), "a");
        assert_eq!(m.history[0].at, 300);
        assert_eq!(m.resume("a").unwrap().pos, 50.0);
    }

    #[test]
    fn tombstone_ttl_prunes_old() {
        let mut old = Tombstones::new();
        old.insert("x".into(), 1000);
        let merged = merge_tombstones(&old, &Tombstones::new(), 1000 + TOMB_TTL_MS + 1);
        assert!(merged.is_empty()); // expired
        let fresh = merge_tombstones(&old, &Tombstones::new(), 1000 + 5);
        assert_eq!(fresh.get("x"), Some(&1000));
    }

    #[test]
    fn history_cap_enforced() {
        let mut m = Library::default();
        for i in 0..(HISTORY_CAP as u64 + 10) {
            update(&mut m, LibMsg::RecordWatch(item(&format!("id{i}"), i)));
        }
        assert_eq!(m.history.len(), HISTORY_CAP);
        assert_eq!(m.history[0].at, HISTORY_CAP as u64 + 9); // newest kept
    }
}
