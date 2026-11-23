//! IPP processor versus the navigation solver: the lines of sight (elevation, azimuth)
//! computed by the IPP processor match those of the GNSS-RTK solutions.
use std::collections::HashMap;

use super::*;
use crate::{processors::elevation_azimuth_deg, tests::navigation::scenarios::navigate};

#[test]
fn ipp_lines_of_sight_match_navigation() {
    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, ESBC_NAV, GRG_SP3]);

    let cfg = QcNavigationConfig::default()
        .with_method(Method::CPP)
        .with_user_parameters(UserParameters::new(
            UserProfile::Static,
            ClockProfile::Quartz,
        ));

    let window = (
        epoch("2020-06-25T00:00:00 GPST"),
        epoch("2020-06-25T00:30:00 GPST"),
    );

    let run = navigate(products, vec![], cfg, window);

    // IPP processor line of sight computation, with the navigated position
    // and the precise orbits, compared to the solver contributions
    let sp3 = sp3::prelude::SP3::from_gzip_file(data_path(GRG_SP3)).unwrap();
    let positions = sp3
        .data
        .iter()
        .map(|(k, e)| ((k.epoch, k.sv), e.position_km))
        .collect::<HashMap<_, _>>();

    let mut compared = 0;

    for sol in run.solutions.iter() {
        let t = sol.solution.epoch;
        for contribution in sol.solution.sv.iter() {
            // SP3 sampling instants only (no interpolation needed)
            if let Some((x, y, z)) = positions.get(&(t, contribution.sv)) {
                let (elevation, azimuth) =
                    elevation_azimuth_deg(sol.solution.pos_m, (x * 1.0E3, y * 1.0E3, z * 1.0E3));
                // the solver evaluates the satellite at transmission time (~70 ms earlier)
                assert!(
                    (elevation - contribution.elevation_deg).abs() < 0.01,
                    "{:?}",
                    contribution
                );
                assert!(
                    (azimuth - contribution.azimuth_deg).abs() < 0.01,
                    "{:?}",
                    contribution
                );
                compared += 1;
            }
        }
    }

    assert!(compared > 20, "{}", compared);
}
