//! P.V.T scenarios. Each scenario deploys a complete gnss-qc context
//! (input products table -> MISO engine -> navigation processor) and verifies
//! the P.V.T solutions stream.
use std::time::Instant;

use super::*;
use crate::{
    geodesy::ecef_to_enu_vector,
    io::{rinex::QcRinexObsSource, sp3::QcSp3Source},
};

/// Test output type
#[derive(Debug, Clone, PartialEq)]
pub enum NavOutput {
    Builtin(QcOutput),
}

impl From<QcOutput> for NavOutput {
    fn from(output: QcOutput) -> Self {
        Self::Builtin(output)
    }
}

/// Results of one navigation run
pub struct NavigationRun {
    pub solutions: Vec<QcPvtSolution>,
    pub report: QcNavigationReport,
    /// Rover reference position (antenna reference point, RINEX header)
    pub truth_ecef_m: (f64, f64, f64),
}

/// Error statistics (m), after convergence
#[derive(Debug)]
pub struct ErrorStats {
    pub count: usize,
    pub max_horizontal: f64,
    pub max_vertical: f64,
    pub rms_3d: f64,
}

impl NavigationRun {
    /// Errors of the solutions obtained after `convergence`
    pub fn errors(&self, convergence: Duration) -> ErrorStats {
        let t0 = self.solutions[0].solution.epoch;

        let errors = self
            .solutions
            .iter()
            .filter(|sol| sol.solution.epoch >= t0 + convergence)
            .map(|sol| {
                let (x, y, z) = sol.solution.pos_m;
                let (tx, ty, tz) = self.truth_ecef_m;
                ecef_to_enu_vector(self.truth_ecef_m, (x - tx, y - ty, z - tz))
            })
            .collect::<Vec<_>>();

        let count = errors.len();
        assert!(count > 0, "no solution after convergence");
        let max_horizontal = errors
            .iter()
            .map(|(e, n, _)| (e * e + n * n).sqrt())
            .fold(0.0, f64::max);
        let max_vertical = errors.iter().map(|(_, _, u)| u.abs()).fold(0.0, f64::max);
        let rms_3d = (errors
            .iter()
            .map(|(e, n, u)| e * e + n * n + u * u)
            .sum::<f64>()
            / count as f64)
            .sqrt();

        ErrorStats {
            count,
            max_horizontal,
            max_vertical,
            rms_3d,
        }
    }
}

/// Deploys the gnss-qc context and runs the navigation
pub fn navigate(
    products: QcInputProducts,
    extra_sources: Vec<Box<dyn QcSource>>,
    cfg: QcNavigationConfig,
    window: (Epoch, Epoch),
) -> NavigationRun {
    let now = Instant::now();

    // truth: rover antenna reference point (header)
    let rover = cfg.rover.clone().unwrap_or_else(esbc);
    let truth_ecef_m = products
        .obs_rinex_iter()
        .find(|(idx, _)| **idx == rover)
        .and_then(|(_, rinex)| QcRinexObsSource::antenna_reference_point(rinex))
        .expect("rover reference position");

    let processor = QcNavigationProcessor::new(almanac(), cfg).unwrap();

    let mut builder = QcEngine::<NavOutput>::builder()
        .with_products(products)
        .with_time_window(Some(window.0), Some(window.1))
        .with_processor(processor);

    for source in extra_sources {
        builder = builder.with_boxed_source(source);
    }

    let engine = builder.build().unwrap();

    let mut solutions = Vec::new();
    let mut report = None;

    for item in engine {
        match item.unwrap().payload {
            NavOutput::Builtin(QcOutput::PVTSolution(solution)) => solutions.push(solution),
            NavOutput::Builtin(QcOutput::NavigationReport(r)) => report = Some(r),
            _ => {}
        }
    }

    let report = report.expect("navigation report");

    println!("{:?} - {:?}", now.elapsed(), report);

    NavigationRun {
        solutions,
        report,
        truth_ecef_m,
    }
}

fn static_user() -> UserParameters {
    UserParameters::new(UserProfile::Static, ClockProfile::Quartz)
}

fn window() -> (Epoch, Epoch) {
    (
        epoch("2020-06-25T00:00:00 GPST"),
        epoch("2020-06-25T01:00:00 GPST"),
    )
}

/// Precise products of GRG, truncated: only states up to `end` are delivered.
fn truncated_sp3(end: Epoch) -> Box<dyn QcSource> {
    let sp3 = sp3::prelude::SP3::from_gzip_file(data_path(GRG_SP3)).unwrap();
    let mut source = QcSp3Source::new(QcIndexing::Agency("GRG".to_string()), sp3);

    let mut items = Vec::new();
    while let QcPoll::Ready(item) = source.poll().unwrap() {
        if item.epoch <= end {
            items.push(item);
        }
    }

    Box::new(QcIterSource::new(
        source.descriptor().clone(),
        items.into_iter(),
    ))
}

