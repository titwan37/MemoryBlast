//! Dynamic Data Science & Anomaly Detection Engine
//!
//! Provides online statistical computation:
//! - Rolling Z-score anomaly detection for process & system memory consumption
//! - Memory leak velocity estimation (dRAM/dt in MB/min) via linear regression over time
//! - Commit charge slope and OOM pressure analysis

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// Classification of process memory behavior based on statistical velocity and Z-score
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Normal,
    ModerateGrowth,
    RapidSpike,
    SustainedLeak,
}

/// Real-time statistical metrics for an individual process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessAnomalyMetrics {
    pub pid: u32,
    pub name: String,
    pub current_bytes: u64,
    pub rolling_mean_bytes: f64,
    pub rolling_std_dev: f64,
    pub z_score: f64,
    pub leak_velocity_mb_min: f64,
    pub severity: AnomalySeverity,
    pub observation_count: usize,
}

/// A historical memory sample for regression
#[derive(Debug, Clone)]
struct SamplePoint {
    time_secs: f64,
    bytes: u64,
}

/// State tracking for a single process
#[derive(Debug)]
struct ProcessHistory {
    name: String,
    samples: VecDeque<SamplePoint>,
    max_samples: usize,
}

impl ProcessHistory {
    fn new(name: String, max_samples: usize) -> Self {
        Self {
            name,
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    fn push(&mut self, time_secs: f64, bytes: u64) {
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(SamplePoint { time_secs, bytes });
    }

    /// Calculates rolling mean and standard deviation
    fn mean_and_std(&self) -> (f64, f64) {
        if self.samples.is_empty() {
            return (0.0, 0.0);
        }
        let n = self.samples.len() as f64;
        let sum: f64 = self.samples.iter().map(|s| s.bytes as f64).sum();
        let mean = sum / n;

        if self.samples.len() < 2 {
            return (mean, 0.0);
        }

        let variance: f64 = self
            .samples
            .iter()
            .map(|s| {
                let diff = s.bytes as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / (n - 1.0);

        (mean, variance.sqrt())
    }

    /// Calculates slope dRAM/dt via linear regression: bytes per second
    fn slope_bytes_per_sec(&self) -> f64 {
        let n = self.samples.len();
        if n < 3 {
            return 0.0;
        }

        let mut sum_t = 0.0;
        let mut sum_b = 0.0;
        let mut sum_tt = 0.0;
        let mut sum_tb = 0.0;

        let t0 = self.samples[0].time_secs;

        for s in &self.samples {
            let t = s.time_secs - t0;
            let b = s.bytes as f64;
            sum_t += t;
            sum_b += b;
            sum_tt += t * t;
            sum_tb += t * b;
        }

        let nf = n as f64;
        let denominator = nf * sum_tt - sum_t * sum_t;
        if denominator.abs() < 1e-9 {
            return 0.0;
        }

        (nf * sum_tb - sum_t * sum_b) / denominator
    }
}

/// Multi-process rolling analytics tracker
pub struct MemoryAnalyticsEngine {
    start_time: Instant,
    process_histories: HashMap<u32, ProcessHistory>,
    commit_history: VecDeque<SamplePoint>,
    window_size: usize,
}

impl MemoryAnalyticsEngine {
    pub fn new(window_size: usize) -> Self {
        Self {
            start_time: Instant::now(),
            process_histories: HashMap::new(),
            commit_history: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    /// Ingests a new snapshot and updates rolling statistics
    pub fn update(
        &mut self,
        current_processes: &[(u32, String, u64)], // (pid, name, bytes)
        total_commit_bytes: u64,
    ) -> Vec<ProcessAnomalyMetrics> {
        let elapsed = self.start_time.elapsed().as_secs_f64();

        // 1. Update commit history
        if self.commit_history.len() >= self.window_size {
            self.commit_history.pop_front();
        }
        self.commit_history.push_back(SamplePoint {
            time_secs: elapsed,
            bytes: total_commit_bytes,
        });

        // 2. Track current active PIDs to clean up terminated ones
        let mut active_pids = std::collections::HashSet::new();
        let mut results = Vec::new();

        for &(pid, ref name, bytes) in current_processes {
            active_pids.insert(pid);
            let history = self
                .process_histories
                .entry(pid)
                .or_insert_with(|| ProcessHistory::new(name.clone(), self.window_size));

            history.push(elapsed, bytes);

            let (mean, std_dev) = history.mean_and_std();
            let z_score = if std_dev > 1024.0 * 1024.0 {
                // Ignore micro-fluctuations under 1MB
                (bytes as f64 - mean) / std_dev
            } else {
                0.0
            };

            let bytes_per_sec = history.slope_bytes_per_sec();
            // Convert to MB / minute
            let mb_per_min = (bytes_per_sec * 60.0) / (1024.0 * 1024.0);

            // Classify severity
            let severity = if mb_per_min > 50.0 || (z_score > 3.0 && mb_per_min > 10.0) {
                AnomalySeverity::RapidSpike
            } else if mb_per_min > 15.0 && history.samples.len() >= 10 {
                AnomalySeverity::SustainedLeak
            } else if mb_per_min > 5.0 {
                AnomalySeverity::ModerateGrowth
            } else {
                AnomalySeverity::Normal
            };

            results.push(ProcessAnomalyMetrics {
                pid,
                name: name.clone(),
                current_bytes: bytes,
                rolling_mean_bytes: mean,
                rolling_std_dev: std_dev,
                z_score,
                leak_velocity_mb_min: mb_per_min,
                severity,
                observation_count: history.samples.len(),
            });
        }

        // Clean up dead processes
        self.process_histories.retain(|pid, _| active_pids.contains(pid));

        results
    }

    /// Computes system-wide commit velocity in MB/minute
    pub fn commit_velocity_mb_min(&self) -> f64 {
        let n = self.commit_history.len();
        if n < 3 {
            return 0.0;
        }

        let mut sum_t = 0.0;
        let mut sum_b = 0.0;
        let mut sum_tt = 0.0;
        let mut sum_tb = 0.0;

        let t0 = self.commit_history[0].time_secs;

        for s in &self.commit_history {
            let t = s.time_secs - t0;
            let b = s.bytes as f64;
            sum_t += t;
            sum_b += b;
            sum_tt += t * t;
            sum_tb += t * b;
        }

        let nf = n as f64;
        let denominator = nf * sum_tt - sum_t * sum_t;
        if denominator.abs() < 1e-9 {
            return 0.0;
        }

        let slope_bytes_per_sec = (nf * sum_tb - sum_t * sum_b) / denominator;
        (slope_bytes_per_sec * 60.0) / (1024.0 * 1024.0)
    }

    /// Estimates seconds until total commit reaches limit, if expanding
    pub fn estimate_seconds_to_limit(&self, current_commit: u64, limit_bytes: u64) -> Option<f64> {
        let slope_bytes_sec = (self.commit_velocity_mb_min() * 1024.0 * 1024.0) / 60.0;
        if slope_bytes_sec <= 0.0 || current_commit >= limit_bytes {
            return None;
        }

        let remaining_bytes = limit_bytes - current_commit;
        Some((remaining_bytes as f64) / slope_bytes_sec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leak_velocity_detection() {
        let mut engine = MemoryAnalyticsEngine::new(30);

        // Simulate a process steadily leaking 10MB every second
        let mut current_bytes = 100 * 1024 * 1024;
        let mut latest_metrics = Vec::new();

        for _i in 0..15 {
            current_bytes += 10 * 1024 * 1024; // +10MB
            latest_metrics = engine.update(
                &[(1234, "leaker.exe".to_string(), current_bytes)],
                current_bytes,
            );
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert_eq!(latest_metrics.len(), 1);
        let m = &latest_metrics[0];
        assert_eq!(m.pid, 1234);
        assert!(m.leak_velocity_mb_min > 0.0);
    }
}
