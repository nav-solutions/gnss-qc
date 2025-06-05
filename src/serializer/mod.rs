pub mod data;
pub mod iter;
pub mod signal;
pub mod state;

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
pub mod ephemeris;

#[cfg(feature = "sp3")]
#[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
pub mod sp3;

use crate::{ context::QcContext};

use data::{QcSerializedItem, QcSerializedRINEXHeader};
use ephemeris::QcEphemerisIterator;
use signal::QcSignalIterator;
use state::State;

#[cfg(feature = "sp3")]
use crate::serializer::data::QcSerializedSP3Header;

#[cfg(feature = "sp3")]
use crate::serializer::sp3::QcPreciseStateIterator;

/// [QcSerializer] to serialize entire [QcContext].
pub struct QcSerializer<'a> {
    /// Reference to [QcContext] being iterated
    ctx: &'a QcContext,

    /// Current [State] of the [QcSerializer]
    state: State,

    /// Total number of RINEX files
    total_rinex_files: usize,

    /// RINEX headers we have streamed
    rinex_headers: Vec<String>,

    /// Total number of SP3 files
    total_sp3_files: usize,

    /// SP3 headers we have streamed
    sp3_headers: Vec<String>,

    /// All [QcEphemerisIterator] sources
    ephemeris_sources: Vec<QcEphemerisIterator<'a>>,

    /// All [QcPreciseStateIterator] sources
    precise_state_sources: Vec<QcPreciseStateIterator<'a>>,

    /// All [QcSignalIterator] sources
    signal_sources: Vec<QcSignalIterator<'a>>,
}

impl QcContext {
    /// Obtain a synchronous [QcSerializer] from current [QcContext], ready to serialize the entire context.
    pub fn serializer<'a>(&'a self) -> QcSerializer<'a> {
        let mut signal_sources = Vec::new();
        let mut ephemeris_sources = Vec::new();

        #[cfg(feature = "sp3")]
        let mut precise_state_sources = Vec::new();

        for (desc, _) in self.data.iter() {
            if let Some(serializer) = self.ephemeris_serializer(desc.indexing.clone()) {
                ephemeris_sources.push(serializer);
            } else if let Some(serializer) = self.signal_serializer(desc.indexing.clone()) {
                signal_sources.push(serializer);
            }

            #[cfg(feature = "sp3")]
            if let Some(serializer) = self.precise_states_serializer(desc.indexing.clone()) {
                precise_state_sources.push(serializer);
            }
        }

        let total_rinex_files = self.total_rinex_files();

        #[cfg(not(feature = "sp3"))]
        let total_sp3_files = 0;

        #[cfg(feature = "sp3")]
        let total_sp3_files = self.total_sp3_files();

        debug!("total rinex files: {}", total_rinex_files);
        debug!("total sp3 files: {}", total_sp3_files);

        QcSerializer {
            ctx: self,
            signal_sources,
            ephemeris_sources,
            precise_state_sources,
            total_sp3_files,
            total_rinex_files,
            state: Default::default(),
            sp3_headers: Default::default(),
            rinex_headers: Default::default(),
        }
    }
}

impl<'a> Iterator for QcSerializer<'a> {
    type Item = QcSerializedItem;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // debug!("state: {:?}", self.state);

