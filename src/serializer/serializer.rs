use crate::{
    context::QcContext,
    serializer::{data::QcSerializedItem, state::State},
};

// Synchronous [QcContext] Iterator
pub struct QcSerializer {
    /// Current [State] of the [QcSerializer]
    state: State,
}

impl QcContext {
    /// Obtain a synchronous [QcSerializer] from current [QcContext], ready to serialize the entire context.
    pub fn serializer<'a>(&'a self) -> QcSerializer {
        QcSerializer {
            state: State::default(),
        }
    }
}

impl QcSerializer {}

impl Iterator for QcSerializer {
    type Item = QcSerializedItem;

    fn next(&mut self) -> Option<Self::Item> {
        // Try to pull new symbol
        let mut ret = Option::<QcSerializedItem>::None;

        loop {
            if ret.is_some() {
                return ret;
            }
            match self.state {
                State::Constants => {
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
    fn ephemeris_context_serializer() {
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
