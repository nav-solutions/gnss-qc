//! Signal combinations (GF, IF, MP) on real data: ESBC00DNK (Septentrio PolaRx5), 2020-06-25.
use std::collections::{BTreeMap, HashMap};

use crate::{
    prelude::*,
    tests::miso::{data_path, load_rinex},
};

const ESBC_OBS: &str = "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz";

struct Results {
    combinations: Vec<(Epoch, QcCombination)>,
    arcs: Vec<QcMultipathArc>,
}

fn run(cfg: QcCombinationConfig) -> Results {
    let mut products = QcInputProducts::default();
    products
        .load_rinex(
            &QcInputConfig::default(),
            data_path(ESBC_OBS),
            load_rinex(ESBC_OBS),
        )
        .unwrap();

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcCombinationProcessor::new(cfg))
        .build()
        .unwrap();

    let (mut combinations, mut arcs) = (Vec::new(), Vec::new());
    let mut last_epoch = None;

    for item in engine {
        let item = item.unwrap();

        // time ordered output
        if let Some(last) = last_epoch {
            assert!(item.epoch >= last);
        }
        last_epoch = Some(item.epoch);

        match item.payload {
            QcOutput::Combination(combination) => combinations.push((item.epoch, combination)),
            QcOutput::MultipathArc(arc) => arcs.push(arc),
            _ => {}
        }
    }

    Results { combinations, arcs }
}

fn receiver(name: &str) -> QcIndexing {
    QcIndexing::Receiver(name.to_string())
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    values[values.len() / 2]
}

