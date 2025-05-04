use crate::{
    context::QcContext,
    prelude::Epoch,
    serializer::{
        ephemeris::{QcEphemerisData, QcEphemerisSerializer},
        signal::QcSignalDataPoint,
    },
};

#[derive(Debug, Copy, Clone)]
pub enum State {
    /// Observations, per source
    Observations(usize),

    #[cfg(feature = "navigation")]
    Ephemeris,
}

#[derive(Debug, Clone)]
pub enum QcSerializedDataPoint {
    /// [QcSignalObservation]
    SignalObservation(QcSignalDataPoint),

    /// [QcEphemerisData]
    EphemerisData(QcEphemerisData),
}

// Synchronous [QcContext] Iterator
pub struct QcSerializer<'a> {
    /// Current [State] of the [QcSerializer]
    state: State,

    /// True when completely done
    eos: bool,

    /// Latest [Epoch] serialized by [QcSerializer]
    epoch: Option<Epoch>,

    /// Number of Observations source to serialize independently
    num_obs_sources: usize,

    #[cfg(feature = "navigation")]
    ephemeris_ser: QcEphemerisSerializer<'a>,
}

impl QcContext {
    /// Obtain [QcSerializer] from current [QcContext], ready to serialize the entire context.
    pub fn serializer<'a>(&'a self) -> QcSerializer<'a> {
        let num_obs_sources = self.observations.keys().count();

        #[cfg(feature = "navigation")]
        let initial_state = if num_obs_sources == 0 {
            State::Observations(0)
        } else {
            State::Ephemeris
        };

        #[cfg(not(feature = "navigation"))]
        let initial_state = State::Observations(0);

        QcSerializer {
            eos: false,
            epoch: None,
            num_obs_sources,
            state: initial_state,
            ephemeris_ser: self.ephemeris_serializer(),
        }
    }
}

impl<'a> Iterator for QcSerializer<'a> {
    type Item = QcSerializedDataPoint;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eos {
            // completely done
            return None;
        }

        None
    }
}
