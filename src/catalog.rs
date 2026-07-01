//! An Elm-style slice for the **home rows** (catalog / provider / studio rails).
//!
//! Same shape as [`crate::runtime`]: a [`Catalog`] model, a [`CatMsg`] set, a pure
//! [`update`], and [`CatEffect`]s the shell runs. It reuses the platform's fixed
//! row list ([`HOME_ROWS`]) and computes the *visible* rows from two inputs:
//! which gating add-ons are installed (fed in from the add-on runtime) and the
//! per-row on/off config the user set. Row data itself is fetched by the shell
//! (a [`CatEffect::FetchRow`] per visible row) and handed back via [`CatMsg`].

use crate::runtime::LoadStatus;
use crate::types::MetaItem;
use std::collections::BTreeMap;

/// The kind of a home row, which decides which add-on gates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum RowKind {
    /// Gated by the `catalog` add-on (the Trending / Top-Rated block).
    Catalog,
    /// Gated by the `providers` add-on (the streaming-service block).
    Provider,
    /// Gated by the `studios` add-on (the studio-logo row).
    Studio,
}

/// A home-row definition (mirrors `HOME_ROWS` in the web client).
#[derive(Debug, Clone, Copy)]
pub struct RowDef {
    /// Stable category key (also the `/api/browse` category and config key).
    pub cat: &'static str,
    /// i18n key for the row heading.
    pub key: &'static str,
    pub kind: RowKind,
}

/// The fixed home-row order. Visibility is computed per-render; this is the source list.
pub const HOME_ROWS: &[RowDef] = &[
    RowDef {
        cat: "trending_movie",
        key: "sec.trending_movies",
        kind: RowKind::Catalog,
    },
    RowDef {
        cat: "trending_tv",
        key: "sec.trending_shows",
        kind: RowKind::Catalog,
    },
    RowDef {
        cat: "top_movie",
        key: "sec.top_movies",
        kind: RowKind::Catalog,
    },
    RowDef {
        cat: "top_tv",
        key: "sec.top_shows",
        kind: RowKind::Catalog,
    },
    RowDef {
        cat: "trending_anime",
        key: "sec.trending_anime",
        kind: RowKind::Catalog,
    },
    RowDef {
        cat: "top_anime",
        key: "sec.top_anime",
        kind: RowKind::Catalog,
    },
    RowDef {
        cat: "prov_netflix",
        key: "sec.netflix",
        kind: RowKind::Provider,
    },
    RowDef {
        cat: "prov_disney",
        key: "sec.disney",
        kind: RowKind::Provider,
    },
    RowDef {
        cat: "prov_prime",
        key: "sec.prime",
        kind: RowKind::Provider,
    },
    RowDef {
        cat: "prov_apple",
        key: "sec.apple",
        kind: RowKind::Provider,
    },
    RowDef {
        cat: "prov_max",
        key: "sec.max",
        kind: RowKind::Provider,
    },
    RowDef {
        cat: "prov_paramount",
        key: "sec.paramount",
        kind: RowKind::Provider,
    },
    RowDef {
        cat: "prov_crunchyroll",
        key: "sec.crunchyroll",
        kind: RowKind::Provider,
    },
    RowDef {
        cat: "studios",
        key: "sec.studios",
        kind: RowKind::Studio,
    },
];

/// Loaded state for a single row.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct RowData {
    pub status: LoadStatus,
    pub items: Vec<MetaItem>,
}

/// Home-rows state.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Catalog {
    /// Gating: is the `catalog` add-on installed? (fed from the add-on runtime)
    pub catalog_installed: bool,
    /// Gating: is the `providers` add-on installed?
    pub providers_installed: bool,
    /// Gating: is the `studios` add-on installed?
    pub studios_installed: bool,
    /// Per-row enable flags (absent == enabled). Persisted by the shell.
    pub row_config: BTreeMap<String, bool>,
    /// Loaded data per category.
    pub rows: BTreeMap<String, RowData>,
    /// The hero/spotlight item, if loaded.
    pub hero: Option<MetaItem>,
}

impl Catalog {
    /// Is a category enabled in the per-row config? (default: yes)
    pub fn row_enabled(&self, cat: &str) -> bool {
        self.row_config.get(cat) != Some(&false)
    }

    fn kind_installed(&self, kind: RowKind) -> bool {
        match kind {
            RowKind::Catalog => self.catalog_installed,
            RowKind::Provider => self.providers_installed,
            RowKind::Studio => self.studios_installed,
        }
    }

    /// The ordered rows that should currently render: gating add-on installed AND
    /// (for catalog/provider rows) the per-row toggle on. The studio row has no
    /// per-row toggle.
    pub fn visible_rows(&self) -> Vec<&'static RowDef> {
        HOME_ROWS
            .iter()
            .filter(|r| self.kind_installed(r.kind))
            .filter(|r| matches!(r.kind, RowKind::Studio) || self.row_enabled(r.cat))
            .collect()
    }
}

/// Things that can happen to the home rows.
#[derive(Debug, Clone)]
pub enum CatMsg {
    /// Gating add-on install state changed (from the add-on runtime).
    SetGating {
        catalog: bool,
        providers: bool,
        studios: bool,
    },
    /// Persisted per-row config loaded at boot.
    HydrateRowConfig(BTreeMap<String, bool>),
    /// User toggled a single row on/off in the Configure modal.
    ToggleRow { cat: String, on: bool },
    /// (Re)build the home: fetch every visible row + the hero.
    LoadHome,
    /// A row's data came back (`None` = fetch failed).
    RowFetched {
        cat: String,
        items: Option<Vec<MetaItem>>,
    },
    /// The hero item came back.
    HeroFetched(Option<MetaItem>),
}

