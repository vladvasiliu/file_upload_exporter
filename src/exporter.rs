use crate::file_walker::DirWalker;
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;
use std::sync::atomic::AtomicU64;
use std::time::UNIX_EPOCH;

pub struct Exporter {
    registry: Registry,
    file_walkers: Vec<DirWalker>,
    collector_status_counter: Family<StatusLabels, Counter>,
    collector_duration_counter: Family<DurationLabels, Counter>,
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
            collector_duration_counter,
            collector_status_counter,
        }
    }
}

pub struct FileStatusCollector {
    pub file_walkers: Vec<DirWalker>,
    // collector_result_value: Family<ResultLabels, Gauge>,
}

impl FileStatusCollector {
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

        for walker in &self.file_walkers {
            let result_labels = &ResultLabels {
                name: walker.name.clone(),
            };

            if let Ok(walk_result) = walker.walk() {
                watcher_success.get_or_create(result_labels).set(1);
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
            } else {
                watcher_success.get_or_create(result_labels).set(0);
            }
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
