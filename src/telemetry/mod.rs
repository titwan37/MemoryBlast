//! High-Throughput Telemetry Pipeline & Polars DataFrame Engine
//!
//! Ingests Tier-1 process memory snapshots and Tier-2 Chromium metadata into
//! columnar Polars DataFrames with ephemeral rolling memory buffers and
//! optional Apache Parquet time-series persistence.

use polars::prelude::*;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::analytics::{MemoryAnalyticsEngine, ProcessAnomalyMetrics};
use crate::sampler::SystemSnapshot;

/// Entry in the rolling telemetry buffer
#[derive(Debug, Clone)]
pub struct TelemetryRecord {
    pub timestamp_ms: u64,
    pub total_physical_bytes: u64,
    pub used_physical_bytes: u64,
    pub committed_bytes: u64,
    pub total_page_file_bytes: u64,
    pub paged_pool_bytes: u64,
    pub non_paged_pool_bytes: u64,
    pub processes_count: usize,
}

/// Telemetry pipeline managing streaming ingestion, Polars conversion, and Parquet export
pub struct TelemetryPipeline {
    buffer_capacity: usize,
    records: Vec<TelemetryRecord>,
    analytics_engine: Mutex<MemoryAnalyticsEngine>,
    storage_dir: PathBuf,
}

impl TelemetryPipeline {
    pub fn new(buffer_capacity: usize, storage_dir: impl Into<PathBuf>) -> Self {
        let dir = storage_dir.into();
        let _ = std::fs::create_dir_all(&dir);

        Self {
            buffer_capacity,
            records: Vec::with_capacity(buffer_capacity),
            analytics_engine: Mutex::new(MemoryAnalyticsEngine::new(buffer_capacity)),
            storage_dir: dir,
        }
    }

    /// Ingests a new SystemSnapshot, updates analytics, and appends to the rolling telemetry log
    pub fn ingest(&mut self, snapshot: &SystemSnapshot) -> Vec<ProcessAnomalyMetrics> {
        let record = TelemetryRecord {
            timestamp_ms: snapshot.timestamp_ms,
            total_physical_bytes: snapshot.mem_stats.total_physical_bytes,
            used_physical_bytes: snapshot.mem_stats.used_physical_bytes,
            committed_bytes: snapshot.mem_stats.committed_bytes,
            total_page_file_bytes: snapshot.mem_stats.total_page_file_bytes,
            paged_pool_bytes: snapshot.mem_stats.paged_pool_bytes,
            non_paged_pool_bytes: snapshot.mem_stats.non_paged_pool_bytes,
            processes_count: snapshot.processes.len(),
        };

        if self.records.len() >= self.buffer_capacity {
            self.records.remove(0);
        }
        self.records.push(record);

        // Update online analytics
        let proc_tuples: Vec<(u32, String, u64)> = snapshot
            .processes
            .iter()
            .map(|p| (p.pid, p.name.clone(), p.working_set_bytes))
            .collect();

        if let Ok(mut engine) = self.analytics_engine.lock() {
            engine.update(&proc_tuples, snapshot.mem_stats.committed_bytes)
        } else {
            Vec::new()
        }
    }

    /// Converts the current snapshot into a high-performance Polars DataFrame
    pub fn snapshot_to_dataframe(snapshot: &SystemSnapshot) -> Result<DataFrame, PolarsError> {
        let n = snapshot.processes.len();
        let mut pids = Vec::with_capacity(n);
        let mut names = Vec::with_capacity(n);
        let mut titles = Vec::with_capacity(n);
        let mut ws_bytes = Vec::with_capacity(n);
        let mut priv_bytes = Vec::with_capacity(n);
        let mut cpus = Vec::with_capacity(n);
        let mut is_chroms = Vec::with_capacity(n);
        let mut is_tabs = Vec::with_capacity(n);

        for p in &snapshot.processes {
            pids.push(p.pid);
            names.push(p.name.as_str());
            titles.push(p.display_title.as_str());
            ws_bytes.push(p.working_set_bytes);
            priv_bytes.push(p.private_bytes);
            cpus.push(p.cpu_percent as f64);
            is_chroms.push(p.is_chromium);
            let tab_flag = p.proc_type_label.contains("Tab") || p.display_title != p.name;
            is_tabs.push(tab_flag);
        }

        DataFrame::new(vec![
            Series::new("pid", pids),
            Series::new("name", names),
            Series::new("title", titles),
            Series::new("working_set_bytes", ws_bytes),
            Series::new("private_bytes", priv_bytes),
            Series::new("cpu_percent", cpus),
            Series::new("is_chromium", is_chroms),
            Series::new("is_tab", is_tabs),
        ])
    }

    /// Persists the current process snapshot to an Apache Parquet time-series file
    pub fn export_snapshot_to_parquet(
        &self,
        snapshot: &SystemSnapshot,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut df = Self::snapshot_to_dataframe(snapshot)?;

        let filename = format!("snapshot_{}.parquet", snapshot.timestamp_ms);
        let target_path = self.storage_dir.join(filename);

        let file = File::create(&target_path)?;
        ParquetWriter::new(file)
            .with_compression(ParquetCompression::Snappy)
            .finish(&mut df)?;

        Ok(target_path)
    }

    /// Exports rolling system telemetry records to a Polars DataFrame
    pub fn telemetry_to_dataframe(&self) -> Result<DataFrame, PolarsError> {
        let timestamps: Vec<u64> = self.records.iter().map(|r| r.timestamp_ms).collect();
        let in_use: Vec<u64> = self.records.iter().map(|r| r.used_physical_bytes).collect();
        let commit_total: Vec<u64> = self.records.iter().map(|r| r.committed_bytes).collect();
        let paged: Vec<u64> = self.records.iter().map(|r| r.paged_pool_bytes).collect();
        let non_paged: Vec<u64> = self.records.iter().map(|r| r.non_paged_pool_bytes).collect();

        DataFrame::new(vec![
            Series::new("timestamp_ms", timestamps),
            Series::new("in_use_bytes", in_use),
            Series::new("commit_total_bytes", commit_total),
            Series::new("paged_pool_bytes", paged),
            Series::new("non_paged_pool_bytes", non_paged),
        ])
    }
}
