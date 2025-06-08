use crate::{
    context::{QcContext, QcSourceDescriptor},
    prelude::{Duration, Epoch},
};

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
pub mod ephemeris;

#[cfg(feature = "sp3")]
#[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
pub mod sp3;

pub mod data;
pub mod iter;
pub mod signal;
pub mod state;

use data::{QcSerializedItem, QcSerializedRINEXHeader};
use ephemeris::QcEphemerisIterator;
use signal::QcSignalIterator;
use state::{State, TemporalState};

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

    /// Total time frame [Duration]
    total_duration: Duration,

    /// Current [Epoch]
    epoch: Option<Epoch>,

    /// Last [Epoch] to be streamed
    last_epoch: Option<Epoch>,

    descriptor_ptr: usize,
    num_rinex_descriptors: usize,
    num_sp3_descriptors: usize,
    first_temporal_iter: bool,

    /// Total descriptors to be serialized
    rinex_descriptors: Vec<&'a QcSourceDescriptor>,

    #[cfg(feature = "sp3")]
    sp3_descriptors: Vec<&'a QcSourceDescriptor>,

    /// All [QcEphemerisIterator] sources
    ephemeris_sources: Vec<QcEphemerisIterator<'a>>,

    /// All [QcSignalIterator] sources
    signal_sources: Vec<QcSignalIterator<'a>>,

    /// All [QcPreciseStateIterator] sources
    #[cfg(feature = "sp3")]
    precise_state_sources: Vec<QcPreciseStateIterator<'a>>,
}

impl QcContext {
    /// Obtain a synchronous [QcSerializer] from current [QcContext], ready to serialize the dataset.   
    /// You can then use the iterate the serializer.
    pub fn serializer<'a>(&'a self) -> QcSerializer<'a> {
        // Signal sources
        let mut signal_sources = Vec::new();

        // Ephemeris sources
        let mut ephemeris_sources = Vec::new();

        // Precise sources
        #[cfg(feature = "sp3")]
        let mut precise_state_sources = Vec::new();

        for (desc, _) in self.data.iter() {
            if let Some(serializer) = self.ephemeris_serializer(&desc.indexing) {
                ephemeris_sources.push(serializer);
                continue;
            } else if let Some(serializer) = self.signal_serializer(&desc.indexing) {
                signal_sources.push(serializer);
                continue;
            }

            #[cfg(feature = "sp3")]
            if let Some(serializer) = self.precise_states_serializer(&desc.indexing) {
                precise_state_sources.push(serializer);
            }
        }

        let total_duration = self.total_duration();
        let epoch = self.first_epoch();
        let last_epoch = self.last_epoch();

        let rinex_descriptors = self
            .data
            .keys()
            .filter(|k| k.product_type.is_rinex_product())
            .collect::<Vec<_>>();

        let num_rinex_descriptors = rinex_descriptors.len();

        #[cfg(feature = "sp3")]
        let sp3_descriptors = self
            .data
            .keys()
            .filter(|k| !k.product_type.is_rinex_product())
            .collect::<Vec<_>>();

        #[cfg(feature = "sp3")]
        let num_sp3_descriptors = sp3_descriptors.len();

        QcSerializer {
            ctx: self,
            signal_sources,
            total_duration,
            epoch,
            last_epoch,
            rinex_descriptors,
            ephemeris_sources,
            descriptor_ptr: 0,
            first_temporal_iter: true,

            num_rinex_descriptors,
            state: Default::default(),

            #[cfg(feature = "sp3")]
            num_sp3_descriptors,

            #[cfg(feature = "sp3")]
            sp3_descriptors,

            #[cfg(feature = "sp3")]
            precise_state_sources,
        }
    }
}

impl<'a> Iterator for QcSerializer<'a> {
    type Item = QcSerializedItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            debug!(
                "serializer: state: {:?} - epoch={} - dt={}",
                self.state,
                self.epoch.unwrap_or_default(),
                self.total_duration
            );

            match self.state {
                State::Constants => {
                    // not supported yet
                    self.state = State::RINEXHeaders;
                }
                State::RINEXHeaders => {
                    if let Some(item) = self.next_rinex_header() {
                        return Some(item);
                    } else {
                        self.descriptor_ptr = 0;
                        self.state = State::SP3Header;
                    }
                }
                State::SP3Header => {
                    if let Some(item) = self.next_sp3_header() {
                        return Some(item);
                    } else {
                        self.descriptor_ptr = 0;
                        self.state = State::Temporal(Default::default());
                    }
                }
                State::Temporal(TemporalState::Ephemeris) => {
                    if let Some(item) = self.next_ephemeris() {
                        return Some(item);
                    } else {
                        self.state = State::Temporal(TemporalState::PreciseOrbits);
                    }
                }
                State::Temporal(TemporalState::PreciseOrbits) => {
                    if let Some(item) = self.next_precise_state() {
                        return Some(item);
                    } else {
                        self.descriptor_ptr = 0;
                        self.state = State::Temporal(TemporalState::Observations);
                    }
                }
                State::Temporal(TemporalState::Observations) => {
                    self.state = State::Done;
                }
                State::Done => {
                    return None;
                }
            }
        }
    }
}

