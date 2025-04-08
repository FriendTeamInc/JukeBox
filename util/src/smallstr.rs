use core::ptr::addr_of;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SmallStr<const N: usize> {
    pub str: [u8; N],
    pub size: u8,
}
impl<const N: usize> SmallStr<N> {
    pub const fn default() -> Self {
        Self {
            str: [0u8; N],
            size: 0,
        }
    }

    pub fn from_str(s: &str) -> Self {
        assert!(s.len() <= N);

        let mut str = [0u8; N];
        str[..s.len()].copy_from_slice(s.as_bytes());
        // the line above is not const yet :(

        Self {
            str: str,
            size: s.len() as u8,
        }
    }

    pub fn to_str(&self) -> &str {
        unsafe { core::str::from_utf8(&*addr_of!(self.str[..self.size as usize])).unwrap() }
    }

    pub fn encode(self) -> [u8; N] {
        assert!(N >= (self.size + 1) as usize);

        let mut data = [0u8; N];

        data[0] = self.size;
        data[1..(self.size as usize) + 1].copy_from_slice(&self.str[..self.size as usize]);

        data
    }

    pub fn decode(data: &[u8]) -> Self {
        let mut str = [0u8; N];
        let size = data[0];

        for i in 0..size as usize {
            str[i] = data[i + 1];
        }

        Self { str, size }
    }
}
