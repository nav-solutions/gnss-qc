use crate::prelude::{SV, Epoch, Carrier};



pub enum Pointer {
    /// Observation RINEX pointer
    Observation(ObservationPointer),
}

impl Iterator for Pointer {
    
}   

#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct ObservationPointer {
    /// Current satellite as [SV]
    pub satellite: SV,

    /// Current Epoch 
    pub epoch: Epoch,

    /// Current signal
    pub signal: Carrier,
}
