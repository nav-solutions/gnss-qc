use std::path::Path;
use thiserror::Error;

use crate::io::input::{indexing::QcIndexing, types::QcProductType};

use hifitime::prelude::Epoch;

#[derive(Debug, Error)]
pub enum IndexingError {
    /// InputFileName could not be determined (OS issue)
    #[error("could not determine input file name")]
    InputFileName,

    #[error("could not determine input file creation meta")]
    InputFileMeta,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct QcProductKey {
    /// File name of this input product.
    pub file_name: String,

    /// Custom alias used in complex indexing method.
    pub alias: String,
}

impl QcProductKey {}

// #[cfg(test)]
// mod test {
//     use crate::input::key::QcProductKey;
//     use crate::input::types::QcProductType;
//
//     #[test]
//     fn test_obs_v2_rinex_key_guessing() {
//         let key = QcProductKey::guess_from_path("../../data/OBS/V2/wsra0010.21o")
//             .unwrap_or_else(|e| {
//                 panic!("Failed to guess InputProductKey for OBS/V2/wsra0010: {}", e);
//             });
//
//         assert_eq!(key.name, "wsra0010.21o");
//         assert_eq!(key.alias, "wsra0010.21o");
//         assert_eq!(key.product_type, QcProductType::RINEX);
//     }
//
//     #[test]
//     fn test_obs_v3_rinex_key_guessing() {
//         let key = QcProductKey::guess_from_path("../../data/OBS/V3/DUTH0630.22O")
//             .unwrap_or_else(|e| {
//                 panic!("Failed to guess InputProductKey for OBS/V3/DUTH0630: {}", e);
//             });
//
//         assert_eq!(key.name, "DUTH0630.22O");
//         assert_eq!(key.alias, "DUTH0630");
//         assert_eq!(key.product_type, QcProductType::RINEX);
//     }
//
//     #[test]
//     fn test_sp3_a_key_guessing() {
//         let key = QcProductKey::guess_from_path("../../data/SP3/A/emr08874.sp3")
//             .unwrap_or_else(|e| {
//                 panic!("Failed to guess InputProductKey for SP3/A/emr08874: {}", e);
//             });
//
//         assert_eq!(key.name, "emr08874.sp3");
//         assert_eq!(key.alias, "emr08874");
//         assert_eq!(key.product_type, QcProductType::SP3);
//     }
//
//     #[test]
//     fn test_sp3_c_key_guessing() {
//         let key = QcProductKey::guess_from_path("../../data/SP3/C/em108871.sp3")
//             .unwrap_or_else(|e| {
//                 panic!("Failed to guess InputProductKey for SP3/C/em108871: {}", e);
//             });
//
//         assert_eq!(key.name, "em108871");
//         assert_eq!(key.alias, "em108871");
//         assert_eq!(key.product_type, QcProductType::SP3);
//     }
//
//     #[test]
//     fn test_sp3_d_key_guessing() {
//         let key = QcProductKey::guess_from_path("../../data/SP3/D/Sta21114.sp3.gz")
//             .unwrap_or_else(|e| {
//                 panic!("Failed to guess InputProductKey for SP3/D/Sta21114.sp3: {}", e);
//             });
//
//         assert_eq!(key.name, "Sta21114.sp3");
//         assert_eq!(key.alias, "Sta21114");
//         assert_eq!(key.product_type, QcProductType::SP3);
//     }
// }
