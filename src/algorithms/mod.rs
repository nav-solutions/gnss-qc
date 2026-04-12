/// Supported [Algorithms].
mod solutions;

use crate::prelude::QcInputSource;

/// All supported [Algorithms].
pub enum Algorithms {
    #[cfg(feature = "navigation")]
    /// Navigation Solutions using PPP technique
    /// for a specific rover or station. This requires the definition the selected
    /// rovers or stations.
    PppSolutions,

    #[cfg(feature = "cggtts")]
    /// CGGTTS (timing oriented) solution using PPP technique,
    /// for a specific "rover" (GNSS receiver). This is usually dedicated
    /// to static rovers (reference station). The definition
    /// of reference stations does not impact this process, which has the
    /// sames requirements as [Algorithms::PppSolutions]
    CggttsSolutions,

    #[cfg(feature = "navigation")]
    /// Requires to solve navigation solutions for each individual rovers
    /// or station, regardless of their class & definition.
    AllPppSolutions,

    /// Navigation Solutions using RTK technique
    /// for a specific rover. This requires the definition of this rover
    /// and at least one reference station (two independent receivers).
    /// When several base stations are referenced, the tool will automatically
    /// take advantage of that.
    #[cfg(feature = "navigation")]
    RtkSolutions,
    
    #[cfg(feature = "navigation")]
    /// Requires to solve navigation solutions for each individual rovers.
    AllRtkSolutions,

}
