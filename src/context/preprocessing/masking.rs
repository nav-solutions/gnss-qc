use crate::prelude::{MaskFilter, Masking, QcContext};

impl Masking for QcContext {
    fn mask(&self, mask: &MaskFilter) -> Self {
        let mut s = self.clone();
        s.mask(mask);
        s
    }

    /// Applies the following [MaskFilter] to this entire [QcContext] with mutable access.
    fn mask_mut(&mut self, mask: &MaskFilter) {
        for (_, data) in self.data.iter_mut() {
            if let Some(rinex) = data.as_mut_rinex() {
                rinex.mask_mut(mask);
            } else if let Some(sp3) = data.as_mut_sp3() {
                sp3.mask_mut(mask);
            }
        }
    }
}
