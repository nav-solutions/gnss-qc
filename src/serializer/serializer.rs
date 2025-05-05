use crate::{
    context::{data::QcSourceDescriptor, QcContext, QcProductType},
    prelude::Epoch,
    serializer::{data::QcSerializedItem, ephemeris::QcEphemerisIterator, state::State},
};

/// [QcSerializer] to serialize entire [QcContext].
pub struct QcSerializer<'a> {
    /// Current [State] of the [QcSerializer]
    state: State,

    // /// Latest serialized epoch
    // latest: Option<Epoch>,
    ephemeris_sources: Vec<QcEphemerisIterator<'a>>,

    // /// Last streamed
    // last_streamed: Option<&'a QcSourceDescriptor>,
    /// All source descriptors contained
    descriptors: Vec<(QcSourceDescriptor, bool)>,
}

impl QcContext {
    /// Obtain a synchronous [QcSerializer] from current [QcContext], ready to serialize the entire context.
    pub fn serializer<'a>(&'a self) -> QcSerializer<'a> {
        let mut ephemeris_sources = Vec::new();
        let mut descriptors = Vec::with_capacity(4);

        for entry in self.data.iter() {
            if let Some(serializer) = self.ephemeris_serializer(entry.descriptor.indexing.clone()) {
                ephemeris_sources.push(serializer);
            }

            descriptors.push((entry.descriptor.clone(), false)); // store for later
        }

        debug!("streamer: latched descriptors: {:#?}", descriptors);

        QcSerializer {
            descriptors,
            ephemeris_sources,
            state: Default::default(),
            // last_streamed: None,
            // latest: Default::default(),
        }
    }
}

impl<'a> Iterator for QcSerializer<'a> {
    type Item = QcSerializedItem;

    fn next(&mut self) -> Option<Self::Item> {
        // Try to pull new symbol
        // let mut ret = Option::<QcSerializedItem>::None;

        loop {
            // if ret.is_some() {
            //     return ret;
            // }

            debug!("state: {:?}", self.state);

            match self.state {
                State::Constants => {
                    self.state = State::Ephemeris;
                }

                #[cfg(feature = "navigation")]
                State::Ephemeris => {
                    // round robin all data sources (until all completed)
                    for (index, (source, done)) in self.descriptors.iter_mut().enumerate() {
                        if !*done && source.product_type == QcProductType::BroadcastNavigation {
                            if let Some(data) = self.ephemeris_sources[index].next() {
                                return Some(QcSerializedItem::Ephemeris(data));
                            } else {
                                *done = true;
                            }
                        }
                    }

                    self.state = State::Done;
                }

                #[cfg(not(feature = "navigation"))]
                State::Ephemeris => {
                    self.state = State::Done;
                }

                State::Meteo => {
                    return None;
                }

                State::PreciseOrbit => {
                    return None;
                }

                State::PreciseClock => {
                    return None;
                }

                State::Signal => {
                    return None;
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
}
