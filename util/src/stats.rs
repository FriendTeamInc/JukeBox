use crate::smallstr::SmallStr;

// We assume each string here only uses ASCII characters and thus each character can fit in a single byte.

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SystemStats {
    pub cpu_name: SmallStr<{ 20 + 1 }>,
    pub cpu_usage: SmallStr<{ 5 + 1 }>,
    pub cpu_temperature: SmallStr<{ 5 + 1 }>,

    pub memory_used: SmallStr<{ 5 + 1 }>,
    pub memory_total: SmallStr<{ 5 + 1 }>,
    pub memory_unit: SmallStr<{ 2 + 1 }>,

    pub gpu_name: SmallStr<{ 20 + 1 }>,
    pub gpu_usage: SmallStr<{ 5 + 1 }>,
    pub gpu_temperature: SmallStr<{ 5 + 1 }>,

    pub vram_used: SmallStr<{ 5 + 1 }>,
    pub vram_total: SmallStr<{ 5 + 1 }>,
    pub vram_unit: SmallStr<{ 2 + 1 }>,
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

    pub fn encode(self) -> [u8; 100] {
        let mut data = [0u8; 100];

        let d = [
            &self.cpu_name.encode()[..],
            &self.cpu_usage.encode()[..],
            &self.cpu_temperature.encode()[..],
            &self.memory_used.encode()[..],
            &self.memory_total.encode()[..],
            &self.memory_unit.encode()[..],
            &self.gpu_name.encode()[..],
            &self.gpu_usage.encode()[..],
            &self.gpu_temperature.encode()[..],
            &self.vram_used.encode()[..],
            &self.vram_total.encode()[..],
            &self.vram_unit.encode()[..],
        ];

        let mut size = 0usize;
        for d in d {
            let len = d.len();
            data[size..size + len].copy_from_slice(d);
            size += len;
        }

        data
    }

    pub fn decode(data: &[u8]) -> Self {
        let mut s = Self::default();

        s.cpu_name = SmallStr::decode(&data[..21]);
        s.cpu_usage = SmallStr::decode(&data[21..27]);
        s.cpu_temperature = SmallStr::decode(&data[27..33]);
        s.memory_used = SmallStr::decode(&data[33..39]);
        s.memory_total = SmallStr::decode(&data[39..45]);
        s.memory_unit = SmallStr::decode(&data[45..48]);
        s.gpu_name = SmallStr::decode(&data[48..69]);
        s.gpu_usage = SmallStr::decode(&data[69..75]);
        s.gpu_temperature = SmallStr::decode(&data[75..81]);
        s.vram_used = SmallStr::decode(&data[81..87]);
        s.vram_total = SmallStr::decode(&data[87..93]);
        s.vram_unit = SmallStr::decode(&data[93..]);

        s
    }
}
