use std::collections::BTreeMap;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMeta {
    pub size: u64,
    pub modified: SystemTime,
    pub hash: Option<String>,
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
