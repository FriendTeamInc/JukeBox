// System stats monitoring for displaying on certain jukebox devices

use std::{collections::VecDeque, sync::Arc, thread::sleep};

use anyhow::Result;
use jukebox_util::{smallstr::SmallStr, stats::SystemStats};
use sysinfo::{Components, MemoryRefreshKind, System, MINIMUM_CPU_UPDATE_INTERVAL};
use tokio::sync::Mutex;

use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, Nvml};
#[cfg(feature = "amd_gpu")]
use rocm_smi_lib::{RocmSmi, RsmiTemperatureMetric, RsmiTemperatureType};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum GpuPreference {
    Nvidia,
    Amd,
    None,
}

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

    pub fn get_cpu_name(cpu_vendor: String, cpu_brand: String) -> String {
        log::debug!("cpu info: {} {}", cpu_vendor, cpu_brand);
        let mut cpu_name = match cpu_vendor.as_str() {
            "GenuineIntel" => {
                // TODO
                cpu_brand
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
                cpu_brand
                    .replace("Ryzen 3", "R3")
                    .replace("Ryzen 5", "R5")
                    .replace("Ryzen 7", "R7")
                    .replace("Ryzen 9", "R9")
                    .replace("1-Core ", "")
                    .replace("2-Core ", "")
                    .replace("4-Core ", "")
                    .replace("6-Core ", "")
                    .replace("8-Core ", "")
                    .replace("12-Core ", "")
                    .replace("16-Core ", "")
                    .replace("24-Core ", "")
                    .replace("32-Core ", "")
                    .replace("Processor", "")
                    .trim()
                    .to_string()
            }
            _ => cpu_brand,
        };

        cpu_name.truncate(20);

        cpu_name
    }

    pub fn to_system_stats(self, cpu_name: String, gpu_info: String) -> SystemStats {
        let cpu_usage = format!("{: >5.1}", self.cpu_usage);
        let (memory_used, memory_total, memory_unit) =
            Self::format_memory(self.memory_used, self.memory_total);

        // sysinfo crate does not provide temperature on windows machines currently
        // TODO: when it eventually does, we can remove this
        let cpu_temperature = if cfg!(target_os = "windows") {
            String::from("  N/A")
        } else {
            format!("{: >5.1}", self.cpu_temperature)
        };

        let mut gpu_name = gpu_info.replace("GeForce", "").trim().to_string();
        gpu_name.truncate(20);

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
    let mut cmps = Components::new_with_refreshed_list();
    log::info!("components:");
    for c in &cmps {
        log::info!("{c:?}");
    }

    // heuristic to figure out which gpu stats to get
    // we can reasonably assume a user won't have both (probably) and the same count (hopefully)
    // if they do, then they probably don't have any discrete gpu installed (for sanity's sake)
    // TODO: enable amd gpu support when cross compiling the rocm lib is possible
    #[cfg(feature = "amd_gpu")]
    let (gpu_preference, nvml, mut rocm) = {
        let nvml = Nvml::init();
        let mut rocm = RocmSmi::init();
        let gpu_preference = match (&nvml, rocm.as_mut()) {
            // if we find both nvidia and amd support, we default to nvidia because its more popular.
            // this may need to change in the future.
            (Ok(_), Ok(_)) => GpuPreference::Nvidia,
            (Ok(_), Err(_)) => GpuPreference::Nvidia,
            (Err(_), Ok(_)) => GpuPreference::Amd,
            (Err(_), Err(_)) => GpuPreference::None,
        };
        log::info!("gpu preference: {:?}", gpu_preference);

        (gpu_preference, nvml, rocm)
    };
    #[cfg(not(feature = "amd_gpu"))]
    let (gpu_preference, nvml) = {
        let nvml = Nvml::init();
        let gpu_preference = match &nvml {
            Ok(_) => GpuPreference::Nvidia,
            Err(_) => GpuPreference::None,
        };
        log::info!("gpu preference: {:?}", gpu_preference);

        (gpu_preference, nvml)
    };

    let mut cpu_info: Option<String> = None;
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
                cpu_info = Some(StatReport::get_cpu_name(
                    cpu.vendor_id().into(),
                    cpu.brand().into(),
                ));
            }

            cpu_usage += cpu.cpu_usage();

            cpu_count += 1;
        }

        let cpu_usage = cpu_usage / (cpu_count as f32);

        let mut cpu_temperature = 0.0;
        cmps.refresh(false);
        for c in &cmps {
            // TODO: find more cpu temp labels
            match c.label() {
                "Tctl" => {
                    cpu_temperature = c.temperature().unwrap();
                }
                _ => (),
            }
        }

        // Get Memory Info
        sys.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
        let memory_used = sys.used_memory();
        let memory_total = sys.total_memory();

        // Get GPU Info
        #[allow(unused_mut)]
        let (mut gpu_info, mut gpu_usage, mut gpu_temperature, mut vram_used, mut vram_total) =
            (String::from("Unknown GPU"), 0.0f32, 0.0f32, 0u64, 0u64);

        #[cfg(feature = "amd_gpu")]
        match gpu_preference {
            GpuPreference::Nvidia => {
                if let Ok(n) = &nvml {
                    if let Ok(d) = n.device_by_index(0) {
                        if let Ok(name) = d.name() {
                            gpu_info = name;
                        }
                        if let Ok(rates) = d.utilization_rates() {
                            gpu_usage = rates.gpu as f32;
                        }
                        if let Ok(temp) = d.temperature(TemperatureSensor::Gpu) {
                            gpu_temperature = temp as f32;
                        }
                        if let Ok(memory) = d.memory_info() {
                            vram_used = memory.used;
                            vram_total = memory.total;
                        }
                    }
                }
            }
            GpuPreference::Amd => {
                if let Ok(r) = &mut rocm {
                    if let Ok(name) = r.get_device_identifiers(0) {
                        gpu_info = name.name.unwrap();
                    }
                    if let Ok(busy) = r.get_device_busy_percent(0) {
                        gpu_usage = busy as f32;
                    }
                    if let Ok(temp) = r.get_device_temperature_metric(
                        0,
                        RsmiTemperatureType::Junction,
                        RsmiTemperatureMetric::Current,
                    ) {
                        gpu_temperature = temp as f32;
                    }
                    if let Ok(memory) = r.get_device_memory_data(0) {
                        vram_used = memory.vram_used;
                        vram_total = memory.vram_total;
                    }
                }
            }
            GpuPreference::None => {}
        }
        #[cfg(not(feature = "amd_gpu"))]
        match gpu_preference {
            GpuPreference::Nvidia => {
                if let Ok(n) = &nvml {
                    if let Ok(d) = n.device_by_index(0) {
                        if let Ok(name) = d.name() {
                            gpu_info = name;
                        }
                        if let Ok(rates) = d.utilization_rates() {
                            gpu_usage = rates.gpu as f32;
                        }
                        if let Ok(temp) = d.temperature(TemperatureSensor::Gpu) {
                            gpu_temperature = temp as f32;
                        }
                        if let Ok(memory) = d.memory_info() {
                            vram_used = memory.used;
                            vram_total = memory.total;
                        }
                    }
                }
            }
            _ => {}
        }

        // push report, we take an average of the past few samples to put in the mutex
        stat_reports.push_back(StatReport {
            cpu_usage,
            cpu_temperature,
            memory_used,
            memory_total,

            gpu_usage,
            gpu_temperature,
            vram_used,
            vram_total,
        });
        if stat_reports.len() > 20 {
            let _ = stat_reports.pop_front();
        }

        *system_stats.blocking_lock() = StatReport::avg(stat_reports.clone())
            .to_system_stats(cpu_info.clone().unwrap(), gpu_info);

        sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    }

    // Ok(())
}
