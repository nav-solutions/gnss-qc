//! Input product
use std::collections::HashMap;

use crate::io::input::{indexing::QcIndexing, key::QcProductKey};

use rinex::Rinex;

#[cfg(feature = "sp3")]
use sp3::prelude::SP3;

// impl QcInputProduct {
//     /// Returns reference to underlying [Rinex] when that applies
//     pub fn as_rinex(&self) -> Option<&Rinex> {
//         match self {
//             Self::RINEX(rinex) => Some(rinex),
//             _ => None,
//         }
//     }
//
//     /// Returns mutable reference to underlying [Rinex] when that applies
//     pub fn as_rinex_mut(&mut self) -> Option<&mut Rinex> {
//         match self {
//             Self::RINEX(rinex) => Some(rinex),
//             _ => None,
//         }
//     }
//
//     /// Returns reference to underlying [SP3] when that applies
//     #[cfg(feature = "sp3")]
//     pub fn as_sp3(&self) -> Option<&SP3> {
//         match self {
//             Self::SP3(sp3) => Some(sp3),
//             _ => None,
//         }
//     }
//
//     /// Returns mutable reference to underlying [SP3] when that applies
//     #[cfg(feature = "sp3")]
//     pub fn as_sp3_mut(&mut self) -> Option<&mut SP3> {
//         match self {
//             Self::SP3(sp3) => Some(sp3),
//             _ => None,
//         }
//     }
// }

/// Structure that holds all supported input products that we can
/// then serialize and process.
pub struct QcInputProducts {
    /// Observation RINEX, indexed by [QcIndexing].
    /// Matching [QcIndexing] are merged together into a single [Rinex] structure that
    /// we can then serialize & process.
    obs_rinex: HashMap<QcProductKey, Rinex>,

    /// Navigation RINEX, indexed by [QcIndexing].
    /// Matching [QcIndexing] are merged together into a single [Rinex] structure that
    /// we can then serialize & process.
    nav_rinex: HashMap<QcProductKey, Rinex>,
}
