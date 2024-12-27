mod file_walker;

use crate::file_walker::{DirWalker, SearchPath};
use std::path::Path;
use std::process::exit;
use std::time::SystemTime;
use serde::Deserialize;
use anyhow::{Context, Result};
use tokio::task;
use tracing::{error, info, instrument, warn};
use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        // .json()
        .finish()
        .with(ErrorLayer::default())
        .init();
    
    let mut settings = match Settings::load("config.example.json5").context("Loading settings") {
        Ok(settings) => settings,
        Err(err) => {
            error!(errror.backtrace = %err.backtrace(), error.message = format!("{:#}", err), error.root_cause = err.root_cause(), "Failed to load settings");
            exit(1);
        }
    };
    
    let search_path = settings.file_watchers.pop().unwrap();
    
    let join = task::spawn_blocking(|| {
        let walker = DirWalker::new(search_path);
    
        match walker.walk() {
            Err(e) => warn!("{}", e),
            Ok(walk_result) => {
                let max_time = walk_result
                    .max_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                info!(
                    max_time = max_time,
                    files_visited = walk_result.files_visited,
                    "Done"
                );
            }
        }
    });

    println!("{:?}", join.await);
}

#[derive(Debug,Deserialize)]
struct Settings {
    listen_port: u16,
    file_watchers: Vec<SearchPath>
}

impl Settings {
    #[instrument]
    pub fn load(file_path: &str) -> Result<Self> {
        let config = config::Config::builder().add_source(config::File::with_name(file_path)).build().context("Reading settings file")?;
        let settings = config.try_deserialize().context("Deserializing settings file")?;
        
        Ok(settings)
    }
}
