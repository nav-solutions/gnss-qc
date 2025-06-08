use core::num;

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

    descriptor_ptr: usize,
    num_rinex_descriptors: usize,
    num_eph_sources: usize,
    num_sp3_descriptors: usize,

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
    num_precise_sources: usize,

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
        let mut num_eph_sources = 0;
        let mut ephemeris_sources = Vec::new();

        // Precise sources
        #[cfg(feature = "sp3")]
        let mut num_precise_sources = 0;

        #[cfg(feature = "sp3")]
        let mut precise_state_sources = Vec::new();

        for (desc, _) in self.data.iter() {
            if let Some(serializer) = self.ephemeris_serializer(&desc.indexing) {
                num_eph_sources += 1;
                ephemeris_sources.push(serializer);
                continue;
            } else if let Some(serializer) = self.signal_serializer(&desc.indexing) {
                signal_sources.push(serializer);
                continue;
            }

            #[cfg(feature = "sp3")]
            if let Some(serializer) = self.precise_states_serializer(&desc.indexing) {
                num_precise_sources += 1;
                precise_state_sources.push(serializer);
            }
        }

        let total_duration = self.total_duration();
        let epoch = self.first_epoch();

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
            rinex_descriptors,
            num_eph_sources,
            ephemeris_sources,
            descriptor_ptr: 0,

            num_rinex_descriptors,
            state: Default::default(),

            #[cfg(feature = "sp3")]
            num_sp3_descriptors,

            #[cfg(feature = "sp3")]
            sp3_descriptors,

            #[cfg(feature = "sp3")]
            num_precise_sources,

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
                        if self.descriptor_ptr == self.num_eph_sources {
                            // consumed all sources
                            self.descriptor_ptr = 0;
                            self.state = State::Temporal(TemporalState::PreciseOrbits);
                        }
                    }
                }
                State::Temporal(TemporalState::PreciseOrbits) => {
                    if let Some(item) = self.next_precise_state() {
                        return Some(item);
                    } else {
                        if self.descriptor_ptr == self.num_precise_sources {
                            // consumed all sources
                            self.descriptor_ptr = 0;
                            self.state = State::Temporal(TemporalState::Observations);
                        }
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
            data: &data.header,
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
        if self.num_eph_sources == 0 {
            return None;
        }

        if self.descriptor_ptr == self.num_eph_sources {
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
        if self.num_precise_sources == 0 {
            return None;
        }

        if self.descriptor_ptr == self.num_precise_sources {
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

        assert_eq!(ctx.total_rinex_files(), 2);
        assert_eq!(ctx.total_sp3_files(), 0);

        let mut serializer = ctx.serializer();

        let mut header_1_found = false;
        let mut header_2_found = false;

        let mut points_1 = 0;
        let mut points_2 = 0;

        while let Some(item) = serializer.next() {
            match item {
                QcSerializedItem::RINEXHeader(header) => {
                    if header.filename.contains("MOJN00DNK_R_20201770000_01D_MN") {
                        header_1_found = true;
                    } else if header.filename.contains("ESBC00DNK_R_20201770000_01D_MN") {
                        header_2_found = true;
                    } else {
                        panic!("received unexpected header content");
                    }
                }
                QcSerializedItem::Ephemeris(data) => {
                    if data.filename.contains("MOJN00DNK_R_20201770000_01D_MN") {
                        points_1 += 1;
                    } else if data.filename.contains("ESBC00DNK_R_20201770000_01D_MN") {
                        points_2 += 1;
                    } else {
                        panic!("received unexpected content");
                    }
                }
                _ => panic!("received unexpected symbol!"),
            }
        }

        assert!(header_1_found, "did not proposed header #1");
        assert!(header_2_found, "did not proposed header #2");

        assert!(points_1 > 0, "did not propose any data points for file #1!");
        assert!(points_2 > 0, "did not propose any data points for file #2!");
    }

    #[test]
    #[cfg(feature = "sp3")]
    fn dual_sp3_context_serializer() {
        init_logger();

        let mut ctx = QcContext::new();

        ctx.load_gzip_sp3_file("data/SP3/C/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3.gz")
            .unwrap();

        ctx.load_gzip_sp3_file("data/SP3/D/COD0MGXFIN_20230500000_01D_05M_ORB.SP3.gz")
            .unwrap();

        assert_eq!(ctx.total_rinex_files(), 0);
        assert_eq!(ctx.total_sp3_files(), 2);

        let mut serializer = ctx.serializer();

        let mut header_1_found = false;
        let mut header_2_found = false;

        let mut points_1 = 0;
        let mut points_2 = 0;

        while let Some(item) = serializer.next() {
            match item {
                QcSerializedItem::SP3Header(header) => {
                    if header
                        .filename
                        .contains("COD0MGXFIN_20230500000_01D_05M_ORB")
                    {
                        header_1_found = true;
                    } else if header
                        .filename
                        .contains("GRG0MGXFIN_20201760000_01D_15M_ORB")
                    {
                        header_2_found = true;
                    } else {
                        panic!("received unexpected header content");
                    }
                }
                QcSerializedItem::PreciseState(data) => {
                    if data.filename.contains("COD0MGXFIN_20230500000_01D_05M_ORB") {
                        points_1 += 1;
                    } else if data.filename.contains("GRG0MGXFIN_20201760000_01D_15M_ORB") {
                        points_2 += 1;
                    } else {
                        panic!("received unexpected content");
                    }
                }
                _ => panic!("received unexpected symbol!"),
            }
        }

        assert!(header_1_found, "did not proposed header #1");
        assert!(header_2_found, "did not proposed header #2");

        assert!(points_1 > 0, "did not propose any data points for file #1!");
        assert!(points_2 > 0, "did not propose any data points for file #2!");
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
