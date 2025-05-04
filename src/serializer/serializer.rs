use crate::{
    context::QcContext,
    serializer::{
        ephemeris::{QcEphemerisData, QcEphemerisSerializer},
        signal::QcSignalDataPoint,
    },
};

use super::signal::QcSignalSerializer;

#[derive(Debug, Copy, Clone)]
pub enum State {
    /// Pulling Observations
    Observations(usize),

    #[cfg(feature = "navigation")]
    /// Pulling Ephemeris data
    Ephemeris,

    /// Consumed all data
    Done,
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

    /// Number of Observations source to serialize independently
    num_obs_sources: usize,

    signal_sources_done: Vec<bool>,
    signal_sources_ser: Vec<QcSignalSerializer<'a>>,

    #[cfg(feature = "navigation")]
    ephemeris_done: bool,

    #[cfg(feature = "navigation")]
    ephemeris_ser: QcEphemerisSerializer<'a>,
}

impl QcContext {
    /// Obtain [QcSerializer] from current [QcContext], ready to serialize the entire context.
    pub fn serializer<'a>(&'a self) -> QcSerializer<'a> {
        let num_obs_sources = self.observations.keys().count();

        #[cfg(feature = "navigation")]
        let initial_state = if num_obs_sources == 0 {
            State::Ephemeris
        } else {
            State::Observations(0)
        };

        #[cfg(not(feature = "navigation"))]
        let initial_state = if num_obs_sources == 0 {
            State::Done
        } else {
            State::Observations(0)
        };

        #[cfg(feature = "navigation")]
        let ephemeris_ser = self.ephemeris_serializer();

        let mut signal_sources_done = Vec::new();
        let mut signal_sources_ser = Vec::new();

        for source in self.observations.keys() {
            signal_sources_done.push(false);
            signal_sources_ser.push(self.signal_serializer(&source));
        }

        QcSerializer {
            state: initial_state,
            num_obs_sources: signal_sources_ser.len(),
            signal_sources_ser,
            signal_sources_done,
            #[cfg(feature = "navigation")]
            ephemeris_done: false,
            #[cfg(feature = "navigation")]
            ephemeris_ser,
        }
    }
}

impl<'a> QcSerializer<'a> {
    #[cfg(feature = "navigation")]
    fn next_state(&self) -> State {
        match self.state {
            State::Observations(index) => {
                if index < self.num_obs_sources - 1 {
                    if !self.signal_sources_done[index + 1] {
                        State::Observations(index + 1)
                    } else {
                        State::Ephemeris
                    }
                } else {
                    if self.ephemeris_done {
                        State::Done
                    } else {
                        State::Ephemeris
                    }
                }
            }
            State::Ephemeris => {
                if self.num_obs_sources > 0 {
                    if self.all_signals_consumed() {
                        if self.ephemeris_done {
                            State::Done
                        } else {
                            State::Ephemeris
                        }
                    } else {
                        State::Observations(self.next_signal_source())
                    }
                } else {
                    State::Done
                }
            }
            State::Done => State::Done,
        }
    }

    #[cfg(not(feature = "navigation"))]
    fn next_state(&self) -> State {
        match self.state {
            State::Observations(index) => {
                if index < self.num_obs_sources - 1 {
                    State::Observations(index + 1)
                } else {
                    State::Observations(0)
                }
            }
        }
    }

    /// True when all data sources have been consumed
    fn all_signals_consumed(&self) -> bool {
        for done in self.signal_sources_done.iter() {
            if !done {
                return false;
            }
        }

        true
    }

    /// Returns index of next signal source to read (for transition efficiency)
    fn next_signal_source(&self) -> usize {
        let mut index = 0;

        for done in self.signal_sources_done.iter() {
            if !done {
                return index;
            }
            index += 1;
        }
        index
    }
}

impl<'a> Iterator for QcSerializer<'a> {
    type Item = QcSerializedDataPoint;

    fn next(&mut self) -> Option<Self::Item> {
        // Try to pull new symbol
        let mut ret = Option::<QcSerializedDataPoint>::None;

        loop {
            if ret.is_some() {
                return ret;
            }

            match self.state {
                State::Observations(index) => {
                    // try to pull data
                    if let Some(data) = self.signal_sources_ser[index].next() {
                        ret = Some(QcSerializedDataPoint::SignalObservation(data));
                    } else {
                        self.signal_sources_done[index] = true;
                    }
                }

                #[cfg(feature = "navigation")]
                State::Ephemeris => {
                    // try to pull data
                    if let Some(data) = self.ephemeris_ser.next() {
                        ret = Some(QcSerializedDataPoint::EphemerisData(data));
                    } else {
                        self.ephemeris_done = true;
                    }
                }

                State::Done => {
                    return None;
                }
            }

            self.state = self.next_state();
        }
    }
}

#[cfg(test)]
mod test {

    use crate::{prelude::QcContext, serializer::serializer::QcSerializer};

    #[test]
    fn ephemeris_context_serializer() {
        let mut ctx = QcContext::new();

        // load NAV
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        let mut serializer = ctx.serializer();

        let mut points = 0;

        while let Some(_) = serializer.next() {
            points += 1;
        }

        assert!(points > 0, "did not propose any ephemeris data points!");
    }

    #[test]
    fn signal_sources_serializer() {
        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();

        let mut serializer = ctx.serializer();

        let mut points = 0;

        while let Some(_) = serializer.next() {
            points += 1;
        }

        assert!(points > 0, "did not propose any ephemeris data points!");
    }
}