#[test]
fn signal_combinations() {
    let results = run(QcCombinationConfig::default());
    let esbc = receiver("SEPT POLARX5 #3047937");

    // every pair of frequencies, not only L1/L2 and L1/L5
    let mut pairs = BTreeMap::<(Constellation, QcCombinationKind, String), usize>::new();

    for (_, c) in results.combinations.iter() {
        assert_eq!(c.receiver, esbc);
        assert_ne!(c.sv.constellation, Constellation::Glonass, "FDMA");
        assert!(c.value_m.is_finite());

        let name = match c.kind {
            QcCombinationKind::Multipath => format!("{} {}", c.lhs, c.rhs),
            _ => format!("{}-{}", c.lhs, c.rhs),
        };
        *pairs.entry((c.sv.constellation, c.kind, name)).or_default() += 1;

        match c.kind {
            QcCombinationKind::Multipath => {
                assert!(c.arc.is_some() && c.debiased_m.is_some());
                assert!(c.is_code());
            }
            _ => {
                assert!(c.arc.is_none() && c.debiased_m.is_none());
            }
        }
    }

    for (pair, count) in pairs.iter() {
        println!("{:?}: {}", pair, count);
    }

    for (constellation, kind, name) in [
        // phase: Lj - L1
        (
            Constellation::GPS,
            QcCombinationKind::GeometryFree,
            "L2W-L1C",
        ),
        (
            Constellation::GPS,
            QcCombinationKind::GeometryFree,
            "L5Q-L1C",
        ),
        // code: C1 - Cj
        (
            Constellation::GPS,
            QcCombinationKind::GeometryFree,
            "C1C-C2W",
        ),
        (
            Constellation::GPS,
            QcCombinationKind::IonosphereFree,
            "C1C-C5Q",
        ),
        (
            Constellation::GPS,
            QcCombinationKind::IonosphereFree,
            "L1C-L2W",
        ),
        // E5b, E5 (AltBOC) as well
        (
            Constellation::Galileo,
            QcCombinationKind::GeometryFree,
            "L7Q-L1C",
        ),
        (
            Constellation::Galileo,
            QcCombinationKind::IonosphereFree,
            "C1C-C8Q",
        ),
        // every code: MP1C, MP1W, MP2W..
        (Constellation::GPS, QcCombinationKind::Multipath, "C1C L2W"),
        (Constellation::GPS, QcCombinationKind::Multipath, "C1W L2W"),
        (Constellation::GPS, QcCombinationKind::Multipath, "C2W L1C"),
        (Constellation::GPS, QcCombinationKind::Multipath, "C5Q L1C"),
        (
            Constellation::Galileo,
            QcCombinationKind::Multipath,
            "C5Q L1C",
        ),
        (
            Constellation::BeiDou,
            QcCombinationKind::Multipath,
            "C2I L6I",
        ),
    ] {
        assert!(
            pairs.contains_key(&(constellation, kind, name.to_string())),
            "{} {} {} not found",
            constellation,
            kind,
            name
        );
    }

    // GF code and GF phase measure the same ionospheric delay (I1 - Ij): within one
    // satellite pass, their difference is constant (ambiguities, biases) + code noise
    let mut gf = HashMap::<(Epoch, SV), (Option<f64>, Option<f64>)>::new();
    for (t, c) in results.combinations.iter() {
        if c.kind != QcCombinationKind::GeometryFree || c.sv.constellation != Constellation::GPS {
            continue;
        }
        let entry = gf.entry((*t, c.sv)).or_default();
        match (c.lhs.to_string().as_str(), c.rhs.to_string().as_str()) {
            ("C1C", "C2W") => entry.0 = Some(c.value_m),
            ("L2W", "L1C") => entry.1 = Some(c.value_m),
            _ => {}
        }
    }

    let mut per_sv = BTreeMap::<SV, Vec<(Epoch, f64)>>::new();
    for ((t, sv), (code, phase)) in gf.iter() {
        if let (Some(code), Some(phase)) = (code, phase) {
            per_sv.entry(*sv).or_default().push((*t, code - phase));
        }
    }

    // first hour of G01..: dispersion of (GF code - GF phase), excluding phase jumps
    let mut dispersions = Vec::new();
    for (_, mut values) in per_sv {
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let first = values.iter().take(120).map(|(_, v)| *v).collect::<Vec<_>>();
        if first.len() < 60 {
            continue;
        }
        let m = median(first.clone());
        let dispersion = median(first.iter().map(|v| (v - m).abs()).collect());
        dispersions.push(dispersion);
    }

    let gf_dispersion = median(dispersions);
    println!(
        "GF code - GF phase: median dispersion {:.3} m",
        gf_dispersion
    );
    assert!(gf_dispersion < 0.5, "{}", gf_dispersion);

    // multipath arcs: statistics of every arc are emitted once
    assert!(!results.arcs.is_empty());
    let mut arcs = results
        .arcs
        .iter()
        .map(|a| {
            (
                a.receiver.to_string(),
                a.sv,
                a.code.clone(),
                a.phase.clone(),
                a.arc,
            )
        })
        .collect::<Vec<_>>();
    let total = arcs.len();
    arcs.sort();
    arcs.dedup();
    assert_eq!(arcs.len(), total, "duplicated arcs");

    let mut samples = HashMap::<(SV, String, String, u32), usize>::new();
    for (_, c) in results.combinations.iter() {
        if c.kind == QcCombinationKind::Multipath {
            *samples
                .entry((c.sv, c.lhs.to_string(), c.rhs.to_string(), c.arc.unwrap()))
                .or_default() += 1;
        }
    }

    for arc in results.arcs.iter() {
        let key = (arc.sv, arc.code.to_string(), arc.phase.to_string(), arc.arc);
        assert_eq!(samples.get(&key).copied(), Some(arc.samples), "{:?}", key);
        assert!(arc.end >= arc.start);
        assert!(arc.rms_m >= 0.0);
    }
    assert_eq!(samples.len(), results.arcs.len());

    // typical code multipath + noise of a geodetic receiver: decimeters
    for (code, phase, low, high) in [
        ("C1C", "L2W", 0.05, 1.0),
        ("C2W", "L1C", 0.05, 1.0),
        ("C5Q", "L1C", 0.02, 1.0),
    ] {
        let rms = results
            .arcs
            .iter()
            .filter(|a| {
                a.sv.constellation == Constellation::GPS
                    && a.code.to_string() == code
                    && a.phase.to_string() == phase
                    && a.samples > 60
            })
            .map(|a| a.rms_m)
            .collect::<Vec<_>>();

        let rms = median(rms);
        println!("GPS MP {} ({}): median RMS {:.3} m", code, phase, rms);
        assert!(rms > low && rms < high, "{}", rms);
    }
}

#[test]
fn combination_selection() {
    // phase GF only, Galileo only
    let results = run(QcCombinationConfig {
        code: false,
        ..Default::default()
    }
    .with_combinations(&[QcCombinationKind::GeometryFree])
    .with_constellations(&[Constellation::Galileo]));

    assert!(!results.combinations.is_empty());
    assert!(results.arcs.is_empty());

    for (_, c) in results.combinations.iter() {
        assert_eq!(c.kind, QcCombinationKind::GeometryFree);
        assert_eq!(c.sv.constellation, Constellation::Galileo);
        assert!(!c.is_code());
        assert!(c.name().starts_with("GF L"), "{}", c.name());
    }

    // unknown receiver: nothing
    let results = run(QcCombinationConfig::default().with_receiver(receiver("UNKNOWN")));
    assert!(results.combinations.is_empty());
}
