//! Input Streams definition
use thiserror::Error;

use crate::io::{
    input::QcInputItem,
    stream::{QcStream, QcSynchronousItem, QcSynchronousStream},
};

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
    // Observation(QcSignalObservation),

    // // /// [MeteoObservation] for precise atmospheric modeling
    // // MeteoObservation(MeteoObservation),

    // // /// [EphemerisMessage]
    // // Ephemeris(EphemerisMessage),
    // /// Resolved [Orbital] state
    // #[cfg(feature = "nav")]
    // OrbitalState(Orbit),
}

pub struct QcInputProductsStreamer {
    // pub obs_stream: QcInputObservationsStream,
}

impl QcStream for QcInputProductsStreamer {
    type Item = QcInputItem;

    type Error = InputStreamError;

    fn next(&mut self) -> Result<Option<QcStreamItem>, Self::Error> {
        Ok(None)
    }

    // fn next(&mut self) -> Result<Option<StreamToken>, Self::Error> {
    //     match self {
    //         Self::Ephemeris(eph) => eph.next(),
    //         Self::Observation(obs) => obs.next(),
    //     }
    // }
}
