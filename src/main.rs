mod file_walker;

use crate::file_walker::{DirWalker, SearchPath};
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use tokio::task;
use tracing::{info, warn};
use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        // .json()
        .finish()
        .with(ErrorLayer::default())
        .init();

    let join = task::spawn_blocking(|| {
        let path = Path::new(r"/some/path");

        let search_path = SearchPath {
            recursive: true,
            path: path.into(),
            labels: HashMap::new(),
            pattern: String::new(),
            name: "zorglub".to_string(),
        };

        let walker = DirWalker::new(search_path);

        info!(path = path.to_string_lossy().as_ref(), "Starting");

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
