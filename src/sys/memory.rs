use serde::{Deserialize, Serialize};
use std::mem::{size_of, zeroed};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows::Win32::System::SystemInformation::{
    GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemMemoryStats {
    pub total_physical_bytes: u64,
    pub available_physical_bytes: u64,
    pub used_physical_bytes: u64,
    pub memory_load_percent: u32,

    pub total_page_file_bytes: u64,
    pub available_page_file_bytes: u64,
    pub committed_bytes: u64,

    pub paged_pool_bytes: u64,
    pub non_paged_pool_bytes: u64,
    pub system_cache_bytes: u64,

    pub page_size: u64,
    pub is_admin: bool,
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct SystemPerformanceInformation {
    idle_process_time: i64,
    io_read_transfer_count: i64,
    io_write_transfer_count: i64,
    io_other_transfer_count: i64,
    io_read_operation_count: u32,
    io_write_operation_count: u32,
    io_other_operation_count: u32,
    available_pages: u32,
    committed_pages: u32,
    commit_limit: u32,
    peak_commitment: u32,
    page_fault_count: u32,
    copy_on_write_count: u32,
    transition_count: u32,
    cache_transition_count: u32,
    demand_zero_count: u32,
    page_read_count: u32,
    page_read_io_count: u32,
    cache_read_count: u32,
    cache_io_count: u32,
    dirty_pages_write_count: u32,
    dirty_write_io_count: u32,
    mapped_pages_write_count: u32,
    mapped_write_io_count: u32,
    paged_pool_pages: u32,
    non_paged_pool_pages: u32,
    paged_pool_allocs: u32,
    paged_pool_frees: u32,
    non_paged_pool_allocs: u32,
    non_paged_pool_frees: u32,
    free_system_ptes: u32,
    resident_system_code_page: u32,
    total_system_driver_pages: u32,
    total_system_code_pages: u32,
    non_paged_pool_lookaside_hits: u32,
    paged_pool_lookaside_hits: u32,
    available_paged_pool_pages: u32,
    resident_system_cache_page: u32,
    resident_paged_pool_page: u32,
    resident_system_driver_page: u32,
    cc_fast_read_no_wait: u32,
    cc_fast_read_wait: u32,
    cc_fast_read_resource_miss: u32,
    cc_fast_read_not_possible: u32,
    cc_fast_mdl_read_no_wait: u32,
    cc_fast_mdl_read_wait: u32,
    cc_fast_mdl_read_resource_miss: u32,
    cc_fast_mdl_read_not_possible: u32,
    cc_map_data_no_wait: u32,
    cc_map_data_wait: u32,
    cc_map_data_wait_miss: u32,
    cc_pin_mapped_data_count: u32,
    cc_pin_reading_no_wait: u32,
    cc_pin_reading_wait: u32,
    cc_pin_reading_no_wait_miss: u32,
    cc_copy_read_no_wait: u32,
    cc_copy_read_wait: u32,
    cc_copy_read_no_wait_miss: u32,
    cc_mdl_read_no_wait: u32,
    cc_mdl_read_wait: u32,
    cc_mdl_read_no_wait_miss: u32,
    cc_read_ahead_io_count: u32,
    cc_lazy_write_io_count: u32,
    cc_lazy_write_pages: u32,
    cc_data_flushes: u32,
    cc_data_pages: u32,
    context_switches: u32,
    first_level_tb_fills: u32,
    second_level_tb_fills: u32,
    system_calls: u32,
    cc_total_dirty_pages: u64,
    cc_dirty_page_threshold: u64,
    resident_available_pages: i64,
    shared_commit: u64,
}

#[link(name = "ntdll")]
extern "system" {
    fn NtQuerySystemInformation(
        system_information_class: u32,
        system_information: *mut std::ffi::c_void,
        system_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

const SYSTEM_PERFORMANCE_INFORMATION_CLASS: u32 = 2;

/// Checks whether the current process is running with Administrator / Elevated privileges.
pub fn is_elevated() -> bool {
    unsafe {
        let mut token_handle = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret_len = 0u32;
        let success = GetTokenInformation(
            token_handle,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );

        let _ = CloseHandle(token_handle);
        success.is_ok() && elevation.TokenIsElevated != 0
    }
}

/// Retrieves global system memory statistics using native Win32 & NT APIs.
pub fn get_system_memory_stats() -> SystemMemoryStats {
    let mut stats = SystemMemoryStats::default();
    stats.is_admin = is_elevated();

    // Query system page size
    let page_size = unsafe {
        let mut sys_info: SYSTEM_INFO = zeroed();
        GetSystemInfo(&mut sys_info);
        sys_info.dwPageSize as u64
    };
    stats.page_size = if page_size == 0 { 4096 } else { page_size };

    // Query GlobalMemoryStatusEx
    unsafe {
        let mut mem_status = MEMORYSTATUSEX {
            dwLength: size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };

        if GlobalMemoryStatusEx(&mut mem_status).is_ok() {
            stats.total_physical_bytes = mem_status.ullTotalPhys;
            stats.available_physical_bytes = mem_status.ullAvailPhys;
            stats.used_physical_bytes = mem_status
                .ullTotalPhys
                .saturating_sub(mem_status.ullAvailPhys);
            stats.memory_load_percent = mem_status.dwMemoryLoad;

            stats.total_page_file_bytes = mem_status.ullTotalPageFile;
            stats.available_page_file_bytes = mem_status.ullAvailPageFile;
            stats.committed_bytes = mem_status
                .ullTotalPageFile
                .saturating_sub(mem_status.ullAvailPageFile);
        }
    }

    // Query NtQuerySystemInformation for paged and non-paged pool statistics
    unsafe {
        let mut perf_info: SystemPerformanceInformation = zeroed();
        let mut return_length = 0u32;
        let status = NtQuerySystemInformation(
            SYSTEM_PERFORMANCE_INFORMATION_CLASS,
            &mut perf_info as *mut _ as *mut std::ffi::c_void,
            size_of::<SystemPerformanceInformation>() as u32,
            &mut return_length,
        );

        if status >= 0 {
            stats.paged_pool_bytes = (perf_info.paged_pool_pages as u64) * stats.page_size;
            stats.non_paged_pool_bytes = (perf_info.non_paged_pool_pages as u64) * stats.page_size;
            stats.system_cache_bytes = (perf_info.resident_system_cache_page as u64) * stats.page_size;
        }
    }

    stats
}
