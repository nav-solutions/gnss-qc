use crate::prelude::{Epoch, SV};

#[derive(Debug, Clone)]
pub struct QcPreciseState {
    /// [SV] State
    pub sv: SV,

    /// State [Epoch]
    pub epoch: Epoch,

    /// State (position _km)
    pub position_km: (f64, f64, f64),
}
