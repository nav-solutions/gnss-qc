pub use crate::prelude::QcContext;

mod ephemeris;
mod signals;

use ephemeris::QcEphemerisBuffer;
use signals::QcSignalBuffer;

/// [QcNavigationBuffer] is obtained from [QcContext] reference
/// and serves any navigation processes.
pub struct QcNavigationBuffer<'a> {
    /// [QcEphemerisBuffer] from data source
    pub ephemeris: QcEphemerisBuffer<'a>,
    /// [QcSignalBuffer] from data source
    pub signals: QcSignalBuffer<'a>,
}

impl QcContext {
    /// Obtain a [QcNavigationBuffer] from this [QcContext], to be used in post processed navigation.
    pub fn navigation_buffer<'a>(&'a self) -> Option<QcNavigationBuffer<'a>> {
        let signals_iter = self.signals_buffer()?;
        let ephemeris_iter = self.ephemeris_buffer()?;

        Some(QcNavigationBuffer {
            ephemeris: ephemeris_iter,
            signals: signals_iter,
        })
    }
}
