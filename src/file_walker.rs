use anyhow::{anyhow, Context, Result};
use regex::bytes::Regex;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::cmp::max;
use std::collections::HashMap;
use std::fs;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::warn;

#[derive(Debug, Deserialize)]
pub struct DirWalker {
    pub name: String,
    path: PathBuf,
    recursive: bool,
    #[serde(deserialize_with = "deserialize_regex")]
    file_regex: Regex,
    labels: HashMap<String, String>,
}

impl DirWalker {
    pub fn walk(&self) -> Result<WalkResult> {
        Self::walk_dir(&self.path, file_callback, self.recursive, &self.file_regex)
    }

    /// Walk a dir and return the time of latest modification
    fn walk_dir(
        dir: &Path,
        file_callback: fn(&DirEntry) -> Result<SystemTime>,
        recursive: bool,
        regex: &Regex,
    ) -> Result<WalkResult> {
        let mut max_time = SystemTime::UNIX_EPOCH;
        let mut files_visited = 0_u64;

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
                            if let Ok(wr) = Self::walk_dir(&path, file_callback, true, regex) {
                                max_time = max(max_time, wr.max_time);
                                files_visited += wr.files_visited;
                            }
                        } else if path.is_file()
                            && regex.is_match(path.as_os_str().as_encoded_bytes())
                        {
                            match file_callback(&entry) {
                                Ok(st) => {
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

pub fn file_callback(d: &DirEntry) -> Result<SystemTime> {
    let metadata = d.metadata().context("failed to read metadata")?;
    metadata.modified().context("failed to read modified time")
}

fn deserialize_regex<'de, D>(deserializer: D) -> std::result::Result<Regex, D::Error>
where
    D: Deserializer<'de>,
{
    let pattern = String::deserialize(deserializer)?;
    let regex = Regex::new(&pattern)
        .map_err(|e| D::Error::custom(format!("invalid file pattern: {}", e)))?;
    Ok(regex)
}

pub struct WalkResult {
    pub(crate) max_time: SystemTime,
    pub(crate) files_visited: u64,
}
