use std::{
    env,
    path::{Component, Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result};
use futures::StreamExt;
use iroh_base::SecretKey;
use iroh_blobs::{
    api::{Store, TempTag},
    format::collection::Collection,
    provider::Event,
    HashAndFormat,
};
use rand::{thread_rng, Rng};
use tokio::sync::mpsc;
use tracing::info;
use walkdir::WalkDir;

/// Gets a secret key from the IROH_SECRET environment variable or generates a new random one.
/// If the environment variable is set, it must be a valid string representation of a secret key.
pub fn get_or_generate_secret_key() -> Result<SecretKey> {
    if let Ok(secret) = env::var("IROH_SECRET") {
        // Parse the secret key from string
        SecretKey::from_str(&secret).context("Invalid secret key format")
    } else {
        // Generate a new random key
        let secret_key = SecretKey::generate(&mut thread_rng());
        println!("Generated new secret key: {}", secret_key);
        println!("To reuse this key, set the IROH_SECRET environment variable to this value");
        Ok(secret_key)
    }
}

/// Create a unique directory for sending files.
pub fn create_send_dir() -> Result<PathBuf> {
    let suffix = rand::thread_rng().gen::<[u8; 16]>();
    let cwd = std::env::current_dir()?;
    let blobs_data_dir = cwd.join(format!(".{}-send-{}", crate_name(), hex::encode(suffix)));
    Ok(blobs_data_dir)
}

pub fn create_recv_dir(content: HashAndFormat) -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let blobs_data_dir = cwd.join(format!(".{}-recv-{}", crate_name(), content));
    Ok(blobs_data_dir)
}

/// Import from a file or directory into the database.
///
/// The returned tag always refers to a collection. If the input is a file, this
/// is a collection with a single blob, named like the file.
///
/// If the input is a directory, the collection contains all the files in the
/// directory.
pub async fn import(path: PathBuf, db: &Store) -> Result<TempTag> {
    let parallelism = num_cpus::get();
    let path = path.canonicalize()?;
    anyhow::ensure!(path.exists(), "path {} does not exist", path.display());
    let root = path.parent().context("context get parent")?;
    // walkdir also works for files, so we don't need to special case them
    let files = WalkDir::new(path.clone()).into_iter();
    // flatten the directory structure into a list of (name, path) pairs.
    // ignore symlinks.
    let data_sources: Vec<(String, PathBuf)> = files
        .map(|entry| {
            let entry = entry?;
            if !entry.file_type().is_file() {
                // Skip symlinks. Directories are handled by WalkDir.
                return Ok(None);
            }
            let path = entry.into_path();
            let relative = path.strip_prefix(root)?;
            let name = canonicalized_path_to_string(relative, true)?;
            anyhow::Ok(Some((name, path)))
        })
        .filter_map(Result::transpose)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut names_and_tags = futures::stream::iter(data_sources)
        .map(|(name, path)| {
            let db = db.clone();
            println!("adding {name}");
            async move { Ok((name, db.add_path(path).await?)) }
        })
        .buffer_unordered(parallelism)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    names_and_tags.sort_by(|(a, _), (b, _)| a.cmp(b));
    // collect the (name, hash) tuples into a collection
    // we must also keep the tags around so the data does not get gced.
    let (collection, tags) = names_and_tags
        .into_iter()
        .map(|(name, tag)| ((name, *tag.hash()), tag))
        .unzip::<_, _, Collection, Vec<_>>();
    let temp_tag = collection.store(db).await?;
    // now that the collection is stored, we can drop the tags
    // data is protected by the collection
    drop(tags);
    Ok(temp_tag)
}

pub async fn export(db: &Store, collection: Collection) -> Result<()> {
    let root = std::env::current_dir()?;
    for (name, hash) in collection.iter() {
        let target = get_export_path(&root, name)?;
        if target.exists() {
            eprintln!(
                "target {} already exists. Export stopped.",
                target.display()
            );
            eprintln!("You can remove the file or directory and try again. The download will not be repeated.");
            anyhow::bail!("target {} already exists", target.display());
        }
        db.export(*hash, target).await?;
    }
    Ok(())
}

/// This function converts an already canonicalized path to a string.
///
/// If `must_be_relative` is true, the function will fail if any component of the path is
/// `Component::RootDir`
///
/// This function will also fail if the path is non canonical, i.e. contains
/// `..` or `.`, or if the path components contain any windows or unix path
/// separators.
pub fn canonicalized_path_to_string(
    path: impl AsRef<Path>,
    must_be_relative: bool,
) -> anyhow::Result<String> {
    let mut path_str = String::new();
    let parts = path
        .as_ref()
        .components()
        .filter_map(|c| match c {
            Component::Normal(x) => {
                let c = match x.to_str() {
                    Some(c) => c,
                    None => return Some(Err(anyhow::anyhow!("invalid character in path"))),
                };

                if !c.contains('/') && !c.contains('\\') {
                    Some(Ok(c))
                } else {
                    Some(Err(anyhow::anyhow!("invalid path component {:?}", c)))
                }
            }
            Component::RootDir => {
                if must_be_relative {
                    Some(Err(anyhow::anyhow!("invalid path component {:?}", c)))
                } else {
                    path_str.push('/');
                    None
                }
            }
            _ => Some(Err(anyhow::anyhow!("invalid path component {:?}", c))),
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let parts = parts.join("/");
    path_str.push_str(&parts);
    Ok(path_str)
}

fn get_export_path(root: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let parts = name.split('/');
    let mut path = root.to_path_buf();
    for part in parts {
        validate_path_component(part)?;
        path.push(part);
    }
    Ok(path)
}

fn validate_path_component(component: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        !component.contains('/'),
        "path components must not contain the only correct path separator, /"
    );
    Ok(())
}

pub fn crate_name() -> &'static str {
    env!("CARGO_CRATE_NAME")
}

pub fn dump_provider_events() -> (
    tokio::task::JoinHandle<()>,
    mpsc::Sender<iroh_blobs::provider::Event>,
) {
    let (tx, mut rx) = mpsc::channel(100);
    let dump_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                Event::ClientConnected {
                    node_id,
                    connection_id,
                    permitted,
                } => {
                    permitted.send(true).await.ok();
                }
                Event::GetRequestReceived {
                    connection_id,
                    request_id,
                    hash,
                    ranges,
                } => {
                    println!(
                        "Get request received: {connection_id} {request_id} {hash} {ranges:?}"
                    );
                }
                Event::TransferCompleted {
                    connection_id,
                    request_id,
                    stats,
                } => {
                    println!("Transfer completed: {connection_id} {request_id} {stats:?}");
                }
                Event::TransferAborted {
                    connection_id,
                    request_id,
                    stats,
                } => {
                    println!("Transfer aborted: {connection_id} {request_id} {stats:?}");
                }
                Event::TransferProgress {
                    connection_id,
                    request_id,
                    index,
                    end_offset,
                } => {
                    info!("Transfer progress: {connection_id} {request_id} {index} {end_offset}");
                }
                _ => {
                    info!("Received event: {:?}", event);
                }
            }
        }
    });
    (dump_task, tx)
}
