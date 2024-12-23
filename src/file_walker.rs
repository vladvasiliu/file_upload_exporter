use anyhow::{anyhow, Context, Result};
use std::cmp::max;
use std::collections::HashMap;
use std::fs;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::warn;

pub struct DirWalker {
    search_path: SearchPath,
}

impl DirWalker {
    pub fn new(search_path: SearchPath) -> Self {
        Self { search_path }
    }

    pub fn walk(&self) -> Result<WalkResult> {
        Self::walk_dir(
            &self.search_path.path,
            file_callback,
            self.search_path.recursive,
        )
    }

    /// Walk a dir and return the time of latest modification
    fn walk_dir(
        dir: &Path,
        file_callback: fn(&DirEntry) -> Result<SystemTime>,
        recursive: bool,
    ) -> Result<WalkResult> {
        let mut max_time = SystemTime::UNIX_EPOCH;
        let mut files_visited = 0_u128;

        if dir.is_dir() {
            match fs::read_dir(dir) {
                Ok(entries) => {
                    for entry in entries {
                        let entry = match entry {
                            Ok(entry) => entry,
                            Err(err) => {
                                warn!("Got io error: {}", err);
                                continue;
                            }
                        };
                        let path = entry.path();
                        if path.is_dir() && recursive {
                            if let Ok(wr) = Self::walk_dir(&path, file_callback, true) {
                                max_time = max(max_time, wr.max_time);
                                files_visited += wr.files_visited;
                            }
                        } else if path.is_file() {
                            match file_callback(&entry) {
                                Ok(st) => {
                                    // println!("{}: {:?}", path.display(), st)
                                    max_time = max(max_time, st);
                                }
                                Err(err) => warn!("{}", err),
                            }
                            files_visited += 1;
                        }
                    }
                }
                Err(err) => {
                    warn!("Failed to read dir {}: {}", dir.display(), err);
                    return Err(err.into());
                }
            }
        } else {
            warn!(
                path = dir.to_string_lossy().as_ref(),
                "Path is not a directory or doesn't exist!"
            );
            return Err(anyhow!("Path is not a directory or doesn't exist!"));
        };

        Ok(WalkResult {
            max_time,
            files_visited,
        })
    }
}

pub fn file_callback(d: &DirEntry) -> anyhow::Result<SystemTime> {
    let metadata = d.metadata().context("Failed to read metadata")?;
    metadata.modified().context("Failed to read modified time")
}

/// A path to monitor, with a name and a file regex to match
pub struct SearchPath {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) recursive: bool,
    pub(crate) pattern: String,
    pub(crate) labels: HashMap<String, String>,
}

pub struct WalkResult {
    pub(crate) max_time: SystemTime,
    pub(crate) files_visited: u128,
}
