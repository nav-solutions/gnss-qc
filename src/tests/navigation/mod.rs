//! Navigation scenarios (P.V.T solutions), on real data:
//! ESBC00DNK (rover) and MOJN00DNK (base), 2020-06-25, GRG precise orbits.
//!
//! ANISE data: de440s.bsp (or any JPL planetary SPK covering 2020, like de421.bsp)
//! and pck11.pca are loaded from `$GNSS_QC_ANISE_DIR` (or `<repo>/.cache`),
//! and downloaded by ANISE otherwise.
#[cfg(feature = "cggtts")]
mod cggtts;
#[cfg(feature = "ionex")]
mod ipp;
mod kepler;
pub mod scenarios;

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anise::almanac::metaload::{MetaAlmanac, MetaFile};

use crate::prelude::*;

pub use crate::tests::miso::{data_path, load_products, load_rinex};

const ANISE_FILES: [(&str, &str, u32); 2] = [
    (
        "de440s.bsp",
        "https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/de440s.bsp",
        0x7286750a,
    ),
    (
        "pck11.pca",
        "http://public-data.nyxspace.com/anise/v0.10/pck11.pca",
        0x1edb3eac,
    ),
];

fn anise_dir() -> PathBuf {
    match std::env::var("GNSS_QC_ANISE_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => Path::new(env!("CARGO_MANIFEST_DIR")).join(".cache"),
    }
}

/// ANISE [Almanac] (loaded once)
pub fn almanac() -> Almanac {
    static ALMANAC: OnceLock<Almanac> = OnceLock::new();

    ALMANAC
        .get_or_init(|| {
            let dir = anise_dir();

            let files = ANISE_FILES
                .iter()
                .map(|(name, remote, crc32)| {
                    let local = dir.join(name);

                    // any local planetary SPK will do
                    let local = if !local.is_file() && name.ends_with(".bsp") {
                        dir.join("de421.bsp")
                    } else {
                        local
                    };

                    if local.is_file() {
                        MetaFile {
                            uri: local.to_string_lossy().to_string(),
                            crc32: None,
                        }
                    } else {
                        MetaFile {
                            uri: remote.to_string(),
                            crc32: Some(*crc32),
                        }
                    }
                })
                .collect();

            MetaAlmanac { files }.process(true).unwrap_or_else(|e| {
                panic!(
                    "failed to build ANISE almanac: {e}. Place de440s.bsp and pck11.pca in {}",
                    dir.display()
                )
            })
        })
        .clone()
}

pub const ESBC_OBS: &str = "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz";
pub const MOJN_OBS: &str = "CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz";
pub const ESBC_NAV: &str = "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";
pub const GRG_SP3: &str = "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz";

pub fn esbc() -> QcIndexing {
    QcIndexing::Receiver("SEPT POLARX5 #3047937".to_string())
}

pub fn mojn() -> QcIndexing {
    QcIndexing::Receiver("SEPT POLARX5 #3022663".to_string())
}

pub fn epoch(s: &str) -> Epoch {
    use std::str::FromStr;
    Epoch::from_str(s).unwrap()
}
