//! A set contains prioritized data entries.
//!
//!
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::Hash;

/// The entry in the priority set.
/// The data field should be orderable, hashable and clonable as for fallback ordering
/// , data indexing and querying. The priority field should be orderable and copiable.
#[derive(Eq, PartialEq)]
struct Entry<D: Ord + Hash + Clone, P: Ord + Copy> {
    data: D,
    priority: P,
}

impl<D: Ord + Hash + Clone, P: Ord + Copy> PartialOrd for Entry<D, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<D: Ord + Hash + Clone, P: Ord + Copy> Ord for Entry<D, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.priority.cmp(&other.priority) {
            // if the priority is equal, fallback to data cmp.
            Ordering::Equal => self.data.cmp(&other.data),
            other => other,
        }
    }
}

/// A priority set with hybrid structures that providing efficient data querying and iteration.
pub struct PrioritySet<D: Ord + Clone + Hash, P: Ord + Copy> {
    /// Sorted Set via BTreeSet
    entries: BTreeSet<Entry<D, P>>,
    /// Entry index for quick query.
    indices: HashMap<D, P>,
}

impl<D: Ord + Clone + Hash, P: Ord + Copy> PrioritySet<D, P> {
    /// Create a new PrioritySet
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { entries: BTreeSet::new(), indices: HashMap::new() }
    }

    /// Insert an entry with priority, it overwrites the legacy priority if it was existed.
    pub fn insert(&mut self, data: D, priority: P) {
        // construct entry with new or legacy data.
        let entry = if let Some(legacy_priority) = self.indices.remove(&data) {
            // Remove the legacy one from the BTreeSet and update the priority.
            let mut legacy = Entry { data: data.clone(), priority: legacy_priority };
            self.entries.remove(&legacy);

            legacy.priority = priority;
            legacy
        } else {
            Entry { data, priority }
        };

        // insert to set.
        self.indices.insert(entry.data.clone(), entry.priority);
        self.entries.insert(entry);
    }

    /// Delete an entry from the set.
    pub fn delete(&mut self, data: &D) -> bool {
        let Some(entry) =
            self.indices.remove(data).map(|p| Entry { data: data.clone(), priority: p })
        else {
            return false;
        };

        self.entries.remove(&entry);
        true
    }

    /// Get the priority of an entry.
    pub fn priority(&self, data: &D) -> Option<P> {
        self.indices.get(data).cloned()
    }

    /// Check if it contains an entry.
    pub fn has(&self, data: &D) -> bool {
        self.indices.contains_key(&data)
    }

    /// Get the entry with the highest priority.
    pub fn top(&self) -> Option<(&D, &P)> {
        self.entries.iter().next().map(|entry| (&entry.data, &entry.priority))
    }

    /// Remove and return the highest priority entry.
    pub fn pop(&mut self) -> Option<(D, P)> {
        self.entries.pop_first().map(|entry| {
            self.indices.remove(&entry.data);
            (entry.data, entry.priority)
        })
    }

    /// Clear the set.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.indices.clear();
    }

    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Length of the set.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return an iterator over entries in priority-ascending order.
    pub fn iter(&self) -> impl Iterator<Item = (&D, &P)> {
        self.entries.iter().map(|entry| (&entry.data, &entry.priority))
    }

    /// Retain the entries with a filter.
    pub fn retain(&mut self, filter: impl Fn(&D) -> bool) {
        self.entries.retain(|entry| filter(&entry.data));
        self.indices.retain(|data, _| filter(data));
    }

    /// Remove entries which are not in `keep`, keep those overlapped untouched,
    /// and add new ones with default priority.
    pub fn reconcile(&mut self, keep: &[D], default: P) {
        let mut retained: HashSet<_> = keep.iter().collect();

        let to_remove = self
            .indices
            .keys()
            .filter(|key| !retained.remove(*key))
            .cloned()
            .collect::<Vec<_>>();

        for item in to_remove {
            let p = self.indices.remove(&item).unwrap();
            let e = Entry{data: item, priority: p};
            self.entries.remove(&e);
        }

        for item in retained {
            self.insert(item.clone(), default);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_insert_delete_and_iteration() {
        let mut ps = PrioritySet::new();

        let k1 = "k1";
        let k2 = "k2";
        ps.insert(k1, 20);
        ps.insert(k2, 10);

        let entries: Vec<_> = ps.iter().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(*entries[0].0, k2);
        assert_eq!(*entries[1].0, k1);

        ps.delete(&k1);

        let entries: Vec<_> = ps.iter().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(*entries[0].0, k2);

        ps.delete(&k2);
        let entries: Vec<_> = ps.iter().collect();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_update() {
        let mut ps = PrioritySet::new();
        let k1 = "k1";
        let k2 = "k2";
        ps.insert(k1, 20);
        assert_eq!(ps.priority(&k1).unwrap(), 20);
        ps.insert(k1, 40);
        assert_eq!(ps.priority(&k1).unwrap(), 40);

        ps.insert(k2, 10);
        assert_eq!(ps.priority(&k2).unwrap(), 10);
        ps.insert(k2, 20);
        assert_eq!(ps.priority(&k2).unwrap(), 20);

        let entries: Vec<_> = ps.iter().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(*entries[0].1, 20);
    }

    #[test]
    fn test_clear() {
        let mut ps = PrioritySet::new();

        ps.insert("k1", 100);
        ps.insert("k2", 50);

        ps.clear();

        assert_eq!(ps.len(), 0);
        assert!(ps.is_empty());
        assert!(ps.iter().next().is_none());
        assert!(ps.top().is_none());
    }

    #[test]
    fn test_retain() {
        let mut ps = PrioritySet::new();

        ps.insert("key1", 10);
        ps.insert("k2", 5);
        ps.insert("k3", 1);

        ps.retain(|key| key.starts_with("key"));

        assert_eq!(ps.len(), 1);
        assert!(!ps.has(&"k2"));
        assert!(!ps.has(&"k3"));
        assert!(ps.has(&"key1"));

        // Verify iteration order
        let entries: Vec<_> = ps.iter().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(*entries[0].0, "key1");
    }

    #[test]
    fn test_reconcile() {
        let mut ps = PrioritySet::new();

        let k1 = "k1";
        let k2 = "k2";
        ps.insert(k1, 100);
        ps.insert(k2, 50);

        let k3 = "k3";
        ps.reconcile(&[k1, k3], 20);

        let entries: Vec<_> = ps.iter().collect();
        assert_eq!(entries.len(), 2);
        assert!(entries
            .iter()
            .any(|e| *e.0 == k1 && *e.1 == 100));
        assert!(entries
            .iter()
            .any(|e| *e.0 == k3 && *e.1 == 20));
    }
}
