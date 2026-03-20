use clap::{Parser, Subcommand};
use fim_core::{compare_snapshots, FileMeta, Snapshot};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const SNAPSHOT_FILE: &str = ".fim_snapshot.json";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Init { 
        dir: String, 
    },
    Check { 
        dir: String, 
    },
}

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

                if let Ok(canonical_path) = fs::canonicalize(&path) {
                    if let Some(path_str) = canonical_path.to_str() {
                        snapshot.insert(
                            path_str.to_string(),
                            FileMeta { size, modified },
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli: Cli = Cli::parse();

    match &cli.command {
        Command::Init { dir } => {
            println!("[*] Building baseline snapshot for '{}'...", dir);
            
            let snap: BTreeMap<String, FileMeta> = build_snapshot(dir)?;
            let json: String = serde_json::to_string_pretty(&snap)?;
            
            fs::write(SNAPSHOT_FILE, json)?;
            
            println!("[+] Snapshot saved to {}", SNAPSHOT_FILE);
        },
        Command::Check { dir } => {
            if !Path::new(SNAPSHOT_FILE).exists() {
                return Err("No snapshot found. Run 'init' first.".into());
            }

            println!("[*] Scanning current directory...");

            let data: String = fs::read_to_string(SNAPSHOT_FILE)?;
            let old_snap: Snapshot = serde_json::from_str(&data)?;

            let new_snap: BTreeMap<String, FileMeta> = build_snapshot(dir)?;
            let diff: BTreeMap<String, fim_core::Alert> = compare_snapshots(&old_snap, &new_snap);

            if diff.is_empty() {
                println!("[+] Integrity check passed: No unauthorized changes.");
            } else {
                println!("\n[!] ALERTS - TAMPERING DETECTED:");
                for (path, alert) in diff {
                    println!("  [{:?}] {}", alert, path);
                }
            }
        },
    }
    Ok(())
}
