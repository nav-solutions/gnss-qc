//! Input Streams definition
use thiserror::Error;

pub mod signal;

use signal::SignalStream;

#[derive(Debug, Error)]
pub enum InputStreamError {
    
}

/// [StreamToken]s are only temporal data points that we can manage.
/// The data source is expected to push them in chronological order.
pub enum StreamToken {
    Observation(SignalToken),
    Ephemeris(EphemerisToken)
}

pub trait Stream {
    type Error;

    /// Attempts to pull a new [StreamToken] from the [Stream],
    /// returning None on the end of stream.
    fn next(&mut self) -> Result<Option<StreamToken>, Self::Error>;
}

pub trait TimestampedStream {
    type Error;

    /// Attempts to pull a new [StreamToken] from the [Stream],
    /// returning None on the end of stream.
    /// The data source is expected to push them in chronological order.
    /// If several data points exist for the same epoch, that is fine.
    /// But the data source should not push tokens that were sampled _before_ the latest
    /// item provided in time. Otherwise, the following algorithms will given inevitably bad
    /// results.
    fn next(&mut self) -> Result<Option<TemporalStreamToken>, Self::Error>;
}

/// [InputStream] definition, feeding data points to a possible pipeline
pub enum InputStream {
    /// [SignalStream]
    Observation(SignalStream),

    /// [StateStream]s
    Ephemeris(EphemerisStream),
}


impl Stream for InputStream {
    type Error = InputStreamError;

    fn next(&mut self) -> Result<Option<StreamToken>, Self::Error> {
        match self {
            Self::Ephemeris(eph) => eph.next(),
            Self::Observation(obs) => obs.next(),
        } 
    }
}

impl TemporalStream for InputTimestampedStream {

}
