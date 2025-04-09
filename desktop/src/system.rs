// TODO: add CPU and Memory monitoring support through sysinfo crate
// TODO: add GPU monitoring support to Rust version through:
// - nvml-wrapper crate (NVIDIA)
// - rocm_smi_lib crate (AMD)
// - Intel Graphics Control Library through Rust wrappers

use std::{collections::VecDeque, sync::Arc, thread::sleep};

use anyhow::Result;
use jukebox_util::{smallstr::SmallStr, stats::SystemStats};
use sysinfo::{MemoryRefreshKind, System, MINIMUM_CPU_UPDATE_INTERVAL};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy)]
struct StatReport {
    pub cpu_usage: f32,
    pub cpu_temperature: f32,
    pub memory_used: u64,
    pub memory_total: u64,

    pub gpu_usage: f32,
    pub gpu_temperature: f32,
    pub vram_used: u64,
    pub vram_total: u64,
}
impl StatReport {
    pub fn avg(reports: VecDeque<Self>) -> Self {
        let count = reports.len();
        let mut cpu_usage = 0.0;
        let mut cpu_temperature = 0.0;
        let mut memory_used = 0;
        let mut memory_total = 0;
        let mut gpu_usage = 0.0;
        let mut gpu_temperature = 0.0;
        let mut vram_used = 0;
        let mut vram_total = 0;

        for r in reports {
            cpu_usage += r.cpu_usage;
            cpu_temperature += r.cpu_temperature;
            memory_used += r.memory_used;
            memory_total = r.memory_total;
            gpu_usage += r.gpu_usage;
            gpu_temperature += r.gpu_temperature;
            vram_used += r.vram_used;
            vram_total = r.vram_total;
        }

        StatReport {
            cpu_usage: cpu_usage / (count as f32),
            cpu_temperature: cpu_temperature / (count as f32),
            memory_used: ((memory_used as f64) / (count as f64)) as u64,
            memory_total: memory_total,
            gpu_usage: gpu_usage / (count as f32),
            gpu_temperature: gpu_temperature / (count as f32),
            vram_used: ((vram_used as f64) / (count as f64)) as u64,
            vram_total: vram_total,
        }
    }

    fn format_memory(memory_used: u64, memory_total: u64) -> (String, String, String) {
        // TODO: support TB memory
        let memory_used = format!(
            "{: >5.1}",
            (memory_used as f64) / (1024.0 * 1024.0 * 1024.0)
        );
        let memory_total = format!(
            "{: >5.1}",
            (memory_total as f64) / (1024.0 * 1024.0 * 1024.0)
        );
        (memory_used, memory_total, "GB".into())
    }

    pub fn to_system_stats(self, cpu_info: (String, String)) -> SystemStats {
        let mut cpu_name = match cpu_info.0.as_str() {
            "GenuineIntel" => {
                // TODO
                cpu_info
                    .1
                    .replace("Core i3 ", "i3-")
                    .replace("Core i5 ", "i5-")
                    .replace("Core i7 ", "i7-")
                    .replace("Core i9 ", "i9-")
                    .replace("Processor", "")
                    .trim()
                    .to_string()
            }
            "AuthenticAMD" => {
                // TODO
                cpu_info
                    .1
                    .replace("Ryzen 3", "R3")
                    .replace("Ryzen 5", "R5")
                    .replace("Ryzen 7", "R7")
                    .replace("Ryzen 9", "R9")
                    .replace("1-Core", "")
                    .replace("2-Core", "")
                    .replace("4-Core", "")
                    .replace("6-Core", "")
                    .replace("8-Core", "")
                    .replace("12-Core", "")
                    .replace("16-Core", "")
                    .replace("24-Core", "")
                    .replace("32-Core", "")
                    .replace("Processor", "")
                    .trim()
                    .to_string()
            }
            _ => cpu_info.1,
        };
        cpu_name.truncate(18);

        let cpu_usage = format!("{: >5.1}", self.cpu_usage);
        let cpu_temperature = format!("{: >5.1}", self.cpu_temperature);
        let (memory_used, memory_total, memory_unit) =
            Self::format_memory(self.memory_used, self.memory_total);

        let gpu_name = "".to_string();
        let gpu_usage = format!("{: >5.1}", self.gpu_usage);
        let gpu_temperature = format!("{: >5.1}", self.gpu_temperature);
        let (vram_used, vram_total, vram_unit) =
            Self::format_memory(self.vram_used, self.vram_total);

        SystemStats {
            cpu_name: SmallStr::from_str(&cpu_name),
            cpu_usage: SmallStr::from_str(&cpu_usage),
            cpu_temperature: SmallStr::from_str(&cpu_temperature),
            memory_used: SmallStr::from_str(&memory_used),
            memory_total: SmallStr::from_str(&memory_total),
            memory_unit: SmallStr::from_str(&memory_unit),
            gpu_name: SmallStr::from_str(&gpu_name),
            gpu_usage: SmallStr::from_str(&gpu_usage),
            gpu_temperature: SmallStr::from_str(&gpu_temperature),
            vram_used: SmallStr::from_str(&vram_used),
            vram_total: SmallStr::from_str(&vram_total),
            vram_unit: SmallStr::from_str(&vram_unit),
        }
    }
}

pub fn system_task(system_stats: Arc<Mutex<SystemStats>>) -> Result<()> {
    let mut sys = System::new();

    let mut cpu_info: Option<(String, String)> = None;
    let mut stat_reports: VecDeque<StatReport> = VecDeque::new();

    loop {
        // Get CPU info
        sys.refresh_cpu_all();
        let mut cpu_usage = 0.0;
        // let mut cpu_temperature = 0;
        let mut cpu_count = 0;

        for cpu in sys.cpus() {
            // I'm not aware of any generally-available system that has hot swappable CPUs.
            // But if it exists and can run JukeBox Desktop, I'd love to meet it.
            if cpu_info.is_none() {
                cpu_info = Some((cpu.vendor_id().into(), cpu.brand().into()));
            }

            cpu_usage += cpu.cpu_usage();

            cpu_count += 1;
        }

        let cpu_usage = cpu_usage / (cpu_count as f32);

        // Get Memory Info
        sys.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
        let memory_used = sys.used_memory();
        let memory_total = sys.total_memory();

        // push report, we take an average of the past 5 to put in the mutex
        stat_reports.push_back(StatReport {
            cpu_usage: cpu_usage,
            cpu_temperature: 0.0,
            memory_used: memory_used,
            memory_total: memory_total,
            gpu_usage: 0.0,
            gpu_temperature: 0.0,
            vram_used: 0,
            vram_total: 0,
        });
        if stat_reports.len() > 10 {
            let _ = stat_reports.pop_front();
        }

        *system_stats.blocking_lock() =
            StatReport::avg(stat_reports.clone()).to_system_stats(cpu_info.clone().unwrap());

        sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    }

    // Ok(())
}
