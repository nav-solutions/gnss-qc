#![doc(
    html_logo_url = "https://raw.githubusercontent.com/nav-solutions/.github/master/logos/logo2.jpg"
)]
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

/*
 * GNSS-Qc is part of the NAV-Solutions framework.
 * Authors: Guillaume W. Bres <guillaume.bressaix@gmail.com> et al.
 * (cf. https://github.com/nav-solutions/gnss-qc/graphs/contributors)
 * This framework is licensed under Mozilla Public V2 license.
 *
 * Documentation:
 * - https://github.com/nav-solutions/gnss-qc
 * - https://github.com/nav-solutions/rinex
 * - https://github.com/nav-solutions/sp3
 * - https://github.com/nav-solutions/cggtts
 */

extern crate gnss_qc_traits as qc_traits;
extern crate gnss_rs as gnss;

pub mod cfg;
pub mod engine;
pub mod geodesy;

#[cfg(all(feature = "sp3", feature = "flate2"))]
pub mod guide;
pub mod io;

#[cfg(feature = "navigation")]
pub mod navigation;

pub mod pipeline;
pub mod processors;
pub mod sync;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::engine::{
        QcEngine, QcEngineBuilder, QcEngineError, QcEngineStats, QcEpochContext, QcEpochSignals,
        QcProcessor, QcRunError, QcStatics,
    };
    pub use crate::io::{
        input::{
            cfg::{QcIndexingConfig, QcInputConfig},
            indexing::QcIndexing,
            product::{QcInputProducts, QcProductSources},
        },
        output::{
            QcEphemerisCoverage, QcEphemerisOutput, QcFnSink, QcOutput, QcOutputItem,
            QcOutputQueue, QcSignalOutput, QcSink,
        },
        rinex::{QcRinexNavSource, QcRinexObsSource, QcRinexSourceError},
        source::{
            QcChainSource, QcChannelSource, QcEphemeris, QcInputItem, QcIterSource, QcLazySource,
            QcPayload, QcPoll, QcPreciseState, QcSignal, QcSource, QcSourceDescriptor,
            QcSourceError,
        },
    };
    pub use crate::processors::{
        QcEphemerisCoverageProcessor, QcEphemerisProcessor, QcSignalProcessor,
    };
    pub use crate::sync::{
        QcBuffered, QcEphemerisStore, QcInterpolatedState, QcPreciseStore, QcSvBuffer,
    };

    #[cfg(feature = "sp3")]
    pub use crate::io::sp3::QcSp3Source;

    #[cfg(feature = "navigation")]
    pub use crate::{
        io::output::{QcNavigationReport, QcPvtSolution},
        navigation::{
            Almanac, ClockProfile, MetaAlmanac, Method, PVTSolution, PVTSolutionType,
            QcNavigationConfig, QcNavigationError, QcNavigationProcessor, QcNavigationTechnique,
            QcOrbitPreference, QcSolverConfig, UserParameters, UserProfile,
        },
    };

    // re-export
    pub use gnss_rs::prelude::{Constellation, SV};
    pub use hifitime::prelude::{Duration, Epoch, TimeScale};
}
