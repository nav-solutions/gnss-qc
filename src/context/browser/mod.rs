//! User data browser
mod pointer;
use pointer::Pointer;

mod point;
pub use point::QcDataPoint;

use crate::{
    context::{
        types::InputType,
    },
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct QcBrowsingCaracteristics {
    /// Temporal axis. Can be null
    /// when no temporal product was loaded.
    time_axis: Option<Timeseries>,

    /// Maximal sat. PRN per Constellation
    max_prn: HashMap<Constellation, u8>,

    /// Minimal sat. PRN per Constellation
    min_prn: HashMap<Constellation, u8>,
}

/// Qc context browser
pub struct QcBrowser {
    /// Current ID 
    id: Id,

    /// ID table
    id_table: HashMap<InputProduct, Id>,

    /// Possible filter
    filter: Option<QcBrowsingFilter>, 

    /// RINEX observation pointer for each file
    obs_iter: HashMap<Id, ObservationIterator>,
}

impl Browser {
    pub fn with_filter(&self, filter: Filter) -> Self {
        let mut s = self.clone();
        s.filter = Some(filter);
        s
    }
}

impl Iterator for QcBrowser {
    fn next(&mut self) -> Option<QcDataPoint> {
        // determine next item to search
        let id = self.id.next()?;

        // retrieve input type for this id
        let input 
        
    }
}

/// Possible browser filter
#[derive(Clone, Debug, PartialEq)]
pub enum QcBrowsingFilter {
    /// Only matching file name
    FileName(String),

    /// Only matching parent dir
    Directory(String),

    /// Only matching file type
    InputType(InputType),
}

impl QcContext {
    /// Obtain [QcBrowsingCaracteristics] for this context,
    /// defining how we will iterate.
    fn browsing_caracteristics(&self) -> QcBrowsingCaracteristics {
        let mut min_prn = HashMap::with_size(4);
        let mut max_prn = HashMap::with_size(4);
        let mut time_axis = None;

        for input in self.input {

            #[cfg(feature = "nav")]
            for 

        }

        QcBrowsingCaracteristics {
            time_axis,
            min_prn,
            max_prn,
            sat_signals,
        }
    }

    /// Obtain a [QcBrowser] to iterate this [QcContext] efficiently.
    pub fn browser(&self) -> QcBrowser {
        QcBrowser::from_caracteristics(self.browsing_caracteristics())
    }

    /// Obtain a [QcBrowser] with a custom filter preset.
    pub fn browser_with_filter(&self, filter: QcBrowsingFilter) -> {
        self.browse().with_filter(filter)
    }
}
