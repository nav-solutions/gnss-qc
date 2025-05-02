use crate::prelude::{QcIdentifier, QcProductType};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Hash, Serialize)]
pub(crate) struct QcDataKey {
    /// [QcIdentifier] being used for this entry
    pub identifier: QcIdentifier,

    /// [QcProductType] we have identified
    pub prod_type: QcProductType,
}