impl<'a> QcSerializer<'a> {
    fn next_rinex_header(&mut self) -> Option<QcSerializedItem<'a>> {
        if self.num_rinex_descriptors == 0 {
            return None;
        }

        if self.descriptor_ptr == self.num_rinex_descriptors {
            return None;
        }

        let descriptor = self.rinex_descriptors[self.descriptor_ptr];

        // retrieve data
        let data = self
            .ctx
            .data
            .iter()
            .filter_map(|(k, v)| {
                if k == descriptor {
                    if let Some(v) = v.as_rinex() {
                        Some(v)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .reduce(|k, _| k)?;

        // serialize
        let serialized = QcSerializedRINEXHeader {
            data: &data.header,
            indexing: &descriptor.indexing,
            product_type: descriptor.product_type,
            filename: &descriptor.filename,
        };

        self.descriptor_ptr += 1;

        // push
        debug!(
            "streaming {} header {}/{}",
            descriptor.filename, self.descriptor_ptr, self.num_rinex_descriptors,
        );

        return Some(QcSerializedItem::RINEXHeader(serialized));
    }

    #[cfg(feature = "sp3")]
    fn next_sp3_header(&mut self) -> Option<QcSerializedItem<'a>> {
        if self.num_sp3_descriptors == 0 {
            return None;
        }

        if self.descriptor_ptr == self.num_sp3_descriptors {
            return None;
        }

        let descriptor = self.sp3_descriptors[self.descriptor_ptr];

        // retrieve data
        let data = self
            .ctx
            .data
            .iter()
            .filter_map(|(k, v)| {
                if k == descriptor {
                    if let Some(v) = v.as_sp3() {
                        Some(v)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .reduce(|k, _| k)?;

        // serialize
        let serialized = QcSerializedSP3Header {
            data: data.header.clone(),
            indexing: &descriptor.indexing,
            product_type: descriptor.product_type,
            filename: &descriptor.filename,
        };

        self.descriptor_ptr += 1;

        // push
        debug!(
            "streaming {} header {}/{}",
            descriptor.filename, self.descriptor_ptr, self.num_sp3_descriptors,
        );

        return Some(QcSerializedItem::SP3Header(serialized));
    }

    pub fn next_ephemeris(&mut self) -> Option<QcSerializedItem<'a>> {
        if self.num_rinex_descriptors == 0 {
            return None;
        }

        if self.descriptor_ptr == self.num_rinex_descriptors {
            return None;
        }

        // retrieve
        let streamer = &mut self.ephemeris_sources[self.descriptor_ptr];

        if let Some(data) = streamer.next() {
            // push
            Some(QcSerializedItem::Ephemeris(data))
        } else {
            // end of stream
            self.descriptor_ptr += 1;
            None
        }
    }

    pub fn next_precise_state(&mut self) -> Option<QcSerializedItem<'a>> {
        if self.num_sp3_descriptors == 0 {
            return None;
        }

        if self.descriptor_ptr == self.num_sp3_descriptors {
            return None;
        }

        // retrieve
        let streamer = &mut self.precise_state_sources[self.descriptor_ptr];

        if let Some(data) = streamer.next() {
            // push
            Some(QcSerializedItem::PreciseState(data))
        } else {
            // end of stream
            self.descriptor_ptr += 1;
            None
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{prelude::QcContext, serializer::data::QcSerializedItem, tests::init_logger};

    #[test]
    fn ephemeris_only_context_serializer() {
        init_logger();

        let mut ctx = QcContext::new();

        // load NAV
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        let mut serializer = ctx.serializer();

        let mut header_found = false;
        let mut points = 0;

        while let Some(item) = serializer.next() {
            match item {
                QcSerializedItem::RINEXHeader(_) => {
                    header_found = true;
                }
                QcSerializedItem::Ephemeris(_) => {
                    points += 1;
                }
                _ => {
                    panic!("proposed unexpected item");
                }
            }
        }
        assert!(header_found, "did not propose header section!");
        assert!(points > 0, "did not propose any data points!");
    }

    #[test]
    #[cfg(feature = "sp3")]
    fn sp3_only_context_serializer() {
        init_logger();

        let mut ctx = QcContext::new();

        ctx.load_gzip_sp3_file("data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz")
            .unwrap();

        let mut serializer = ctx.serializer();

        let mut header_found = false;
        let mut points = 0;

        while let Some(item) = serializer.next() {
            match item {
                QcSerializedItem::SP3Header(_) => {
                    header_found = true;
                }
                QcSerializedItem::PreciseState(_) => {
                    points += 1;
                }
                _ => {
                    panic!("proposed unexpected item");
                }
            }
        }
        assert!(header_found, "did not propose header section!");
        assert!(points > 0, "did not propose any data points!");
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

        assert!(points > 0, "did not propose any data points!");
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
