mod exporter;
mod file_walker;

use crate::exporter::FileStatusCollector;
use crate::file_walker::DirWalker;
use anyhow::{Context, Result};
use prometheus_client::encoding::text::encode_registry;
use serde::Deserialize;
use std::process::exit;
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

    let mut settings = match Settings::load("config.json5").context("Loading settings") {
        Ok(settings) => settings,
        Err(err) => {
            error!(errror.backtrace = %err.backtrace(), error.message = format!("{:#}", err), error.root_cause = err.root_cause(), "Failed to load settings");
            exit(1);
        }
    };

    let file_status_collector = FileStatusCollector {
        file_walkers: settings.file_watchers,
    };

    let registry = file_status_collector.collect();
    let mut output = String::new();
    encode_registry(&mut output, &registry);
    println!("{}", &output);

    // let join = task::spawn_blocking(|| {
    //     let walker = DirWalker::new(search_path);
    //
    //     match walker.walk() {
    //         Err(e) => warn!("{}", e),
    //         Ok(walk_result) => {
    //             let max_time = walk_result
    //                 .max_time
    //                 .duration_since(SystemTime::UNIX_EPOCH)
    //                 .unwrap()
    //                 .as_secs();
    //             info!(
    //                 max_time = max_time,
    //                 files_visited = walk_result.files_visited,
    //                 "Done"
    //             );
    //         }
    //     }
    // });

    // println!("{:?}", join.await);
}

#[derive(Debug, Deserialize)]
struct Settings {
    listen_port: u16,
    file_watchers: Vec<DirWalker>,
}

impl Settings {
    #[instrument]
    pub fn load(file_path: &str) -> Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name(file_path))
            .build()
            .context("Reading settings file")?;
        let settings = config
            .try_deserialize()
            .context("Deserializing settings file")?;

        Ok(settings)
    }
}
