//! Qc processor
use crate::pipeline::QcPipelineSetupError;

#[cfg(feature = "nav")]
use anise::prelude::Almanac;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QcAlgorithm {
    // [QcAlgorithm::PPP] is post-processed
    /// navigation without ground reference and high precision products.
    /// [QcAlgorithm::PrecisePointPositioning] applies to a single rover.
    /// You can use [QcAlgorithm::PPP] to calibrate your RTK reference station(s)
    /// which can then be used in [QcAlgorithm::RTK] navigation.
    ///
    /// Requirements:
    /// - one rover definition (user defined)
    /// - synchronous signal observations
    /// - synchronous ephemeris or state source
    ///
    /// Optional:
    /// - calibration parameters
    ///
    /// Warning:
    /// - it is not possible to combine PPP to RTK to simplify internal logic.
    #[cfg(feature = "nav")]
    PPP,

    /// [QcAlgorithm::RTK] is post-processed or real-time navigation
    /// using at least one rover and at least one static ground reference.
    /// This will solve (_only_) the spatial state the preselected rover.
    /// If the baseline (distance from rover to nearest ground reference) goes above threshold,
    /// the solutions are rejected.
    /// Applies to the preselected rover.
    ///
    /// Requirements:
    /// - one rover definition (user defined)
    /// - one calibrated base definition (user defined)
    /// - synchronous signal observations
    /// - synchronous ephemeris or state source
    ///
    /// Optional:
    /// - calibration parameters
    ///
    /// Warning:
    /// - it is not possible to combine PPP to RTK to simplify internal logic.
    #[cfg(feature = "nav")]
    RTK,

    /// [QcAlgorithm::SignalProj] combines synchronous kepler (satellite position solver)
    /// and signal observations, for a compelling projection which is the input data
    /// to the navigation solver. This can be used to verify the signal quality and conditions
    /// and debug the navigation process.
    ///
    /// Requirements:
    /// - synchronous signal observations
    /// - synchronous ephemeris or state source
    #[cfg(feature = "nav")]
    SignalProj,

    /// [QcAlgorithm::TEC] will solve Total Electronic Content of the ionosphere
    /// using all dual frequency observations.
    ///
    /// Requirements:
    /// - synchronous signal observations
    /// - synchronous ephemeris or state source
    #[cfg(feature = "nav")]
    TEC,

    /// [QcAlgorithm::IPP] will solve the ionosphere pierce point coordinates in 3D space
    /// for each signal source in line of sight, from dual frequency observations.
    ///
    /// Requirements:
    /// - synchronous signal observations
    /// - synchronous ephemeris or state source
    #[cfg(feature = "nav")]
    IPP,

    /// [QcAlgorithm::StateResiduals] synchronizes and combines all satellite position
    /// sources, by possibly deploying a keplerian solver when needed,
    /// and cross compare each indivual source. This will stream out the coordinate
    /// residuals (position errors).
    #[cfg(feature = "nav")]
    StateResiduals,

    /// [QcAlgorithm::Cggtts] will process the CGGTTS track for the preselected receiver (static
    /// rover)
    #[cfg(feature = "nav")]
    CGGTTS,
}

impl QcAlgorithm {
    /// Returns true if this [QcAlgorithm] requires initialization of an [Almanac]
    fn needs_almanac(&self) -> bool {
        match self {
            Self::PPP
            | Self::RTK
            | Self::CGGTTS
            | Self::StateResiduals
            | Self::TEC
            | Self::IPP
            | Self::SignalProj => true,
            _ => false,
        }
    }
}

pub struct QcProcessor {
    operations: Vec<QcAlgorithm>,

    #[cfg(feature = "nav")]
    almanac: Almanac,
}

impl QcProcessor {
    /// Warning:
    /// - it is not possible to combine PPP to RTK to simplify internal logic.
    pub fn new(operations: Vec<QcAlgorithm>) -> Result<Self, QcPipelineSetupError> {
        Self::validate_operations(&operations);
        Self { operations }
    }

    /// Validates list of operations
    fn validate_operations(operations: &Vec<QcAlgorithm>) -> Result<(), QcPipelineSetupError> {
        #[cfg(feature = "nav")]
        {
            if operations.contains(&QcAlgorithm::PPP) {
                if operations.contains(&QcAlgorithm::RTK) {
                    return Err(QcSetupError::IncompatibleOperations);
                }
            }
        }
        Ok(())
    }
}

// impl std::str::FromStr for QcAlgorithm {
//     type Err = QcAlgorithmError;
//
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         match s.lower() {
//             "ppp" => Ok(Self::PrecisePointPositioning),
//             "rtk" => Ok(Self::RealTimeKinematics),
//             "sig" | "signal" => Ok(Self::SignalAnalysis),
//             "tec" => Ok(Self::TEC),
//             "ipp" => Ok(Self::IPP),
//             "cggtts" => Ok(Self::CGGTTS),
//             "kepler" => Ok(Self::KeplerSolver),
//         }
//     }
// }
//
// pub struct QcContext {
//     /// Possible antenna compensation database,
//     /// from which we may pull compensation parameters at required time
//     /// for ultra precise applications.
//     antenna_parameters: HashMap<Indexing, AntennaParameters>,
// }
//
// pub struct QcProcessor<TS: TimestampedStream> {
//     /// Abstract input [TimestampedStream]s where each
//     /// item is sampled.
//     temporal_streams: Vec<TS>,
//
//     /// Current epoch being processed
//     current_epoch: Option<Epoch>,
//
//     /// Algorithms
//     algorithms: Vec<QcAlgorithm>,
//
//     /// Static user context that will not evolve during the entire processing
//     context: QcContext,
// }
//
// impl <TS: TimestampedStream> QcProcessor<TS> {
//
//     /// Creates a new [QcProcessor] dedicated to [QcAlgorithm::PrecisePointPositioning]
//     pub fn ppp(rover: QcIndexing, temporal_streams: Vec<TS>) {
//         Self {
//             temporal_streams,
//             algorithms: vec![QcAlgorithm::PrecisePointPositioning],
//         }
//     }
// }
