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
pub mod orbit;

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
            QcClockResidual, QcCombination, QcCombinationKind, QcEphemerisCoverage,
            QcEphemerisOutput, QcFnSink, QcMeteoOutput, QcMultipathArc, QcObservationResidual,
            QcOrbitResidual, QcOutput, QcOutputItem, QcOutputQueue, QcPreciseClockOutput,
            QcSignalOutput, QcSink, QcTecMapPoint, QcTecMapStatistics,
        },
        rinex::{
            QcRinexClockSource, QcRinexMeteoSource, QcRinexNavSource, QcRinexObsSource,
            QcRinexSourceError,
        },
        source::{
            QcChainSource, QcChannelSource, QcClockId, QcEphemeris, QcInputItem, QcIterSource,
            QcLazySource, QcMeteoSample, QcPayload, QcPoll, QcPreciseClock, QcPreciseState,
            QcSignal, QcSource, QcSourceDescriptor, QcSourceError, QcTecPoint,
        },
    };
    pub use crate::processors::{
        QcClockAlignment, QcClockProduct, QcClockResidualConfig, QcClockResidualProcessor,
        QcCombinationConfig, QcCombinationProcessor, QcEphemerisCoverageProcessor,
        QcEphemerisProcessor, QcMeteoConfig, QcMeteoProcessor, QcObservationResidualConfig,
        QcObservationResidualProcessor, QcOrbitProduct, QcOrbitResidualConfig,
        QcOrbitResidualProcessor, QcPreciseClockConfig, QcPreciseClockProcessor, QcSignalProcessor,
        QcTecConfig, QcTecProcessor, QcTecRegion, QcTecSampling,
    };
    pub use crate::sync::{
        QcBuffered, QcClockStore, QcEphemerisStore, QcInterpolatedState, QcMeteoStore,
        QcPreciseStore, QcSvBuffer, QcTecInterpolation, QcTecMap, QcTecStore, QcTecValue,
    };

    #[cfg(feature = "sp3")]
    pub use crate::io::sp3::QcSp3Source;

    #[cfg(feature = "ionex")]
    pub use crate::io::ionex::QcIonexSource;

    #[cfg(feature = "rtcm")]
    pub use crate::io::rtcm::{QcRtcmDecoder, QcRtcmSource, QcRtcmStats};

    #[cfg(feature = "ntrip")]
    pub use crate::io::ntrip::{QcNtripConfig, QcNtripError, QcNtripRole, QcNtripSource};

    #[cfg(feature = "navigation")]
    pub use crate::{
        io::output::{QcNavigationReport, QcPvtSolution},
        navigation::{
            Almanac, ClockProfile, MetaAlmanac, Method, PVTSolution, PVTSolutionType,
            QcNavigationConfig, QcNavigationError, QcNavigationProcessor, QcNavigationTechnique,
            QcSolverConfig, UserParameters, UserProfile,
        },
    };

    #[cfg(feature = "ionex")]
    pub use crate::{
        io::output::QcIppOutput,
        processors::{QcIppConfig, QcIppProcessor},
    };

    pub use crate::orbit::{QcClockKind, QcClockPreference, QcOrbitKind, QcOrbitPreference};

    #[cfg(feature = "cggtts")]
    pub use crate::{
        io::output::{QcCggttsReport, QcCggttsTrack},
        navigation::{QcCggttsConfig, QcCggttsError, QcCggttsProcessor},
    };

    #[cfg(feature = "cggtts")]
    pub use cggtts::prelude::{
        CommonViewCalendar, CommonViewClass, CommonViewPeriod, Track as CggttsTrack, CGGTTS,
    };

    // re-export
    pub use gnss_rs::prelude::{Constellation, SV};
    pub use hifitime::prelude::{Duration, Epoch, TimeScale};
}