fn cpp(orbits: QcOrbitPreference) -> QcNavigationConfig {
    QcNavigationConfig::default()
        .with_method(Method::CPP)
        .with_orbits(orbits)
        .with_user_parameters(static_user())
}

/// Convergence period, excluded from the accuracy statistics
fn convergence() -> Duration {
    Duration::from_seconds(600.0)
}

/// Scenario 1: absolute navigation (CPP, GPS + Galileo), broadcast ephemerides only.
/// OBS + NAV: orbits and clocks are resolved from the radio ephemerides (Keplerian solving).
#[test]
fn absolute_cpp_broadcast() {
    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, ESBC_NAV]);

    let run = navigate(products, vec![], cpp(QcOrbitPreference::Auto), window());

    // every epoch solved, only broadcast orbits
    assert_eq!(run.report.epochs, 121);
    assert_eq!(run.report.solutions, 121, "{:?}", run.report.failures);
    assert_eq!(run.report.precise_orbits, 0);
    assert!(run.report.broadcast_orbits > 0);

    for sol in run.solutions.iter() {
        assert_eq!(sol.receiver, esbc());
        assert_eq!(sol.solution.solution_type, PVTSolutionType::PPP);
    }

    // broadcast orbits and clocks: meter level
    let errors = run.errors(convergence());
    println!("{:?}", errors);
    assert_eq!(errors.count, 101); // after 10' convergence
    assert!(errors.max_horizontal < 3.0, "{:?}", errors);
    assert!(errors.max_vertical < 4.0, "{:?}", errors);
    assert!(errors.rms_3d < 3.0, "{:?}", errors);
}

/// Scenario 2: OBS + NAV + SP3. Precise products are preferred (Auto):
/// orbits and clocks come from the SP3 (Lagrange interpolation), except at the very
/// first epoch, where the transmission instant precedes the first SP3 sample
/// (no extrapolation): broadcast ephemerides take over for that epoch only.
/// The solutions are more accurate than with broadcast ephemerides.
#[test]
fn absolute_cpp_precise_orbits() {
    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, ESBC_NAV, GRG_SP3]);

    let run = navigate(products, vec![], cpp(QcOrbitPreference::Auto), window());

    assert_eq!(run.report.solutions, 121, "{:?}", run.report.failures);

    // precise products are preferred: > 99% of the orbital states
    let total = run.report.precise_orbits + run.report.broadcast_orbits;
    assert!(
        run.report.precise_orbits as f64 > 0.99 * total as f64,
        "{:?}",
        run.report
    );

    let errors = run.errors(convergence());
    println!("{:?}", errors);
    assert!(errors.max_horizontal < 1.5, "{:?}", errors);
    assert!(errors.max_vertical < 2.0, "{:?}", errors);
    assert!(errors.rms_3d < 1.6, "{:?}", errors);
}

/// Scenario 3: the precise products stop covering the session (truncated SP3, 03:00).
/// - Auto: broadcast ephemerides take over, satellite per satellite, the solutions
///   stream continues without interruption.
/// - PreciseOnly (user choice): no more solutions once the precise products end.
#[test]
fn absolute_cpp_precise_coverage_loss() {
    let window = (
        epoch("2020-06-25T02:00:00 GPST"),
        epoch("2020-06-25T03:30:00 GPST"),
    );

    let sp3_end = epoch("2020-06-25T03:00:00 GPST");

    // Auto
    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, ESBC_NAV]);
    let sources = vec![truncated_sp3(sp3_end)];
    let auto = navigate(products, sources, cpp(QcOrbitPreference::Auto), window);

    assert_eq!(auto.report.epochs, 181);
    assert_eq!(auto.report.solutions, 181, "{:?}", auto.report.failures);
    assert!(auto.report.precise_orbits > 0);
    assert!(auto.report.broadcast_orbits > 0);

    // continuous solutions stream (30s)
    for pair in auto.solutions.windows(2) {
        assert_eq!(
            pair[1].solution.epoch - pair[0].solution.epoch,
            Duration::from_seconds(30.0)
        );
    }

    let errors = auto.errors(convergence());
    println!("auto: {:?}", errors);
    assert!(errors.max_horizontal < 3.0, "{:?}", errors);
    assert!(errors.max_vertical < 4.0, "{:?}", errors);

    // PreciseOnly
    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, ESBC_NAV]);
    let sources = vec![truncated_sp3(sp3_end)];
    let precise = navigate(
        products,
        sources,
        cpp(QcOrbitPreference::PreciseOnly),
        window,
    );

    assert_eq!(precise.report.broadcast_orbits, 0);
    assert!(precise.report.precise_orbits > 0);

    // solutions up to the end of the precise products (02:00 to 03:00 included), none after
    let last = precise.solutions.last().unwrap().solution.epoch;
    assert_eq!(last, sp3_end);
    assert_eq!(
        precise.report.solutions, 121,
        "{:?}",
        precise.report.failures
    );
    assert_eq!(precise.report.failures.values().sum::<usize>(), 60);
}