/// Side-effects for the shell to run.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum CatEffect {
    /// `GET /api/browse?cat=<cat>`, then dispatch [`CatMsg::RowFetched`].
    FetchRow { cat: String },
    /// `GET /api/hero`, then dispatch [`CatMsg::HeroFetched`].
    FetchHero,
    /// Persist the per-row config (localStorage `stredio.catalogRows` / provider rows).
    PersistRowConfig,
    /// Repaint the home.
    Repaint,
}

/// Pure reducer for the home rows.
pub fn update(model: &mut Catalog, msg: CatMsg) -> Vec<CatEffect> {
    match msg {
        CatMsg::SetGating {
            catalog,
            providers,
            studios,
        } => {
            model.catalog_installed = catalog;
            model.providers_installed = providers;
            model.studios_installed = studios;
            vec![CatEffect::Repaint]
        }

        CatMsg::HydrateRowConfig(cfg) => {
            model.row_config = cfg;
            vec![CatEffect::Repaint]
        }

        CatMsg::ToggleRow { cat, on } => {
            model.row_config.insert(cat, on);
            vec![CatEffect::PersistRowConfig, CatEffect::Repaint]
        }

        CatMsg::LoadHome => {
            let mut fx = vec![CatEffect::FetchHero];
            for row in model.visible_rows() {
                if !matches!(row.kind, RowKind::Studio) {
                    model.rows.entry(row.cat.to_string()).or_default().status = LoadStatus::Loading;
                    fx.push(CatEffect::FetchRow {
                        cat: row.cat.to_string(),
                    });
                }
            }
            fx
        }

        CatMsg::RowFetched { cat, items } => match items {
            Some(items) => {
                model.rows.insert(
                    cat,
                    RowData {
                        status: LoadStatus::Loaded,
                        items,
                    },
                );
                vec![CatEffect::Repaint]
            }
            None => {
                model.rows.entry(cat).or_default().status = LoadStatus::Failed;
                vec![]
            }
        },

        CatMsg::HeroFetched(item) => {
            if item.is_some() {
                model.hero = item;
                vec![CatEffect::Repaint]
            } else {
                vec![]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> MetaItem {
        MetaItem {
            id: id.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn gating_hides_and_shows_blocks() {
        let mut m = Catalog::default();
        assert_eq!(m.visible_rows().len(), 0); // nothing installed -> no rows

        update(
            &mut m,
            CatMsg::SetGating {
                catalog: true,
                providers: false,
                studios: false,
            },
        );
        let vis = m.visible_rows();
        assert_eq!(vis.len(), 6); // only the six catalog rows
        assert!(vis.iter().all(|r| matches!(r.kind, RowKind::Catalog)));

        update(
            &mut m,
            CatMsg::SetGating {
                catalog: true,
                providers: true,
                studios: true,
            },
        );
        assert_eq!(m.visible_rows().len(), 14); // 6 + 7 + studio
    }

    #[test]
    fn per_row_toggle_hides_one_row_and_persists() {
        let mut m = Catalog::default();
        update(
            &mut m,
            CatMsg::SetGating {
                catalog: true,
                providers: false,
                studios: false,
            },
        );
        let fx = update(
            &mut m,
            CatMsg::ToggleRow {
                cat: "top_movie".into(),
                on: false,
            },
        );
        assert!(fx.contains(&CatEffect::PersistRowConfig));
        assert_eq!(m.visible_rows().len(), 5); // one catalog row hidden
        assert!(!m.visible_rows().iter().any(|r| r.cat == "top_movie"));
    }

    #[test]
    fn load_home_fetches_each_visible_row_plus_hero() {
        let mut m = Catalog::default();
        update(
            &mut m,
            CatMsg::SetGating {
                catalog: true,
                providers: false,
                studios: true,
            },
        );
        let fx = update(&mut m, CatMsg::LoadHome);
        // hero + 6 catalog rows (studio row has no data fetch)
        assert!(fx.contains(&CatEffect::FetchHero));
        let fetches = fx
            .iter()
            .filter(|e| matches!(e, CatEffect::FetchRow { .. }))
            .count();
        assert_eq!(fetches, 6);
        assert_eq!(m.rows["trending_movie"].status, LoadStatus::Loading);
    }

    #[test]
    fn row_fetched_stores_items() {
        let mut m = Catalog::default();
        let fx = update(
            &mut m,
            CatMsg::RowFetched {
                cat: "top_movie".into(),
                items: Some(vec![item("1"), item("2")]),
            },
        );
        assert_eq!(fx, vec![CatEffect::Repaint]);
        assert_eq!(m.rows["top_movie"].items.len(), 2);
        assert_eq!(m.rows["top_movie"].status, LoadStatus::Loaded);
    }

    #[test]
    fn row_fetch_failure_marks_failed_no_repaint() {
        let mut m = Catalog::default();
        let fx = update(
            &mut m,
            CatMsg::RowFetched {
                cat: "top_movie".into(),
                items: None,
            },
        );
        assert_eq!(fx, vec![]);
        assert_eq!(m.rows["top_movie"].status, LoadStatus::Failed);
    }
}
