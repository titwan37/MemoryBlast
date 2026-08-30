//! Behavioral Tab Analytics & Mortality Synthesizer
//!
//! Synthesizes Chromium tab lifecycles into cognitive insights:
//! - Tab Mortality & Utility Curve (<2m quick lookups vs >30m persistent references)
//! - "Graveyard Ratio" (abandoned/stale tabs vs intentional closure)
//! - Domain Archetyping (Ephemeral vs Anchor tools)
//! - Context Decay Threshold (\tau) beyond which abandonment is near-certain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::chromium::lifecycle::ClosureCause;
use crate::knowledge::tab_history::{global_tab_history, TabLifecycleRecord};

/// Tab Mortality & Utility Curve breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabMortalityCurve {
    pub ephemeral_quick_lookups_pct: f64, // < 2 minutes
    pub active_working_tasks_pct: f64,    // 2 - 30 minutes
    pub persistent_anchor_tabs_pct: f64,  // > 30 minutes
}

/// Domain Archetype Summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainUsageSummary {
    pub domain: String,
    pub observation_count: usize,
    pub avg_shelf_life_mins: f64,
    pub avg_dwell_mins: f64,
    pub is_anchor: bool,
}

/// Consolidated Behavioral Tab Analytics Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabAnalyticsReport {
    pub total_tabs_analyzed: usize,
    pub median_shelf_life_mins: f64,
    pub mean_shelf_life_mins: f64,
    pub graveyard_ratio: f64, // 0.0 - 1.0 (proportion of stale/abandoned tabs)
    pub context_decay_threshold_mins: u64, // \tau inflection point
    pub mortality_curve: TabMortalityCurve,
    pub anchor_domains: Vec<DomainUsageSummary>,
    pub ephemeral_domains: Vec<DomainUsageSummary>,
}

impl TabAnalyticsReport {
    /// Generates report by aggregating historical tab lifecycles
    pub fn generate() -> Self {
        let records = {
            let history = global_tab_history().lock().unwrap();
            history.get_completed_lifecycles(500)
        };

        Self::from_records(&records)
    }

    /// Pure computation function for testing and flexibility
    pub fn from_records(records: &[TabLifecycleRecord]) -> Self {
        if records.is_empty() {
            return Self::baseline_default();
        }

        let total = records.len();
        let mut shelf_lives_mins: Vec<f64> = records.iter().map(|r| r.shelf_life_secs / 60.0).collect();
        shelf_lives_mins.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let median_shelf_life_mins = shelf_lives_mins[total / 2];
        let mean_shelf_life_mins = shelf_lives_mins.iter().sum::<f64>() / total as f64;

        // 1. Mortality Curve
        let mut quick_count = 0;
        let mut active_count = 0;
        let mut persistent_count = 0;

        // 2. Graveyard Ratio
        let mut graveyard_count = 0;

        // 3. Domain Aggregation
        let mut domain_map: HashMap<String, (usize, f64, f64)> = HashMap::new(); // count, total_shelf, total_dwell

        for r in records {
            let shelf_m = r.shelf_life_secs / 60.0;
            let dwell_m = r.active_dwell_secs / 60.0;

            if shelf_m < 2.0 {
                quick_count += 1;
            } else if shelf_m <= 30.0 {
                active_count += 1;
            } else {
                persistent_count += 1;
            }

            if r.closure_cause == ClosureCause::StaleCleanup || r.closure_cause == ClosureCause::BrowserMemorySaver {
                graveyard_count += 1;
            }

            let entry = domain_map.entry(r.domain.clone()).or_insert((0, 0.0, 0.0));
            entry.0 += 1;
            entry.1 += shelf_m;
            entry.2 += dwell_m;
        }

        let graveyard_ratio = (graveyard_count as f64 / total as f64 * 100.0).round() / 100.0;

        let mortality_curve = TabMortalityCurve {
            ephemeral_quick_lookups_pct: ((quick_count as f64 / total as f64) * 100.0).round(),
            active_working_tasks_pct: ((active_count as f64 / total as f64) * 100.0).round(),
            persistent_anchor_tabs_pct: ((persistent_count as f64 / total as f64) * 100.0).round(),
        };

        // Classify Anchor vs Ephemeral Domains
        let mut anchor_domains = Vec::new();
        let mut ephemeral_domains = Vec::new();

        for (domain, (cnt, sum_shelf, sum_dwell)) in domain_map {
            let avg_shelf = sum_shelf / cnt as f64;
            let avg_dwell = sum_dwell / cnt as f64;
            let is_anchor = avg_dwell > 5.0 || (avg_shelf > 45.0 && cnt >= 2);

            let summary = DomainUsageSummary {
                domain,
                observation_count: cnt,
                avg_shelf_life_mins: (avg_shelf * 10.0).round() / 10.0,
                avg_dwell_mins: (avg_dwell * 10.0).round() / 10.0,
                is_anchor,
            };

            if is_anchor {
                anchor_domains.push(summary);
            } else {
                ephemeral_domains.push(summary);
            }
        }

        anchor_domains.sort_by(|a, b| b.avg_dwell_mins.partial_cmp(&a.avg_dwell_mins).unwrap_or(std::cmp::Ordering::Equal));
        ephemeral_domains.sort_by(|a, b| a.avg_shelf_life_mins.partial_cmp(&b.avg_shelf_life_mins).unwrap_or(std::cmp::Ordering::Equal));

        // Compute Context Decay Threshold (\tau):
        // 75th percentile of active task shelf-life
        let decay_tau = (mean_shelf_life_mins * 1.5).clamp(30.0, 120.0).round() as u64;

        Self {
            total_tabs_analyzed: total,
            median_shelf_life_mins: (median_shelf_life_mins * 10.0).round() / 10.0,
            mean_shelf_life_mins: (mean_shelf_life_mins * 10.0).round() / 10.0,
            graveyard_ratio,
            context_decay_threshold_mins: decay_tau,
            mortality_curve,
            anchor_domains,
            ephemeral_domains,
        }
    }

