use std::{fmt::Debug, ops::Add};

#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
#[repr(transparent)]
pub struct u24([u8; 3]);
// impl Add for u24 {
//     type Output = Self;
//     fn add(self, rhs: Self) -> Self {
//         Self::from_u32(self.to_u32() + rhs.to_u32())
//     }
// }

impl u24 {
    pub fn to_u32(self) -> u32 {
        let u24([a, b, c]) = self;
        u32::from_be_bytes([0, a, b, c])
    }
    pub fn from_u32(n: u32) -> Self {
        let [a, b, c, d] = n.to_le_bytes();
        debug_assert!(d == 0);
        u24([a, b, c])
    }
}

impl From<[u8; 3]> for u24 {
    fn from(value: [u8; 3]) -> Self {
        Self(value)
    }
}

impl Debug for u24 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = self.to_u32();
        write!(f, "{value}")
    }
}
