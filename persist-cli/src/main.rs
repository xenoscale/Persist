/*!
Persist CLI - Command-line interface for the Persist agent snapshot system.

This CLI provides utilities for inspecting, managing, and debugging agent snapshots
stored in various backends (local filesystem, S3).
*/

use clap::{Parser, Subcommand, ValueEnum};
use persist_core::{
    config::{StorageBackend, StorageConfig},
    create_engine_from_config, LocalFileStorage, PersistError, SnapshotMetadata, StorageAdapter,
};
use std::path::PathBuf;
use tabled::{Table, Tabled};
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "persist")]
#[command(about = "CLI for Persist agent snapshot system")]
#[command(version)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Storage backend to use
    #[arg(short, long, global = true, value_enum, default_value = "disk")]
    storage: StorageType,

    /// Storage path (directory for disk, bucket for S3)
    #[arg(short, long, global = true)]
    path: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Clone, Debug)]
enum StorageType {
    Disk,
    S3,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available snapshots
    List {
        /// Show additional details
        #[arg(short, long)]
        detailed: bool,
    },
    /// Show details of a specific snapshot
    Show {
        /// Snapshot identifier (path or key)
        snapshot_id: String,
    },
    /// Verify integrity of a snapshot
    Verify {
        /// Snapshot identifier (path or key)
        snapshot_id: String,
    },
    /// Delete a snapshot
    Delete {
        /// Snapshot identifier (path or key)
        snapshot_id: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Tabled)]
struct SnapshotInfo {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Agent ID")]
    agent_id: String,
    #[tabled(rename = "Session ID")]
    session_id: String,
    #[tabled(rename = "Index")]
    index: u64,
    #[tabled(rename = "Created")]
    timestamp: String,
    #[tabled(rename = "Size")]
    size: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose);

    // Create storage config
    let storage_config = create_storage_config(&cli)?;

    // Execute command
    match cli.command {
        Commands::List { detailed } => list_snapshots(&storage_config, detailed).await?,
        Commands::Show { snapshot_id } => show_snapshot(&storage_config, &snapshot_id).await?,
        Commands::Verify { snapshot_id } => verify_snapshot(&storage_config, &snapshot_id).await?,
        Commands::Delete { snapshot_id, force } => {
            delete_snapshot(&storage_config, &snapshot_id, force).await?
        }
    }

    Ok(())
}

