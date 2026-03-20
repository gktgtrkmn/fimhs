use core::{compare_snapshots, FileMeta, Snapshot};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const SNAPSHOT_FILE: &str = ".fim_snapshot.json";

fn build_snapshot<P: AsRef<Path>>(dir: P) -> std::io::Result<Snapshot> {
    let mut snapshot: BTreeMap<String, FileMeta> = Snapshot::new();
    visit_dirs(dir.as_ref(), &mut snapshot)?;
    Ok(snapshot)
}

fn visit_dirs(dir: &Path, snapshot: &mut Snapshot) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry: fs::DirEntry = entry?;
            let path: std::path::PathBuf = entry.path();

            if let Some(file_name) = path.file_name() {
                if file_name == SNAPSHOT_FILE {
                    continue;
                }
            }

            if path.is_dir() {
                visit_dirs(&path, snapshot)?;
            } else {
                let metadata: fs::Metadata = entry.metadata()?;
                let size: u64 = metadata.len();
                let modified: std::time::SystemTime = metadata.modified()?;

                if let Some(path_str) = path.to_str() {
                    snapshot.insert(
                        path_str.to_string(),
                        FileMeta { size, modified },
                    );
                }
            }
        }
    }
    Ok(())
}
