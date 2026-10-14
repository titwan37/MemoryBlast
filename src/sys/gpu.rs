use std::collections::HashMap;
use windows::core::{w, PCWSTR};
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
};

/// Default number of samples before recreating the PDH query to discover newly spawned processes.
const REFRESH_INTERVAL_SAMPLES: u32 = 20;

/// Windows PDH engine for capturing per-process GPU utilization.
/// Queries `\GPU Engine(*)\Utilization Percentage` across 3D, Compute (CUDA), Copy, and Video engines.
pub struct GpuSampler {
    query: isize,
    counter: isize,
    sample_count: u32,
    last_samples: HashMap<u32, f32>,
}

unsafe impl Send for GpuSampler {}

impl GpuSampler {
    /// Creates a new `GpuSampler` instance, initializing the PDH query.
    pub fn new() -> Option<Self> {
        let mut sampler = Self {
            query: 0,
            counter: 0,
            sample_count: 0,
            last_samples: HashMap::new(),
        };

        if sampler.init_query() {
            Some(sampler)
        } else {
            None
        }
    }

    /// Initializes or re-initializes the underlying PDH query with wildcard counter.
    fn init_query(&mut self) -> bool {
        self.close_query();

        unsafe {
            let mut query = 0;
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return false;
            }

            let mut counter = 0;
            let status = PdhAddEnglishCounterW(
                query,
                w!("\\GPU Engine(*)\\Utilization Percentage"),
                0,
                &mut counter,
            );

            if status != 0 {
                let _ = PdhCloseQuery(query);
                return false;
            }

            // Prime the initial counter baseline
            let _ = PdhCollectQueryData(query);

            self.query = query;
            self.counter = counter;
            self.sample_count = 0;
            true
        }
    }

    /// Closes the active query handle to prevent resource leaks.
    fn close_query(&mut self) {
        if self.query != 0 {
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
            self.query = 0;
            self.counter = 0;
        }
    }

    /// Samples GPU engine counters and aggregates utilization percentages by Process ID.
    pub fn sample(&mut self) -> HashMap<u32, f32> {
        if self.query == 0 && !self.init_query() {
            return self.last_samples.clone();
        }

        // Recreate query periodically to discover newly launched processes
        if self.sample_count >= REFRESH_INTERVAL_SAMPLES {
            self.init_query();
            return self.last_samples.clone();
        }

        self.sample_count = self.sample_count.saturating_add(1);

        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return self.last_samples.clone();
            }

            let mut buffer_size: u32 = 0;
            let mut item_count: u32 = 0;

            let _ = PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                None,
            );

            if buffer_size == 0 {
                return self.last_samples.clone();
            }

            // Allocate an 8-byte aligned buffer of u64 with extra headroom so PDH never writes past capacity
            let words_needed = (buffer_size as usize + 7) / 8 + 64;
            let mut buffer: Vec<u64> = vec![0u64; words_needed];
            let mut actual_buffer_bytes = (buffer.len() * std::mem::size_of::<u64>()) as u32;

            let status = PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut actual_buffer_bytes,
                &mut item_count,
                Some(buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
            );

            if status != 0 {
                return self.last_samples.clone();
            }

            let items = std::slice::from_raw_parts(
                buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
                item_count as usize,
            );

            let mut pid_gpu: HashMap<u32, f32> = HashMap::with_capacity(64);

            for item in items {
                if item.FmtValue.CStatus != 0 || item.szName.is_null() {
                    continue;
                }

                let Ok(name) = item.szName.to_string() else {
                    continue;
                };

                let val = item.FmtValue.Anonymous.doubleValue;
                if val <= 0.001 {
                    continue;
                }

                if let Some(rest) = name.strip_prefix("pid_") {
                    if let Some(idx) = rest.find('_') {
                        if let Ok(pid) = rest[..idx].parse::<u32>() {
                            let entry = pid_gpu.entry(pid).or_insert(0.0);
                            *entry += val as f32;
                        }
                    }
                }
            }

            // Cap individual process GPU % at reasonable boundary
            for usage in pid_gpu.values_mut() {
                if *usage > 100.0 {
                    *usage = 100.0;
                }
            }

            self.last_samples = pid_gpu;
            self.last_samples.clone()
        }
    }
}

impl Drop for GpuSampler {
    fn drop(&mut self) {
        self.close_query();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_sampler_loop() {
        if let Some(mut sampler) = GpuSampler::new() {
            for i in 0..60 {
                let samples = sampler.sample();
                println!("Iteration {}: Sampled {} processes", i, samples.len());
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}
