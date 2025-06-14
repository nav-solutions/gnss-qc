use crate::context::{QcIndexing, QcProductType};

/// [QcMatchBy] regroups all our data search methods
#[derive(Debug, Clone, PartialEq)]
pub enum QcMatchBy {
    /// Acces data from this file source
    Filename(String),

    /// Access data indexed by [QcIndexing]
    Indexing(QcIndexing),
    
    /// Access all data of this [QcProductType]
    ProductType(QcProductType),
}


impl QcMatchBy {
    /// Builds a new [QcProductType] filter
    pub fn product_type(product: QcProductType) -> Self {
        Self::ProductType(product)
    }
    
    /// Builds a new [QcIndexing] filter
    pub fn indexing(index: QcIndexing) -> Self {
        Self::Indexing(index)
    }

    /// Builds a new file name filter
    pub fn file_name(name: &str) -> Self {
        Self::Filename(name.to_string())
    }
    
    /// Builds a new Agency name filter
    pub fn agency(name: &str) -> Self {
        Self::indexing(QcIndexing::from_agency(name))
    }

    /// Builds a new custom label filter
    pub fn custom(label: &str) -> Self {
        Self::indexing(QcIndexing::from_custom_label(label))
    }
    
    /// Builds a new Geodetic marker filter
    pub fn geodetic_marker(name: &str) -> Self {
        Self::indexing(QcIndexing::GeodeticMarker(name.to_string()))
    }

    /// Builds a new GNSS receiver model filter
    pub fn gnss_receiver_model(model: &str) -> Self {
        Self::indexing(QcIndexing::GnssReceiver(model.to_string()))
    }
    
    /// Builds a new receiver Antenna model filter
    pub fn gnss_receiver_antenna(model: &str) -> Self {
        Self::indexing(QcIndexing::RxAntenna(model.to_string()))
    }

    /// Builds a new operator name filter
    pub fn operator(name: &str) -> Self {
        Self::indexing(QcIndexing::Operator(name.to_string()))
    }
}
