use rinex::{
    navigation::Ephemeris,
    prelude::{Epoch, SV},
};

use crate::prelude::Orbit;

#[derive(Debug, Clone)]
pub struct QcEphemerisData {
    /// Time of clock as [Epoch]
    pub toc: Epoch,

    /// Time of issue of [Ephemeris] as [Epoch]
    pub toe: Epoch,

    /// [SV] source
    pub sv: SV,

    /// [Ephemeris]
    pub ephemeris: Ephemeris,
}

impl QcEphemerisData {
    /// Tries to form [QcEphemerisData] from RINEX [Ephemeris]
    pub fn from_ephemeris(sv: SV, toc: Epoch, ephemeris: &Ephemeris) -> Option<Self> {
        let ts = sv.constellation.timescale()?;
        let toe = ephemeris.toe(ts)?;
        Some(Self {
            sv,
            toe,
            toc,
            ephemeris: ephemeris.clone(),
        })
    }

    /// Converts [QcEphemerisData] to ANISE [Orbit]
    fn to_orbit(&self, t: Epoch) -> Option<Orbit> {
        let orbit = self.ephemeris.kepler2position(self.sv, self.toc, t)?;
        Some(orbit)
    }
}
