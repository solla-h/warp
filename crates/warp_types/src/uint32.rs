use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct Uint32(pub u32);

impl From<u32> for Uint32 {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<Uint32> for u32 {
    fn from(value: Uint32) -> Self {
        value.0
    }
}