            match self.state {
                State::Constants => {
                    self.state = State::RINEXHeaders;
                }

                State::RINEXHeaders => {
                    for (desc, data) in self.ctx.data.iter() {
                        if let Some(rinex) = data.as_rinex() {
                            if !self.rinex_headers.contains(&desc.filename) {
                                debug!(
                                    "streaming header ={} {}/{}",
                                    desc.filename,
                                    self.rinex_headers.len() + 1,
                                    self.total_rinex_files
                                );

                                self.rinex_headers.push(desc.filename.clone());

                                let serialized = QcSerializedRINEXHeader {
                                    data: rinex.header.clone(),
                                    indexing: desc.indexing.clone(),
                                    product_type: desc.product_type,
                                    filename: desc.filename.clone(),
                                };

                                return Some(QcSerializedItem::RINEXHeader(serialized));
                            }
                        }
                    }

                    if self.rinex_headers.len() == self.total_rinex_files {
                        self.state = State::SP3Header;
                    }
                }

                #[cfg(not(feature = "sp3"))]
                State::SP3Header => {
                    self.state = State::Ephemeris;
                }

                #[cfg(feature = "sp3")]
                State::SP3Header => {
                    for (desc, data) in self.ctx.data.iter() {
                        if let Some(sp3) = data.as_sp3() {
                            if !self.sp3_headers.contains(&desc.filename) {
                                debug!(
                                    "streaming header ={} {}/{}",
                                    desc.filename,
                                    self.sp3_headers.len() + 1,
                                    self.total_sp3_files,
                                );

                                self.sp3_headers.push(desc.filename.clone());

                                let serialized = QcSerializedSP3Header {
                                    data: sp3.header.clone(),
                                    indexing: desc.indexing.clone(),
                                    product_type: desc.product_type,
                                    filename: desc.filename.clone(),
                                };

                                return Some(QcSerializedItem::SP3Header(serialized));
                            }
                        }
                    }

                    if self.sp3_headers.len() == self.total_sp3_files {
                        self.state = State::Ephemeris;
                    }
                }

                #[cfg(not(feature = "navigation"))]
                State::Ephemeris => {
                    self.state = State::Meteo;
                }

                #[cfg(feature = "navigation")]
                State::Ephemeris => {
                    // round robin all data sources (until all completed)
                    for source in self.ephemeris_sources.iter_mut() {
                        if !source.iter.eos {
                            if let Some(data) = source.next() {
                                return Some(QcSerializedItem::Ephemeris(data));
                            }
                        }
                    }

                    self.state = State::Meteo;
                }

                State::Meteo => {
                    self.state = State::PreciseOrbit;
                }

                #[cfg(feature = "sp3")]
                State::PreciseOrbit => {
                    // round robin all data sources (until all completed)
                    for source in self.precise_state_sources.iter_mut() {
                        if !source.iter.eos {
                            if let Some(data) = source.next() {
                                return Some(QcSerializedItem::PreciseState(data));
                            }
                        }
                    }

                    self.state = State::PreciseClock;
                }

                #[cfg(not(feature = "sp3"))]
                State::PreciseOrbit => {
                    self.state = State::PreciseClock;
                }

                State::PreciseClock => {
                    self.state = State::Signal;
                }

                State::Signal => {
                    // round robin all data sources (until all completed)
                    for source in self.signal_sources.iter_mut() {
                        if !source.iter.eos {
                            if let Some(data) = source.next() {
                                return Some(QcSerializedItem::Signal(data));
                            }
                        }
                    }

                    self.state = State::Done;
                }

                State::Done => {
                    return None;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{prelude::QcContext, tests::init_logger};

    #[test]
    fn ephemeris_only_context_serializer() {
        init_logger();

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
    fn dual_ephemeris_context_serializer() {
        init_logger();

        let mut ctx = QcContext::new();

        // load NAV
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        // load NAV
        ctx.load_gzip_rinex_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz")
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
        init_logger();
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

    #[test]
    fn dual_signal_sources_serializer() {
        init_logger();
        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();
        ctx.load_rinex_file("data/OBS/V3/VLNS0630.22O").unwrap();

        let mut serializer = ctx.serializer();

        let mut points = 0;

        while let Some(_) = serializer.next() {
            points += 1;
        }

        assert!(points > 0, "did not propose any ephemeris data points!");
    }

    #[test]
    fn dual_signal_dual_eph_serializer() {
        init_logger();
        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();
        ctx.load_rinex_file("data/OBS/V3/VLNS0630.22O").unwrap();

        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        ctx.load_gzip_rinex_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        let mut serializer = ctx.serializer();

        let mut points = 0;

        while let Some(_) = serializer.next() {
            points += 1;
        }

        assert!(points > 0, "did not propose any ephemeris data points!");
    }
}
