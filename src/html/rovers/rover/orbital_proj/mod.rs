#[cfg(feature = "sp3")]
mod sp3;

#[cfg(feature = "sp3")]
use sp3::Projection as SP3Projection;

mod brdc;
use brdc::Projection as BrdcProjection;

use crate::{
    prelude::Rinex,
    context::QcContext,
    config::QcOrbitPreference
};

pub enum Projection {
    Brdc(BrdcProjection),
    #[cfg(feature = "sp3")]
    SP3(SP3Projection),
}


impl Projection {
    pub fn new(ctx: &QcContext, observations: &Rinex) -> Self {

        match ctx.configuration.orbit_preference {
            QcOrbitPreference::RadioBroadcast => {
                
            },
            QcOrbitPreference::PreciseProducts => {

            },
        }
    }
}