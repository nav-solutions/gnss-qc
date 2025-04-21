use crate::prelude::QcContext;

use rinex::{
    navigation::Ephemeris,
    prelude::{Epoch, SV},
};

pub struct QcEphemerisData {
    pub sv: SV,
    pub toe: Epoch,
    pub toc: Epoch,
    pub ephemeris: Ephemeris,
}

pub struct QcEphemerisBuffer<'a> {
    pub iter: Box<dyn Iterator<Item = QcEphemerisData> + 'a>,
}

impl QcContext {
    /// Obtain [QcEphemerisBuffer] from this [QcContext].
    pub fn ephemeris_buffer<'a>(&'a self) -> Option<QcEphemerisBuffer<'a>> {
        let brdc = self.brdc_navigation()?;

        Some(QcEphemerisBuffer {
            iter: Box::new(brdc.nav_ephemeris_frames_iter().filter_map(|(k, v)| {
                let sv_ts = k.sv.constellation.timescale()?;
                let toe = v.toe(sv_ts)?;
                Some(QcEphemerisData {
                    toe,
                    toc: k.epoch,
                    sv: k.sv,
                    ephemeris: v.clone(),
                })
            })),
        })
    }
}
