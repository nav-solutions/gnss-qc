use crate::prelude::{QcIndexing, QcProductType};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Hash, Serialize)]
pub(crate) struct QcDataKey {
    /// [QcIndexing] being used for this entry
    pub index: QcIndexing,

    /// [QcProductType] we have identified
    pub prod_type: QcProductType,
}
