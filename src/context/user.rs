//! Options to customize QcContext behavior
use thiserror::Error;

use crate::prelude::{TimeScale, TimeSeries};

// #[cfg(feature = "serde")]
// use serde::{Deserialize, Serialize};

/// [QcContextPreferences] are used to customize how
/// the [QcContext] will be processed and will behave.
#[derive(Debug, Clone, PartialEq, Eq)]
// #[cfg_attr(feature = "serde", Derive(Serialize, Deserialize))]
pub struct QcContextPreferences {
    /// Prefered [TimeScale].
    /// All data points and solutions will be expressed in given [TimeScale].
    pub timescale: TimeScale,

    /// The prefered reference Frame ID.
    /// Can be used to manually select a reference frame.
    #[cfg(feature = "navigation")]
    pub frame_uid: u8,
    
    /// User this flag to force the update to the JPL database.
    /// This serves no purpose when using a difference frame than the JPL frame.
    /// When using this frame, it is recommended to keep it up to date,
    /// with regular network access.
    #[cfg(feature = "navigation")]
    pub force_jpl_update: bool,

    /// Possible temporal arc, used to crop the temporal axis
    /// in future analysis. Data points sampled outside of this arc
    /// will be dropped out, they will not contribute to the solutions
    /// and will not be analyzed.
    pub time_window: Option<TimeSeries>,
}

impl Default for QcContextPreferences {
    /// Defines default [QcContextPreferences]:
    /// - timescale set to [TimeScale::GPST]
    /// - no temporal axis filter
    fn default() -> Self {
        Self {
            time_window: None,
            timescale: TimeScale::GPST,
            #[cfg(feature = "navigation")],
            frame_uid: EARTH_J2000,
            #[cfg(feature = "navigation")]
            force_jpl_update: false,
        }
    }
}

#[cfg(feature = "navigation")]
impl QcContextPreferences {
    /// Creates a DE440 ANISE [MetaFile]
    fn anise_de440s_bsp() -> MetaFile {
        MetaFile {
            crc32: Some(0x7286750a),
            uri: String::from("http://public-data.nyxspace.com/anise/de440s.bsp"),
        }
    }

    /// Creates a PCK11 PCA ANISE [MetaFile]
    fn anise_pck11_pca() -> MetaFile {
        MetaFile {
            crc32: Some(0x8213b6e9),
            uri: String::from("http://public-data.nyxspace.com/anise/v0.5/pck11.pca"),
        }
    }

    /// Creates a JPL BPC ANISE [MetaFile]
    fn anise_jpl_bpc() -> MetaFile {
        MetaFile {
            crc32: None,
            uri:
                "https://naif.jpl.nasa.gov/pub/naif/generic_kernels/pck/earth_latest_high_prec.bpc"
                    .to_string(),
        }
    }

    /// Creates an ANISE [MetaAlmanac] from these preferences
    fn to_meta_almanac(&self) -> MetaAlmanac {
        let mut meta = 
            MetaAlmanac {
                files: vec![Self::anise_pck11_pca(), Self::anise_de440s_bsp()],
            };

        if self.frame_uid == JPL {
            meta.files.push(Self::anise_jpl_bpc());
        }
    }
    
    /// Creates [Almanac] and [Frame] definition from these preferences.
    fn to_almanac_frame(&self) -> Result<(Almanac, Frame), Error> {
        let frame = Frame::from_id(self.frame_id)?;
        let almanac = self.to_meta_almanac().process(false)?;
        let frame = almanac.frame_from_uid(self.frame_uid)?;
        Ok((almanac, frame))
    }
}
