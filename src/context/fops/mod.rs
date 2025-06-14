mod merge;
mod synthesis;

/// [FileOps] describes the list of supported File Operations
pub enum QcFileOps {
    /// [FileOps::Merge] will combine identical products
    /// into a single one. You can typically load several
    /// single GNSS products and use this to form a multi-GNSS product.
    Merge,

    /// [FileOps::TimeBinning] will split temporal products
    /// in a batch of equal duration.
    TimeBinning,
    
    /// [FileOps::SingleGnssBinning] will split Multi-GNSS products
    /// into single GNSS products
    GnssBinning,
}

/// [QcFileOpsBuilder] is used to describe File Operations.
/// [QcFileOps] describes the list of supported operations.
#[derive(Debug, Clone)]
pub struct QcFileOpsBuilder {
    /// [QcFileOps]
    pub(crate) fops: QcFileOps,

    /// [QcMatchBy], set to None when [QcFileOps]
    /// should apply to all components.
    pub(crate) match_by: Option<QcMatchBy>,
}

impl QcFileOpsBuilder {
    /// Apply [FileOps::Merge] to all components
    pub fn merge(&self) -> Self {
        Self {
            fops: QcFileOps::Merge,
            match_by: None,
        }
    }

    /// Apply [FileOps::TimeBinning] to all components
    pub fn time_binning(&self) -> Self {
        Self {
            fops: QcFileOps::Merge,
            match_by: None,
        }
    }

    /// Apply [QcFileOps::GnssBinning] to all components
    pub fn gnss_binning(&self) -> Self {
        Self {
            fops: QcFileOps::GnssBinning,
            match_by: None,
        }
    }

    /// Restrict operation to this [QcProductType]
    pub fn match_by_product(&self, product_type: QcProductType) -> Self {
        let mut s = self.clone();
        s.match_by = Some(QcMatchBy::product_type(product_type));
        s
    }

    /// Restrict operation to this file name.
    pub fn match_by_file(&self, name: &str) -> Self {
        let mut s = self.clone();
        s.match_by = Some(QcMatchBy::file_name(name));
        s
    }

    /// Restrict operation to this agency (data publisher/producer).
    /// This is particularly useful in multi-products and multi labotaroties setups.
    pub fn match_by_agency(&self, name: &str) -> Self {
        let mut s = self.clone();
        s.match_by = Some(QcMatchBy::agency(name));
        s
    }
    
    /// Restrict operation to this GNSS receiver model.
    /// This is particularly useful in RTK/differential setups.
    pub fn match_by_agency(&self, name: &str) -> Self {
        let mut s = self.clone();
        s.match_by = Some(QcMatchBy::agency(name));
        s
    }

    /// Restrict operation to data labeled by this custom ID
    pub fn match_by_custom_id(&self, id: &str) -> Self {
        let mut s = self.clone();
        s.match_by = Some(QcMatchBy::custom(id));
        s
    }
}
