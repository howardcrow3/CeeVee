//! The model catalog is data: swapping in a better small model is a config
//! change, not a code change. A signed, updatable copy comes with the content
//! manifests (TECHNICAL_PLAN 17.6); this is the copy built into the app.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    General,
    Embedding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub role: Role,
    pub name: String,
    pub file: String,
    pub size_bytes: u64,
    pub license: String,
    pub min_ram_gb: u64,
    pub default: bool,
    /// Download location. `None` until the file is published.
    pub url: Option<String>,
    /// Lowercase hex SHA-256. `None` until the file is published; downloads are refused without it.
    pub sha256: Option<String>,
}

impl ModelEntry {
    /// True when the entry has everything needed to download and verify it.
    pub fn is_downloadable(&self) -> bool {
        self.url.is_some() && self.sha256.as_ref().is_some_and(|h| h.len() == 64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub schema: u32,
    pub models: Vec<ModelEntry>,
}

impl Catalog {
    pub fn builtin() -> Catalog {
        serde_json::from_str(include_str!("../catalog.json"))
            .expect("built-in catalog is valid JSON")
    }

    pub fn get(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }

    /// The model to use for `role` on a machine with `ram_gb` of memory: the
    /// largest entry that fits, preferring the default when nothing larger fits.
    pub fn recommend(&self, role: Role, ram_gb: u64) -> Option<&ModelEntry> {
        let fits = |m: &&ModelEntry| m.role == role && m.min_ram_gb <= ram_gb;
        self.models
            .iter()
            .filter(fits)
            .max_by_key(|m| (m.min_ram_gb, m.default))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_catalog_parses_with_one_default_per_role() {
        let c = Catalog::builtin();
        for role in [Role::General, Role::Embedding] {
            assert_eq!(
                c.models
                    .iter()
                    .filter(|m| m.role == role && m.default)
                    .count(),
                1
            );
        }
        assert!(c
            .models
            .iter()
            .all(|m| m.license == "Apache-2.0" || m.license == "MIT"));
    }

    #[test]
    fn recommendation_follows_memory() {
        let c = Catalog::builtin();
        assert_eq!(c.recommend(Role::General, 6), None);
        assert_eq!(
            c.recommend(Role::General, 8).unwrap().id,
            "qwen3-4b-instruct-q4km"
        );
        assert_eq!(
            c.recommend(Role::General, 12).unwrap().id,
            "qwen3-4b-instruct-q4km"
        );
        assert_eq!(
            c.recommend(Role::General, 16).unwrap().id,
            "qwen3-8b-instruct-q4km"
        );
        assert_eq!(
            c.recommend(Role::Embedding, 8).unwrap().id,
            "bge-small-en-v1.5-q8"
        );
    }

    #[test]
    fn every_builtin_entry_is_pinned_to_a_url_and_a_sha256() {
        for m in &Catalog::builtin().models {
            let url = m.url.as_deref().unwrap_or("");
            let hash = m.sha256.as_deref().unwrap_or("");
            assert!(url.starts_with("https://"), "{}: url", m.id);
            assert!(
                hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()),
                "{}: sha256",
                m.id
            );
            assert!(m.is_downloadable(), "{}", m.id);
        }
    }

    #[test]
    fn an_entry_without_a_url_or_a_hash_is_not_downloadable() {
        let mut m = Catalog::builtin().models[0].clone();
        m.url = None;
        assert!(!m.is_downloadable());
        let mut m = Catalog::builtin().models[0].clone();
        m.sha256 = Some("abc".into());
        assert!(!m.is_downloadable());
    }
}
