use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use closecall_core::platform::Discovered;
use closecall_core::{ProviderId, Target, TargetId, TargetKindTag};

pub(crate) struct Entry {
    pub provider: usize,
    pub key: String,
    pub target: Target,
}

/// Warm inventory: provider key to engine `TargetId`, with `first_seen_ms` from the engine's
/// monotonic clock.
#[derive(Default)]
pub(crate) struct Index {
    keys: HashMap<(usize, String), TargetId>,
    entries: BTreeMap<TargetId, Entry>,
    last_id: u64,
}

impl Index {
    /// `Some` when the target is new or its content changed, which is what share-guard acts on.
    pub fn upsert(
        &mut self,
        provider: usize,
        provider_id: &ProviderId,
        found: Discovered,
        now_ms: u64,
    ) -> Option<TargetId> {
        let mut target = found.target;
        target.provider = provider_id.clone();
        match self.keys.get(&(provider, found.key.clone())) {
            Some(&id) => {
                let entry = self.entries.get_mut(&id)?;
                target.id = id;
                target.first_seen_ms = entry.target.first_seen_ms;
                if entry.target == target {
                    return None;
                }
                entry.target = target;
                Some(id)
            }
            None => {
                self.last_id += 1;
                let id = TargetId(self.last_id);
                target.id = id;
                target.first_seen_ms = now_ms;
                self.keys.insert((provider, found.key.clone()), id);
                self.entries.insert(
                    id,
                    Entry {
                        provider,
                        key: found.key,
                        target,
                    },
                );
                Some(id)
            }
        }
    }

    pub fn remove_key(&mut self, provider: usize, key: &str) {
        if let Some(id) = self.keys.remove(&(provider, key.to_owned())) {
            self.entries.remove(&id);
        }
    }

    pub fn remove(&mut self, id: TargetId) {
        if let Some(entry) = self.entries.remove(&id) {
            self.keys.remove(&(entry.provider, entry.key));
        }
    }

    /// A target re-read right before acting keeps its id and age.
    pub fn replace(&mut self, mut target: Target) {
        if let Some(entry) = self.entries.get_mut(&target.id) {
            target.first_seen_ms = entry.target.first_seen_ms;
            target.provider = entry.target.provider.clone();
            entry.target = target;
        }
    }

    /// Applies a full snapshot of one provider and returns what was added or changed.
    pub fn reconcile(
        &mut self,
        provider: usize,
        provider_id: &ProviderId,
        found: Vec<Discovered>,
        now_ms: u64,
    ) -> Vec<TargetId> {
        let present: HashSet<String> = found.iter().map(|d| d.key.clone()).collect();
        let gone: Vec<TargetId> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.provider == provider && !present.contains(&entry.key))
            .map(|(id, _)| *id)
            .collect();
        for id in gone {
            self.remove(id);
        }
        found
            .into_iter()
            .filter_map(|discovered| self.upsert(provider, provider_id, discovered, now_ms))
            .collect()
    }

    pub fn get(&self, id: TargetId) -> Option<&Entry> {
        self.entries.get(&id)
    }

    pub fn contains(&self, id: TargetId) -> bool {
        self.entries.contains_key(&id)
    }

    pub fn targets(&self, scope: Option<&BTreeSet<TargetId>>) -> Vec<(usize, Target)> {
        self.entries
            .values()
            .filter(|entry| scope.is_none_or(|ids| ids.contains(&entry.target.id)))
            .map(|entry| (entry.provider, entry.target.clone()))
            .collect()
    }

    pub fn ages(&self) -> Vec<(TargetId, u64)> {
        self.entries
            .values()
            .map(|entry| (entry.target.id, entry.target.first_seen_ms))
            .collect()
    }

    /// Kinds with at least one target, in a fixed order.
    pub fn counts(&self, provider: usize) -> Vec<(TargetKindTag, usize)> {
        [
            TargetKindTag::Tab,
            TargetKindTag::Document,
            TargetKindTag::AppWindow,
            TargetKindTag::Process,
        ]
        .into_iter()
        .map(|kind| {
            let count = self
                .entries
                .values()
                .filter(|e| e.provider == provider && e.target.kind.tag() == kind)
                .count();
            (kind, count)
        })
        .filter(|(_, count)| *count > 0)
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use closecall_core::testing::{TargetExt, tab};

    use super::*;

    fn found(key: &str, url: &str) -> Discovered {
        Discovered {
            key: key.to_owned(),
            target: tab(url, "t").with_provider("p"),
        }
    }

    #[test]
    fn keeps_id_and_age_across_changes_and_reports_only_real_changes() {
        let provider = ProviderId("p".to_owned());
        let mut index = Index::default();
        let id = index.upsert(0, &provider, found("a", "https://a.example/"), 5);
        assert_eq!(id, Some(TargetId(1)));
        assert_eq!(
            index.upsert(0, &provider, found("a", "https://a.example/"), 9),
            None
        );
        assert_eq!(
            index.upsert(0, &provider, found("a", "https://b.example/"), 9),
            Some(TargetId(1))
        );
        let entry = index.get(TargetId(1)).map(|e| e.target.first_seen_ms);
        assert_eq!(entry, Some(5));
    }

    #[test]
    fn reconcile_removes_missing_keys_of_that_provider_only() {
        let provider = ProviderId("p".to_owned());
        let other = ProviderId("q".to_owned());
        let mut index = Index::default();
        index.upsert(0, &provider, found("a", "https://a.example/"), 0);
        index.upsert(0, &provider, found("b", "https://b.example/"), 0);
        index.upsert(1, &other, found("a", "https://a.example/"), 0);
        let changed = index.reconcile(0, &provider, vec![found("b", "https://b.example/")], 1);
        assert!(changed.is_empty());
        assert_eq!(index.targets(None).len(), 2);
        assert!(!index.contains(TargetId(1)));
    }
}
