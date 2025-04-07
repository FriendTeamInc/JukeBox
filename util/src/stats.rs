use crate::smallstr::SmallStr;

// We assume each string here only uses ASCII characters and thus each character can fit in a single byte.

#[derive(Debug, Clone, PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SystemStats {
    cpu_name: SmallStr<20>,
    cpu_usage: SmallStr<5>,
    cpu_temperature: SmallStr<5>,

    memory_used: SmallStr<5>,
    memory_total: SmallStr<5>,
    memory_unit: SmallStr<2>,

    gpu_name: SmallStr<20>,
    gpu_usage: SmallStr<5>,
    gpu_temperature: SmallStr<5>,

    vram_used: SmallStr<5>,
    vram_total: SmallStr<5>,
    vram_unit: SmallStr<2>,
}
impl SystemStats {
    pub const fn default() -> Self {
        Self {
            cpu_name: SmallStr::default(),
            cpu_usage: SmallStr::default(),
            cpu_temperature: SmallStr::default(),

            memory_used: SmallStr::default(),
            memory_total: SmallStr::default(),
            memory_unit: SmallStr::default(),

            gpu_name: SmallStr::default(),
            gpu_usage: SmallStr::default(),
            gpu_temperature: SmallStr::default(),

            vram_used: SmallStr::default(),
            vram_total: SmallStr::default(),
            vram_unit: SmallStr::default(),
        }
    }
}
