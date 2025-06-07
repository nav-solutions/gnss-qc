use crate::prelude::{Duration, Epoch, QcContext, Split};

impl Split for QcContext {
    fn split(&self, t: Epoch) -> (Self, Self)
    where
        Self: Sized,
    {
        (self.clone(), self.clone())
    }

    fn split_even_dt(&self, dt: Duration) -> Vec<Self>
    where
        Self: Sized,
    {
        Default::default()
    }

    fn split_mut(&mut self, t: Epoch) -> Self {
        let rhs = self.clone();

        for (_, data) in self.data.iter_mut() {
            if let Some(rinex) = data.as_mut_rinex() {
                rinex.split_mut(t);
            } else if let Some(sp3) = data.as_mut_sp3() {
                sp3.split_mut(t);
            }
        }

        rhs
    }
}
