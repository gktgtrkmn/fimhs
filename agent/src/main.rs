use clap::{Parser, Subcommand};
use core::{FileMeta, Snapshot, compare_snapshots};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::path::Path;

use jwalk::WalkDir;
use rayon::prelude::*;
use tracing::{Level, debug, error, info, info_span, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

const SNAPSHOT_FILE: &str = ".fim_snapshot.json";

#[derive(Parser, Debug)]
#[command(author, version, about = "File Integrity Monitor", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Init { dir: String },
    Check { dir: String },
}

fn init_logging() {
    let sub = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .compact()
        .with_thread_ids(true)
        .finish();

    tracing::subscriber::set_global_default(sub).expect("Failed to initialzie logging")
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

            let _span = info_span!("process_file", path = ?path).entered();
            let canonical_path: std::path::PathBuf = match fs::canonicalize(&path) {
                Ok(p) => p,
                Err(e) => {
                    debug!(error = ?e, "Failed to canonicalize path");
                    return None;
                }
            };
            let path_str: String = canonical_path.to_str()?.to_string();

            let metadata: fs::Metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    debug!(error = ?e, "Failed to get metadata");
                    return None;
                }
            };
            let size: u64 = metadata.len();

            let modified: std::time::SystemTime = match metadata.modified() {
                Ok(m) => m,
                Err(e) => {
                    debug!(error = ?e, "Failed to fet modified time");
                    return None;
                }
            };

            debug!("Computing hash...");
            let hash: Option<String> = match compute_blake3(&path_str) {
                Ok(h) => Some(h),
                Err(e) => {
                    warn!(error = ?e, "Failed to compute hash, skipping hash field");
                    None
                }
            };

            Some((
                path_str,
                FileMeta {
                    size,
                    modified,
                    hash,
                },
            ))
        })
        .collect();

    Ok(snapshot)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let cli: Cli = Cli::parse();

    match &cli.command {
        Command::Init { dir } => {
            info!(target_dir = dir, "Building baseline snapshot...");

            let snap: BTreeMap<String, FileMeta> = match build_snapshot(dir) {
                Ok(s) => s,
                Err(e) => {
                    error!(error = ?e, "Snapshot generation failed");
                    return Err(e.into());
                }
            };

            let json: String = serde_json::to_string_pretty(&snap)?;

            if let Err(e) = fs::write(SNAPSHOT_FILE, json) {
                error!(error = ?e, file = SNAPSHOT_FILE, "Failed to write snapshot to disk");
                return Err(e.into());
            };

            info!(
                file = SNAPSHOT_FILE,
                scanned = snap.len(),
                "Snapshot successfully saved"
            );
        }
        Command::Check { dir } => {
            if !Path::new(SNAPSHOT_FILE).exists() {
                error!("No snapshot found. Run 'init' first");
                return Err("No snapshot found".into());
            }

            info!(target_dir = dir, "Scanning current directory...");

            let data: String = fs::read_to_string(SNAPSHOT_FILE)?;
            let old_snap: Snapshot = serde_json::from_str(&data)?;

            let new_snap: BTreeMap<String, FileMeta> = build_snapshot(dir)?;
            let total_scanned = new_snap.len();

            let diff: BTreeMap<String, core::Alert> = compare_snapshots(&old_snap, &new_snap);

            if diff.is_empty() {
                info!(
                    scanned = total_scanned,
                    "Integrity check passes: No unauthorized changes."
                );
            } else {
                error!(
                    scanned = total_scanned,
                    alerts = diff.len(),
                    "TAMPERING DETECTED!"
                );
                for (path, alert) in &diff {
                    warn!(alert_type = ?alert, file = path, "Unauthorized change")
                }
            }
        }
    }
    Ok(())
}
