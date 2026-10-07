//! Input Streams definition
use thiserror::Error;

// pub mod signal;
// pub mod meteo;
//
// #[cfg(feature = "rtcm")]
// use rtcm_rs::msg::message::Message as RtcmMessage;

// mod preprocessing;

// use signal::SignalStream;

#[derive(Debug, Error)]
pub enum InputStreamError {}

pub enum QcStreamItem {
    // /// [SignalObservation]
    // Observation(SignalObservation),

    // /// [MeteoObservation] for precise atmospheric modeling
    // MeteoObservation(MeteoObservation),

    // /// [EphemerisMessage]
    // Ephemeris(EphemerisMessage),
    /// Resolved [Orbital] state
    #[cfg(feature = "nav")]
    OrbitalState(Orbit),
}

pub trait QcStream {
    type Error;

    /// Attempts to pull a new [StreamItem] from this [Stream],
    /// returning None on the end of stream.
    /// For temporal data points, the data source is expected to push items in chronological order,
    /// otherwhise this framework will return an error and will not process, avoiding
    /// generating invalid results.
    fn next(&mut self) -> Result<Option<QcStreamItem>, Self::Error>;
}

// impl Stream for InputStream {
//     type Error = InputStreamError;
//
//     fn next(&mut self) -> Result<Option<StreamToken>, Self::Error> {
//         match self {
//             Self::Ephemeris(eph) => eph.next(),
//             Self::Observation(obs) => obs.next(),
//         }
//     }
// }
