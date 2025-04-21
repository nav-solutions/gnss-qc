pub use crate::prelude::QcContext;

mod ephemeris;
mod signals;

use ephemeris::QcEphemerisBuffer;
use signals::QcSignalBuffer;

/// [QcNavigationBuffer] is obtained from [QcContext] reference and
/// and is used in all post navigation processes.
pub struct QcNavigationBuffer<'a> {
    /// [QcEphemerisBuffer] from data source
    pub ephemeris: QcEphemerisBuffer<'a>,
    /// [QcSignalBuffer] from data source
    pub signals: QcSignalBuffer<'a>,
}

impl QcContext {
    /// Obtain a [QcNavigationBuffer] from this [QcContext], to be used in post processed navigation.
    /// This requires a minimum of one Observation RINEX and one Navigation RINEX files.
    /// It is up to you to provide correct data, we do not have verification logic.
    /// ```
    /// use gnss_qc::prelude::QcContext;
    ///
    /// let mut ctx = QcContext::new();
    ///
    /// ctx.load_rinex_file("data/MET/V2/abvi0010.15m").unwrap();
    /// assert!(ctx.navigation_buffer().is_none(), "not navigation compatible!");
    ///
    /// ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz").unwrap();
    /// assert!(ctx.navigation_buffer().is_none(), "not navigation compatible!");
    ///
    /// ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();
    /// assert!(ctx.navigation_buffer().is_some(), "navigation compatible!");
    /// ```
    pub fn navigation_buffer<'a>(&'a self) -> Option<QcNavigationBuffer<'a>> {
        let signals_iter = self.signals_buffer()?;
        let ephemeris_iter = self.ephemeris_buffer()?;

        Some(QcNavigationBuffer {
            ephemeris: ephemeris_iter,
            signals: signals_iter,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{
        context::navigation::buffer::signals::QcMeasuredData,
        prelude::{Epoch, QcContext, SV},
    };

    #[test]
    fn nav_buffering() {
        let mut ctx = QcContext::new();

        // load other type of data
        ctx.load_rinex_file("data/MET/V2/abvi0010.15m").unwrap();
        assert!(
            ctx.navigation_buffer().is_none(),
            "not navigation compatible!"
        );

        // load observations
        ctx.load_rinex_file("data/OBS/V3/LARM0010.22O").unwrap();
        assert!(
            ctx.navigation_buffer().is_none(),
            "not navigation compatible!"
        );

        // load nav
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();
        assert!(ctx.navigation_buffer().is_some(), "navigation compatible!");
    }
}
