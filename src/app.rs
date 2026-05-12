use std::collections::VecDeque;
use nvml_wrapper::Nvml;
use sysinfo::{
    CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, Networks, ProcessRefreshKind,
    RefreshKind, System,
};
use crate::render3d::Mesh;

pub const HISTORY_LEN: usize = 60;

#[allow(dead_code)]
pub enum CurrentScreen {
    Main,
    CPUScreen,
    GPUScreen,
    MemoryScreen,
    NetworkScreen,
    DiskScreen,
    TaskListScreen,
}

#[allow(dead_code)]
pub struct App {
    pub exit: bool,
    pub current_screen: CurrentScreen,
    pub cpu_history: VecDeque<f64>,
    pub ram_history: VecDeque<f64>,
    pub gpu_memory_history: VecDeque<f64>,
    pub net_download_history: VecDeque<f64>,
    pub net_upload_history: VecDeque<f64>,
    pub disk_history: VecDeque<f64>,
    pub networks: Networks,
    pub nvml: Option<Nvml>,
    pub processes: System,
    pub system_cpu: System,
    pub system_memory: System,
    pub disks: Disks,
    pub cpu_per_core_history: Vec<VecDeque<f64>>,
    pub mesh: Mesh,
    pub frame_count: u64,
}

impl App {
    pub fn new(mesh: Mesh) -> App {
        let system_cpu = System::new_with_specifics(
            RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
        );
        let core_count = system_cpu.cpus().len();
        App {
            current_screen: CurrentScreen::Main,
            exit: false,
            cpu_history: VecDeque::with_capacity(HISTORY_LEN),
            ram_history: VecDeque::with_capacity(HISTORY_LEN),
            gpu_memory_history: VecDeque::with_capacity(HISTORY_LEN),
            net_download_history: VecDeque::with_capacity(HISTORY_LEN),
            net_upload_history: VecDeque::with_capacity(HISTORY_LEN),
            disk_history: VecDeque::with_capacity(HISTORY_LEN),
            networks: Networks::new_with_refreshed_list(),
            nvml: Nvml::init().ok(),
            processes: System::new_with_specifics(
                RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
            ),
            system_cpu,
            system_memory: System::new_with_specifics(
                RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
            ),
            disks: Disks::new_with_refreshed_list_specifics(DiskRefreshKind::everything()),
            cpu_per_core_history: (0..core_count)
                .map(|_| VecDeque::with_capacity(HISTORY_LEN))
                .collect(),
            mesh,
            frame_count: 0,
        }
    }
}

pub fn push_history(buf: &mut VecDeque<f64>, value: f64) {
    if buf.len() >= HISTORY_LEN {
        buf.pop_front();
    }
    buf.push_back(value);
}

pub fn to_sparkline_data(buf: &VecDeque<f64>, scale: f64) -> Vec<u64> {
    buf.iter().map(|v| (v * scale) as u64).collect()
}

pub fn format_bytes_per_sec(bps: f64) -> String {
    if bps >= 1_000_000.0 {
        format!("{:.1} MB/s", bps / 1_000_000.0)
    } else if bps >= 1_000.0 {
        format!("{:.1} KB/s", bps / 1_000.0)
    } else {
        format!("{:.0} B/s", bps)
    }
}

pub fn format_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const KB: f64 = 1024.0;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

pub fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m:02}m {s:02}s")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}
