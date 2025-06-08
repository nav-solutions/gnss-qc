use crate::prelude::{Duration, Epoch, QcContext};

impl QcContext {
    /// Returns first (earliest) [Epoch] of this [QcContext], considering
    /// all data symbols from all temporal products.   
    /// Returns [None] for [QcContext]s that only comprise a-temporal products.
    pub fn first_epoch(&self) -> Option<Epoch> {
        let mut ret = Option::<Epoch>::None;

        for (_, data) in self.data.iter() {
            if let Some(rinex) = data.as_rinex() {
                if let Some(t0) = rinex.first_epoch() {
                    if let Some(ret) = &mut ret {
                        if t0 < *ret {
                            *ret = t0;
                        }
                    } else {
                        ret = Some(t0);
                    }
                }
            }

            #[cfg(feature = "sp3")]
            if let Some(sp3) = data.as_sp3() {
                let t0 = sp3.first_epoch();
                if let Some(ret) = &mut ret {
                    if t0 < *ret {
                        *ret = t0;
                    }
                } else {
                    ret = Some(t0);
                }
            }
        }

        ret
    }

    /// Returns last (latest) [Epoch] of this [QcContext], considering
    /// all data symbols from all temporal products.   
    /// Returns [None] for [QcContext]s that only comprise a-temporal products.
    pub fn last_epoch(&self) -> Option<Epoch> {
        let mut ret = Option::<Epoch>::None;

        for (_, data) in self.data.iter() {
            if let Some(rinex) = data.as_rinex() {
                if let Some(t) = rinex.last_epoch() {
                    if let Some(ret) = &mut ret {
                        if t > *ret {
                            *ret = t;
                        }
                    } else {
                        ret = Some(t);
                    }
                }
            }

            #[cfg(feature = "sp3")]
            if let Some(sp3) = data.as_sp3() {
                if let Some(t) = sp3.last_epoch() {
                    if let Some(ret) = &mut ret {
                        if t > *ret {
                            *ret = t;
                        }
                    } else {
                        ret = Some(t);
                    }
                }
            }
        }

        ret
    }

    /// Returns total [Duration] of this [QcContext], considering  all temporal products.   
    /// Returns [Duration::ZERO] for [QcContext]s that only comprise a-temporal products.
    pub fn total_duration(&self) -> Duration {
        let mut duration = Duration::ZERO;

        if let Some(t0) = self.first_epoch() {
            if let Some(t) = self.last_epoch() {
                duration = t - t0;
            }
        }

        duration
    }
}

#[cfg(test)]
mod test {

    use std::str::FromStr;

    use crate::{
        prelude::{Epoch, QcContext},
        tests::init_logger,
    };

    #[test]
    fn sampling_simple_time_frame() {
        init_logger();
        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/OBS/V3/LARM0010.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/LARM0630.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0630.22O").unwrap();

        ctx.load_gzip_rinex_file("data/MET/V3/POTS00DEU_R_20232540000_01D_05M_MM.rnx.gz")
            .unwrap();

        let t0_utc = Epoch::from_str("2021-12-31T23:59:42").unwrap();
        let t_n_gpst = Epoch::from_str("2023-09-11T23:55:00").unwrap();

        let t0 = ctx.first_epoch().expect("T0 should be determined!");

        assert_eq!(t0, t0_utc);

        let t_n = ctx.last_epoch().expect("Undetermined last epoch!");

        assert_eq!(t_n, t_n_gpst);
        assert_eq!(ctx.total_duration(), t_n - t0);
    }

    #[test]
    fn sampling_intermediate_time_frame() {
        init_logger();
        let mut ctx = QcContext::new();

        // load data
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
            .unwrap();

        ctx.load_gzip_rinex_file("data/CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz")
            .unwrap();

        ctx.load_gzip_sp3_file("data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz")
            .unwrap();

        let t0_utc = Epoch::from_str("2020-06-24T19:49:42 UTC").unwrap();
        let t_n_gpst = Epoch::from_str("2020-06-26T00:00:00 GPST").unwrap();

        let t0 = ctx.first_epoch().expect("T0 should be determined!");

        assert_eq!(t0, t0_utc);

        let t_n = ctx.last_epoch().expect("Undetermined last epoch!");

        assert_eq!(t_n, t_n_gpst);
        assert_eq!(ctx.total_duration(), t_n - t0);
    }
}
