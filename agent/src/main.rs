use clap::{Parser, Subcommand};
use core::{compare_snapshots, FileMeta, Snapshot};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::Path;
use std::io;

const SNAPSHOT_FILE: &str = ".fim_snapshot.json";

use jwalk::WalkDir;
use rayon::prelude::*;

#[derive(Parser, Debug)]
#[command(author, version, about = "File Integrity Monitor", long_about = None)]
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

fn compute_blake3(filepath: &str) -> io::Result<String> {
    let mut file: File = File::open(filepath)?;
    let mut hasher: blake3::Hasher = blake3::Hasher::new();

    io::copy(&mut file, &mut hasher)?;
    
    Ok(hasher.finalize().to_hex().to_string())
}

fn build_snapshot<P: AsRef<Path>>(dir: P) -> std::io::Result<Snapshot> {
    let snapshot: Snapshot = WalkDir::new(dir)
        .skip_hidden(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.file_name() != SNAPSHOT_FILE)
        .par_bridge()
        .filter_map(|entry| {
            let path: std::path::PathBuf = entry.path();
            let canonical_path: std::path::PathBuf = fs::canonicalize(&path).ok()?;
            let path_str: String = canonical_path.to_str()?.to_string();

            let metadata: fs::Metadata = entry.metadata().ok()?;
            let size: u64 = metadata.len();
            let modified: std::time::SystemTime = metadata.modified().ok()?;
            
            let hash: Option<String> = compute_blake3(&path_str).ok();

            Some((
                path_str,
                FileMeta { size, modified, hash },
            ))        
        })
        .collect();
    Ok(snapshot)
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
            let total_scanned = new_snap.len();

            let diff: BTreeMap<String, core::Alert> = compare_snapshots(&old_snap, &new_snap);

            if diff.is_empty() {
                println!("[+] Integrity check passed: No unauthorized changes.");
            } else {
                println!("\n[!] ALERTS - TAMPERING DETECTED:");
                for (path, alert) in &diff {
                    println!("  [{:?}] {}", alert, path);
                }
            }

            println!("{} files checked", total_scanned);
            println!("{} alerts", diff.len())
        },
    }
    Ok(())
}