/// Scenario 4: differential navigation (RTK-CPP), ESBC00DNK rover, MOJN00DNK base (~50 km).
/// Both stations are signal sources of the MISO engine, synchronized epoch by epoch.
/// The base position is taken from its RINEX header (source descriptor).
#[test]
fn rtk_cpp_esbc_mojn() {
    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, MOJN_OBS, ESBC_NAV]);

    let cfg = cpp(QcOrbitPreference::Auto)
        .with_rover(esbc())
        .with_rtk_base(mojn());

    let run = navigate(products, vec![], cfg, window());

    assert_eq!(run.report.receiver, esbc());
    assert_eq!(run.report.solutions, 121, "{:?}", run.report.failures);

    for sol in run.solutions.iter() {
        assert_eq!(sol.receiver, esbc());
        assert_eq!(sol.solution.solution_type, PVTSolutionType::RTK);
    }

    // decimeter level (truth and base coordinates: RINEX headers, same frame)
    let errors = run.errors(convergence());
    println!("{:?}", errors);
    assert!(errors.max_horizontal < 0.6, "{:?}", errors);
    assert!(errors.max_vertical < 1.0, "{:?}", errors);
    assert!(errors.rms_3d < 0.6, "{:?}", errors);
}

/// ESBC00DNK antenna reference point, expressed in ITRF2014 (frame of the GRG products):
/// absolute techniques converge to this position (RINEX header: ETRF2000).
fn itrf2014(run: &NavigationRun) -> (f64, f64, f64) {
    crate::geodesy::etrf2000_to_itrf2014(run.truth_ecef_m, 2020.48)
}

/// Scenario 5: SPP (single frequency, L1 / E1), OBS + NAV + SP3.
/// Only the primary signals are proposed to the solver, precise products clocks
/// are referenced to the iono free combination: the broadcast group delays are applied.
/// No ionosphere model yet: meter level accuracy (low ionospheric activity here:
/// night time, solar minimum).
#[test]
fn absolute_spp_precise_orbits() {
    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, ESBC_NAV, GRG_SP3]);

    let cfg = cpp(QcOrbitPreference::Auto).with_method(Method::SPP);
    let run = navigate(products, vec![], cfg, window());

    assert_eq!(run.report.solutions, 121, "{:?}", run.report.failures);

    let errors = run.errors(convergence());
    println!("{:?}", errors);
    assert!(errors.max_horizontal < 2.0, "{:?}", errors);
    assert!(errors.max_vertical < 2.5, "{:?}", errors);
    assert!(errors.rms_3d < 2.5, "{:?}", errors);
}

/// Scenario 6: float PPP (iono free code and phase), OBS + NAV + SP3, 3 hours.
/// Compared to the ITRF2014 coordinates of the station after 1h30 of convergence:
/// centimeter level (header coordinates are ETRF2000: 0.75 m away).
#[test]
fn absolute_ppp() {
    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, ESBC_NAV, GRG_SP3]);

    let cfg = cpp(QcOrbitPreference::Auto).with_method(Method::PPP);

    let window = (
        epoch("2020-06-25T00:00:00 GPST"),
        epoch("2020-06-25T03:00:00 GPST"),
    );

    let mut run = navigate(products, vec![], cfg, window);

    assert_eq!(run.report.solutions, 361, "{:?}", run.report.failures);

    run.truth_ecef_m = itrf2014(&run);

    let errors = run.errors(Duration::from_hours(1.5));
    println!("{:?}", errors);
    assert_eq!(errors.count, 181);
    assert!(errors.max_horizontal < 0.15, "{:?}", errors);
    assert!(errors.max_vertical < 0.15, "{:?}", errors);
    assert!(errors.rms_3d < 0.12, "{:?}", errors);
}

/// Scenario 7: RTK-PPP (double differences of iono free code and phase),
/// ESBC00DNK rover, MOJN00DNK base, OBS + NAV + SP3.
#[test]
fn rtk_ppp_esbc_mojn() {
    let products = load_products(
        &QcInputConfig::default(),
        &[ESBC_OBS, MOJN_OBS, ESBC_NAV, GRG_SP3],
    );

    let cfg = cpp(QcOrbitPreference::Auto)
        .with_method(Method::PPP)
        .with_rover(esbc())
        .with_rtk_base(mojn());

    let run = navigate(products, vec![], cfg, window());

    assert_eq!(run.report.solutions, 121, "{:?}", run.report.failures);

    for sol in run.solutions.iter() {
        assert_eq!(sol.solution.solution_type, PVTSolutionType::RTK);
    }

    let errors = run.errors(convergence());
    println!("{:?}", errors);
    assert!(errors.max_horizontal < 0.4, "{:?}", errors);
    assert!(errors.max_vertical < 0.4, "{:?}", errors);
    assert!(errors.rms_3d < 0.4, "{:?}", errors);
}