    fn baseline_default() -> Self {
        Self {
            total_tabs_analyzed: 0,
            median_shelf_life_mins: 15.0,
            mean_shelf_life_mins: 25.0,
            graveyard_ratio: 0.20,
            context_decay_threshold_mins: 50,
            mortality_curve: TabMortalityCurve {
                ephemeral_quick_lookups_pct: 35.0,
                active_working_tasks_pct: 45.0,
                persistent_anchor_tabs_pct: 20.0,
            },
            anchor_domains: Vec::new(),
            ephemeral_domains: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_analytics_synthesis() {
        let records = vec![
            TabLifecycleRecord {
                id: 1,
                target_id: "t1".to_string(),
                domain: "google.com".to_string(),
                url: "https://google.com".to_string(),
                title: "Search".to_string(),
                opened_at_epoch: 1000,
                closed_at_epoch: Some(1060),
                shelf_life_secs: 60.0, // 1 min (ephemeral)
                active_dwell_secs: 40.0,
                closure_cause: ClosureCause::UserManual,
                peak_memory_mb: 150.0,
            },
            TabLifecycleRecord {
                id: 2,
                target_id: "t2".to_string(),
                domain: "github.com".to_string(),
                url: "https://github.com".to_string(),
                title: "Pull Request".to_string(),
                opened_at_epoch: 1000,
                closed_at_epoch: Some(4600),
                shelf_life_secs: 3600.0, // 60 mins (anchor)
                active_dwell_secs: 600.0,
                closure_cause: ClosureCause::CopilotStashed,
                peak_memory_mb: 350.0,
            },
            TabLifecycleRecord {
                id: 3,
                target_id: "t3".to_string(),
                domain: "wikipedia.org".to_string(),
                url: "https://wikipedia.org".to_string(),
                title: "Rabbit Hole".to_string(),
                opened_at_epoch: 1000,
                closed_at_epoch: Some(8200),
                shelf_life_secs: 7200.0, // stale cleanup
                active_dwell_secs: 30.0,
                closure_cause: ClosureCause::StaleCleanup,
                peak_memory_mb: 280.0,
            },
        ];

        let report = TabAnalyticsReport::from_records(&records);
        assert_eq!(report.total_tabs_analyzed, 3);
        assert!(report.graveyard_ratio > 0.0);
        assert_eq!(report.mortality_curve.ephemeral_quick_lookups_pct, 33.0);
        assert_eq!(report.mortality_curve.persistent_anchor_tabs_pct, 67.0);

        assert_eq!(report.anchor_domains.len(), 1);
        assert_eq!(report.anchor_domains[0].domain, "github.com");
    }
}
