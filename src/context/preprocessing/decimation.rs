use crate::prelude::{Decimate, DecimationFilter, QcContext};

impl Decimate for QcContext {
    fn decimate(&self, f: &DecimationFilter) -> Self {
        let mut s = self.clone();
        s.decimate(f);
        s
    }

    /// Applies the following [DecimationFilter] to the entire [QcContext] with mutable access.
    fn decimate_mut(&mut self, f: &DecimationFilter) {
        for (_, data) in self.data.iter_mut() {
            if let Some(rinex) = data.as_mut_rinex() {
                rinex.decimate_mut(f);
            } else if let Some(sp3) = data.as_mut_sp3() {
                sp3.decimate_mut(f);
            }
        }
    }
}
