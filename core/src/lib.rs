use std::collections::BTreeMap;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMeta {
    pub size: u64,
    pub modified: SystemTime,
}

pub type Snapshot = BTreeMap<String, FileMeta>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alert {
    Added,
    Modified,
    Deleted,
}

pub fn compare_snapshots(old: &Snapshot, new: &Snapshot) -> BTreeMap<String, Alert> {
    let mut diffs: BTreeMap<String, Alert> = BTreeMap::new();

    for path in new.keys().filter(|k| !old.contains_key(*k)) {
        diffs.insert(path.clone(), Alert::Added);
    }

    for path in old.keys().filter(|k| !new.contains_key(*k)) {
        diffs.insert(path.clone(), Alert::Deleted);
    }

    for (path, old_meta) in old.iter() {
        if let Some(new_meta) = new.get(path) {
            if old_meta != new_meta {
                diffs.insert(path.clone(), Alert::Modified);
            }
        }
    }

    diffs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_snapshots() {
        let mut old: BTreeMap<String, FileMeta> = Snapshot::new();
        let mut new: BTreeMap<String, FileMeta> = Snapshot::new();

        let base_time: SystemTime = SystemTime::UNIX_EPOCH;

        old.insert(
            "file_a.txt".into(),
            FileMeta {
                size: 100,
                modified: base_time,
            },
        );
        new.insert(
            "file_a.txt".into(),
            FileMeta {
                size: 100,
                modified: base_time,
            },
        );

        old.insert(
            "file_b.txt".into(),
            FileMeta {
                size: 200,
                modified: base_time,
            },
        );
        new.insert(
            "file_b.txt".into(),
            FileMeta {
                size: 250,
                modified: base_time,
            },
        );

        old.insert(
            "file_c.txt".into(),
            FileMeta {
                size: 300,
                modified: base_time,
            },
        );

        new.insert(
            "file_d.txt".into(),
            FileMeta {
                size: 400,
                modified: base_time,
            },
        );

        let diffs: BTreeMap<String, Alert> = compare_snapshots(&old, &new);

        assert_eq!(diffs.len(), 3);
        assert_eq!(diffs.get("file_b.txt"), Some(&Alert::Modified));
        assert_eq!(diffs.get("file_c.txt"), Some(&Alert::Deleted));
        assert_eq!(diffs.get("file_d.txt"), Some(&Alert::Added));
        assert_eq!(diffs.get("file_a.txt"), None);
    }
}
