//! User guide.
//!
//! These pages are the `docs/` folder of the repository: every code example
//! is compiled by the test suite.
//!
//! - [context]: context definition, input products and indexing
//! - [miso]: the MISO engine, sources, synchronization, outputs, real-time operation
//! - [processors]: built-in processors, writing your own
//! - `navigation`: P.V.T solutions (`navigation` feature), CGGTTS (`cggtts` feature)
//! - `ipp`: Ionospheric Pierce Points (`ionex` feature)
//! - `residuals`: observation, orbit and clock residuals (A − B)
//! - `ionex`: TEC maps (IONEX, `ionex` feature)
//! - `clocks`: precise clocks (Clock RINEX), satellite clocks of the navigation
//! - [meteo]: meteo observations (Meteo RINEX)
//! - [testing]: test data and running the tests

#[doc = include_str!("../docs/context.md")]
pub mod context {}

#[doc = include_str!("../docs/miso.md")]
pub mod miso {}

#[doc = include_str!("../docs/processors.md")]
pub mod processors {}

#[doc = include_str!("../docs/testing.md")]
pub mod testing {}

#[cfg(all(feature = "ionex", feature = "sp3"))]
#[doc = include_str!("../docs/ipp.md")]
pub mod ipp {}

#[cfg(feature = "navigation")]
#[doc = include_str!("../docs/clocks.md")]
pub mod clocks {}

#[doc = include_str!("../docs/meteo.md")]
pub mod meteo {}

#[cfg(feature = "ionex")]
#[doc = include_str!("../docs/ionex.md")]
pub mod ionex {}

#[cfg(feature = "sp3")]
#[doc = include_str!("../docs/residuals/README.md")]
pub mod residuals {
    #[doc = include_str!("../docs/residuals/observations.md")]
    pub mod observations {}

    #[doc = include_str!("../docs/residuals/orbits.md")]
    pub mod orbits {}

    #[doc = include_str!("../docs/residuals/clocks.md")]
    pub mod clocks {}
}

#[cfg(all(feature = "navigation", feature = "sp3"))]
#[doc = include_str!("../docs/navigation/README.md")]
pub mod navigation {
    #[doc = include_str!("../docs/navigation/spp.md")]
    pub mod spp {}

    #[doc = include_str!("../docs/navigation/cpp.md")]
    pub mod cpp {}

    #[doc = include_str!("../docs/navigation/ppp.md")]
    pub mod ppp {}

    #[doc = include_str!("../docs/navigation/rtk.md")]
    pub mod rtk {}

    #[cfg(feature = "cggtts")]
    #[doc = include_str!("../docs/navigation/cggtts.md")]
    pub mod cggtts {}
}
