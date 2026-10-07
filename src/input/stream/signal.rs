use gnss_rs::prelude::SV;
use hifitime::prelude::Epoch;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

pub struct SignalToken {
    pub epoch: Epoch,
    pub satellite: SV,
}

pub trait SignalSource {
    type Error = InputSignalError;

    fn next(&mut self) -> Result<Option<
}

pub struct SignalStream<S: SignalSource> {
    source: S,
    last: Option<SignalToken>, 
}
