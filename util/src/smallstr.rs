use core::ptr::addr_of;

#[derive(Debug, Clone, PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SmallStr<const N: usize> {
    str: [u8; N],
    size: u8,
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
}
