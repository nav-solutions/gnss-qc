use hifitime::prelude::Epoch;
use crate::input::TimestampedStream;

pub enum QcAlgorithm {
    /// [QcAlgorithm::PrecisePointPositioning] is post-processed 
    /// navigation without ground reference and high precision products.
    /// [QcAlgorithm::PrecisePointPositioning] applies to a single rover.
    #[cfg(feature = "nav")]
    PrecisePointPositioning,

    /// [QcAlgorithm::RealTimeKinematics] is post-processed or real-time navigation
    /// using at least one rover and at least one static ground reference.
    /// This will solve (_only_) the spatial state the preselected rover.
    /// If the baseline (distance from rover to nearest ground reference) goes above threshold,
    /// the solutions are rejected.
    /// Applies to the preselected rover.
    #[cfg(feature = "nav")]
    RealTimeKinematics,

    /// [QcAlgorithm::SignalObservations] will serialize all input signal observations
    /// into individual tokens that you can further analyze. One signal observation must exist
    /// for an item to be streamed.
    SignalObservations,

    /// [QcAlgorithm::ClockObservations] will serialize all input clock (either ground or spaceborn)
    /// information from each indiviaul data source into indivial tokens that you can further
    /// analyze. One clock must exist for an item to be streamed.
    ClockObservations,

    /// [QcAlgorithm::KeplerSolver] will serialize all ephemeris message and resolve
    /// satellite orbital state of each individual message source.
    #[cfg(feature = "nav")]
    KeplerSolver,

    /// [QcAlgorithm::Cggtts] will process the CGGTTS track for the preselected receiver (static
    /// rover)
    #[cfg(feature = "nav")]
    CGGTTS,

    /// [QcAlgorithm::TEC] will solve Total Electronic Content of the ionosphere
    /// using all dual frequency observations.
    TEC,
    
    /// [QcAlgorithm::IPP] will solve the ionosphere pierce point coordinates in 3D space
    /// for each signal source in line of sight, from dual frequency observations.
    IPP,
}

impl std::str::FromStr for QcAlgorithm {
    type Err = QcAlgorithmError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.lower() {
            "ppp" => Ok(Self::PrecisePointPositioning),
            "rtk" => Ok(Self::RealTimeKinematics),
            "sig" | "signal" => Ok(Self::SignalAnalysis),
            "tec" => Ok(Self::TEC),
            "ipp" => Ok(Self::IPP),
            "cggtts" => Ok(Self::CGGTTS),
            "kepler" => Ok(Self::KeplerSolver),
        }
    }
}

pub struct QcContext {
    /// Possible antenna compensation database,
    /// from which we may pull compensation parameters at required time
    /// for ultra precise applications.
    antenna_parameters: HashMap<Indexing, AntennaParameters>,
}

pub struct QcProcessor<TS: TimestampedStream> {
    /// Abstract input [TimestampedStream]s where each
    /// item is sampled.
    temporal_streams: Vec<TS>,

    /// Current epoch being processed
    current_epoch: Option<Epoch>,

    /// Algorithms
    algorithms: Vec<QcAlgorithm>,

    /// Static user context that will not evolve during the entire processing
    context: QcContext,
}

impl <TS: TimestampedStream> QcProcessor<TS> {

    /// Creates a new [QcProcessor] dedicated to [QcAlgorithm::PrecisePointPositioning]
    pub fn ppp(rover: QcIndexing, temporal_streams: Vec<TS>) {
        Self {
            temporal_streams,
            algorithms: vec![QcAlgorithm::PrecisePointPositioning],
        }
    }
}