fn init_logging(verbose: bool) {
    let filter = if verbose {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"))
    } else {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

fn create_storage_config(cli: &Cli) -> Result<StorageConfig, anyhow::Error> {
    let backend = match cli.storage {
        StorageType::Disk => StorageBackend::Local,
        StorageType::S3 => StorageBackend::S3,
    };

    let path = cli.path.clone().unwrap_or_else(|| match backend {
        StorageBackend::Local => "./snapshots".to_string(),
        StorageBackend::S3 => std::env::var("AWS_S3_BUCKET").unwrap_or_else(|_| {
            eprintln!("Error: AWS_S3_BUCKET environment variable is required for S3 storage");
            std::process::exit(1);
        }),
    });

    match backend {
        StorageBackend::Local => {
            let mut config = StorageConfig::default_local();
            config.local_base_path = Some(std::path::PathBuf::from(path));
            Ok(config)
        }
        StorageBackend::S3 => Ok(StorageConfig::s3_with_bucket(path)),
    }
}

async fn list_snapshots(
    storage_config: &StorageConfig,
    detailed: bool,
) -> Result<(), anyhow::Error> {
    info!("Listing snapshots from {:?}", storage_config);

    match storage_config.backend {
        StorageBackend::Local => {
            let path = storage_config
                .local_base_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "./snapshots".to_string());
            list_local_snapshots(&path, detailed).await
        }
        StorageBackend::S3 => {
            warn!("S3 snapshot listing not yet implemented");
            Ok(())
        }
    }
}

async fn list_local_snapshots(path: &str, _detailed: bool) -> Result<(), anyhow::Error> {
    let path = PathBuf::from(path);
    if !path.exists() {
        println!("No snapshots directory found at: {}", path.display());
        return Ok(());
    }

    let mut snapshots = Vec::new();
    let storage = LocalFileStorage::new();

    // Read directory contents
    let entries = std::fs::read_dir(&path)?;
    for entry in entries {
        let entry = entry?;
        let file_path = entry.path();

        if file_path.is_file() {
            let path_str = file_path.to_string_lossy();

            // Try to load and parse metadata
            match load_snapshot_metadata(&storage, &path_str) {
                Ok(metadata) => {
                    let size = match std::fs::metadata(&file_path) {
                        Ok(meta) => format_size(meta.len()),
                        Err(_) => "Unknown".to_string(),
                    };

                    snapshots.push(SnapshotInfo {
                        id: file_path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                        agent_id: metadata.agent_id.clone(),
                        session_id: metadata.session_id.clone(),
                        index: metadata.snapshot_index,
                        timestamp: format_timestamp(metadata.timestamp.timestamp()),
                        size,
                    });
                }
                Err(e) => {
                    warn!("Failed to load metadata for {}: {}", path_str, e);
                }
            }
        }
    }

    if snapshots.is_empty() {
        println!("No snapshots found");
    } else {
        snapshots.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        let table = Table::new(snapshots);
        println!("{table}");
    }

    Ok(())
}

async fn show_snapshot(
    storage_config: &StorageConfig,
    snapshot_id: &str,
) -> Result<(), anyhow::Error> {
    info!("Showing snapshot: {}", snapshot_id);

    let engine = create_engine_from_config(storage_config.clone())?;

    match engine.load_snapshot(snapshot_id) {
        Ok((metadata, _data)) => {
            println!("Snapshot Details:");
            println!("  ID: {snapshot_id}");
            println!("  Agent ID: {}", metadata.agent_id);
            println!("  Session ID: {}", metadata.session_id);
            println!("  Index: {}", metadata.snapshot_index);
            println!(
                "  Created: {}",
                format_timestamp(metadata.timestamp.timestamp())
            );
            println!("  Format Version: {}", metadata.format_version);
            println!("  Content Hash: {}", metadata.content_hash);

            if let Some(description) = &metadata.description {
                println!("  Description: {description}");
            }
        }
        Err(e) => {
            error!("Failed to load snapshot: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

async fn verify_snapshot(
    storage_config: &StorageConfig,
    snapshot_id: &str,
) -> Result<(), anyhow::Error> {
    info!("Verifying snapshot: {}", snapshot_id);

    let engine = create_engine_from_config(storage_config.clone())?;

    match engine.load_snapshot(snapshot_id) {
        Ok((_metadata, _data)) => {
            println!("✓ Snapshot is valid and integrity check passed");
        }
        Err(PersistError::IntegrityCheckFailed { expected, actual }) => {
            error!("✗ Integrity check failed:");
            error!("  Expected hash: {}", expected);
            error!("  Actual hash: {}", actual);
            return Err(anyhow::anyhow!("Integrity check failed"));
        }
        Err(e) => {
            error!("✗ Failed to verify snapshot: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

async fn delete_snapshot(
    storage_config: &StorageConfig,
    snapshot_id: &str,
    force: bool,
) -> Result<(), anyhow::Error> {
    if !force {
        print!("Are you sure you want to delete snapshot '{snapshot_id}'? (y/N): ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().to_lowercase().starts_with('y') {
            println!("Deletion cancelled");
            return Ok(());
        }
    }

    let _engine = create_engine_from_config(storage_config.clone())?;

    // Get storage adapter to delete
    match storage_config.backend {
        StorageBackend::Local => {
            let storage = LocalFileStorage::new();
            storage.delete(snapshot_id)?;
            println!("✓ Snapshot deleted successfully");
        }
        StorageBackend::S3 => {
            #[cfg(feature = "s3")]
            {
                use persist_core::S3StorageAdapter;
                let bucket = storage_config
                    .s3_bucket
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("S3 bucket not configured"))?;
                let storage = S3StorageAdapter::new(bucket.to_string())?;
                storage.delete(snapshot_id)?;
                println!("✓ Snapshot deleted successfully");
            }
            #[cfg(not(feature = "s3"))]
            {
                return Err(anyhow::anyhow!("S3 support not enabled"));
            }
        }
    }

    Ok(())
}

fn load_snapshot_metadata(
    storage: &impl StorageAdapter,
    path: &str,
) -> Result<SnapshotMetadata, PersistError> {
    let data = storage.load(path)?;

    // Try to decompress and parse
    use persist_core::compression::{CompressionAdapter, GzipCompressor};
    let compressor = GzipCompressor::new();
    let decompressed = compressor.decompress(&data)?;

    // Parse JSON
    let json: serde_json::Value = serde_json::from_slice(&decompressed)?;
    let metadata = json["metadata"].clone();
    let metadata: SnapshotMetadata = serde_json::from_value(metadata)?;

    Ok(metadata)
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

fn format_timestamp(timestamp: i64) -> String {
    use chrono::{Local, TimeZone};

    match Local.timestamp_opt(timestamp, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => timestamp.to_string(),
    }
}
