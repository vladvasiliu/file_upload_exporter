use crate::file_walker::DirWalker;
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::{Registry, Unit};
use std::sync::atomic::AtomicU64;
use std::time::{Instant, UNIX_EPOCH};

pub struct Exporter {
    registry: Registry,
    file_walkers: Vec<DirWalker>,
}

impl Exporter {
    pub fn new(file_walkers: Vec<DirWalker>) -> Self {
        let mut registry = <Registry>::default();

        let collector_status_counter = Family::<StatusLabels, Counter>::default();
        registry.register(
            "collection_status",
            "Whether the file status collection succeeded",
            collector_status_counter.clone(),
        );

        let collector_duration_counter = Family::<DurationLabels, Counter>::default();
        registry.register(
            "collection_duration",
            "How long collection status took",
            collector_duration_counter.clone(),
        );

        Self {
            file_walkers,
            registry,
        }
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn collect(&self) -> Registry {
        let mut registry = <Registry>::default();
        let watcher_upload_time = Family::<ResultLabels, Gauge<u64, AtomicU64>>::default();
        registry.register(
            "watcher_upload_time",
            "Latest file change timestamp",
            watcher_upload_time.clone(),
        );
        let watcher_file_count = Family::<ResultLabels, Gauge<u64, AtomicU64>>::default();
        registry.register(
            "watcher_file_count",
            "Number of files visited by the watcher",
            watcher_file_count.clone(),
        );
        let watcher_success = Family::<ResultLabels, Gauge>::default();
        registry.register(
            "watcher_success",
            "Whether the watcher succeeded",
            watcher_success.clone(),
        );
        let watcher_duration = Family::<ResultLabels, Gauge<u64, AtomicU64>>::default();
        registry.register_with_unit(
            "watcher_duration",
            "How long the watcher took, in seconds",
            Unit::Seconds,
            watcher_duration.clone(),
        );

        for walker in &self.file_walkers {
            let result_labels = &ResultLabels {
                name: walker.name.clone(),
            };

            let walk_start = Instant::now();
            let walk_result = walker.walk();
            let walk_duration = walk_start.elapsed();
            let mut success = 0;

            if let Ok(walk_result) = walk_result {
                success = 1;
                watcher_file_count
                    .get_or_create(result_labels)
                    .set(walk_result.files_visited);
                watcher_upload_time.get_or_create(result_labels).set(
                    walk_result
                        .max_time
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );
            }
            watcher_success.get_or_create(result_labels).set(success);
            watcher_duration
                .get_or_create(result_labels)
                .set(walk_duration.as_secs());
        }

        registry
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct StatusLabels {
    name: String,
    status: StatusValue,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
enum StatusValue {
    SUCCESS,
    FAILURE,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct DurationLabels {
    name: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct ResultLabels {
    name: String,
}
