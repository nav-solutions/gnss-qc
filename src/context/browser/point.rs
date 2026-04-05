use crate::prelude::{SV, Carrier};

#[cfg(feature = "nav")]
use crate::prelude::Vector3

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum QcDataPoint {
    /// [SignalObservation] data point
    SignalObservation(SignalObservation),

    /// [SatelliteState]
    #[cfg_attr(feature = "nav")]
    SatelliteState(SatelliteState),
    
    /// [SatelliteClock]
    #[cfg_attr(feature = "nav")]
    SatelliteClock(SatelliteClock),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SignalObservation {
    /// satellite: signal source
    pub satellite: SV,

    /// carrier: signal type
    pub carrier: Carrier,

    /// signal to noise ratio in dB
    pub cn0_db: Option<f64>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg(feature = "nav")]
impl SatelliteState {
    /// satellite
    pub satellite: SV,
    
    /// True if this state was predicted, not observed.
    pub observed: bool,

    /// state
    pub state: Vector3,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg(feature = "nav")]
pub struct SatelliteClock {
    /// satellite
    pub satellite: SV,

    /// state with respect to prefered timescale
    pub state: Vector3,
}
