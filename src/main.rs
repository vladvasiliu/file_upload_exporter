mod exporter;
mod file_walker;

use crate::exporter::Exporter;
use crate::file_walker::DirWalker;
use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::State;
use axum::http::{Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use prometheus_client::encoding::text::{encode, encode_registry};
use serde::Deserialize;
use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
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

    let settings = match Settings::load("config.json5").context("Loading settings") {
        Ok(settings) => settings,
        Err(err) => {
            error!(errror.backtrace = %err.backtrace(), error.message = format!("{:#}", err), error.root_cause = err.root_cause(), "Failed to load settings");
            exit(1);
        }
    };

    let file_status_collector = Arc::new(Exporter::new(settings.file_watchers));

    let router = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(file_status_collector);
    let listener = match tokio::net::TcpListener::bind(("", settings.listen_port)).await {
        Ok(l) => {
            info!("Listening on {}", l.local_addr().unwrap());
            l
        }
        Err(err) => {
            error!(error.message = %err, "Failed to bind listener");
            exit(1)
        }
    };

    match axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    {
        Ok(_) => info!("Shutting down"),
        Err(err) => {
            error!(error.message = %err, "Server failed");
            exit(1)
        }
    }
}

async fn metrics_handler(State(exporter): State<Arc<Exporter>>) -> impl IntoResponse {
    let mut buffer = String::new();

    let local_registry = match task::spawn_blocking(move || exporter.collect()).await {
        Ok(lr) => lr,
        Err(err) => {
            warn!(error.message = %err, "Failed to collect latest update");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(err.to_string()))
                .unwrap();
        }
    };

    match encode_registry(&mut buffer, &local_registry)
        .and_then(|()| encode_registry(&mut buffer, &local_registry))
    {
        Ok(()) => Response::builder()
            .status(StatusCode::OK)
            .header(
                "CONTENT_TYPE",
                "application/openmetrics-text; version=1.0.0; charset=utf-8",
            )
            .body(Body::from(buffer))
            .unwrap(),
        Err(err) => {
            warn!("Failed to encode registry: {}", err);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(err.to_string()))
                .unwrap()
        }
    }
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
