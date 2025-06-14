use crate::prelude::{Decimate, DecimationFilter, QcContext};

impl QcContext {
    /// Applies the following [DecimationFilter] to the entire [QcContext] with mutable access.
    pub fn decimate_mut(&mut self, f: &DecimationFilter) {
        for (_, data) in self.data.iter_mut() {
            if let Some(rinex) = data.as_mut_rinex() {
                rinex.decimate_mut(f);
            } else if let Some(sp3) = data.as_mut_sp3() {
                sp3.decimate_mut(f);
            }
        }
    }
}
