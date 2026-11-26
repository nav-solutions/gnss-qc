//! TEC estimated from signal observations: a network of two receivers,
//! ESBC00DNK and MOJN00DNK (65 km apart), 2020-06-25, broadcast ephemerides.
use std::collections::{BTreeMap, HashMap};

use crate::{prelude::*, tests::miso::load_products};

const ESBC_OBS: &str = "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz";
const MOJN_OBS: &str = "CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz";
const ESBC_NAV: &str = "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";

struct Results {
    estimates: Vec<(Epoch, QcTecEstimate)>,
    arcs: Vec<QcTecArc>,
}

fn run(cfg: QcObsTecConfig) -> Results {
    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, MOJN_OBS, ESBC_NAV]);

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcObsTecProcessor::new(cfg))
        .build()
        .unwrap();

    let (mut estimates, mut arcs) = (Vec::new(), Vec::new());
    let mut last = None;

    for item in engine {
        let item = item.unwrap();

        // time ordered outputs
        if let Some(last) = last {
            assert!(item.epoch >= last);
        }
        last = Some(item.epoch);

        match item.payload {
            QcOutput::TecEstimate(estimate) => estimates.push((item.epoch, *estimate)),
            QcOutput::TecArc(arc) => arcs.push(*arc),
            _ => {}
        }
    }

    Results { estimates, arcs }
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    values[values.len() / 2]
}

#[test]
fn network_tec() {
    let results = run(QcObsTecConfig::default().with_constellations(&[Constellation::GPS]));

    let receivers = results
        .estimates
        .iter()
        .map(|(_, e)| e.receiver.to_string())
        .collect::<std::collections::BTreeSet<_>>();

    // both receivers of the network contribute
    assert_eq!(receivers.len(), 2, "{:?}", receivers);
    assert!(
        results.estimates.len() > 40_000,
        "{}",
        results.estimates.len()
    );

    for (_, e) in results.estimates.iter() {
        assert_eq!(e.sv.constellation, Constellation::GPS);

        // L1 / L2, precise codes first
        assert_eq!(e.code_1.to_string(), "C1W", "{:?}", e);
        assert_eq!(e.code_j.to_string(), "C2W", "{:?}", e);
        assert_eq!(e.phase_1.to_string(), "L1C");
        assert_eq!(e.phase_j.to_string(), "L2W");

        // elevation mask, single layer model (450 km)
        assert!(e.elevation_deg >= 15.0);
        assert_eq!(e.altitude_km, 450.0);
        assert!(
            e.obliquity_factor >= 1.0 && e.obliquity_factor < 2.8,
            "{}",
            e.obliquity_factor
        );

        // pierce points around Denmark (55°N, 8.5°E): within 15° at 15° of elevation
        assert!((e.latitude_deg - 55.2).abs() < 15.0, "{}", e.latitude_deg);
        assert!((e.longitude_deg - 8.6).abs() < 25.0, "{}", e.longitude_deg);

        // formulas
        assert!((e.slant_tecu - (e.phase_slant_tecu + e.leveling_tecu)).abs() < 1.0E-9);
        assert!((e.vertical_tecu - e.slant_tecu / e.obliquity_factor).abs() < 1.0E-9);
        assert!(e.map_vertical_tecu.is_none(), "no TEC map in the inputs");
    }

    // arcs: every arc is closed once, with its exact leveling
    let mut last_of_arc = HashMap::<(String, SV, u32), &QcTecEstimate>::new();
    let mut samples = HashMap::<(String, SV, u32), usize>::new();

    for (_, e) in results.estimates.iter() {
        let key = (e.receiver.to_string(), e.sv, e.arc);
        last_of_arc.insert(key.clone(), e);
        *samples.entry(key).or_default() += 1;
    }

    assert_eq!(results.arcs.len(), last_of_arc.len());

    for receiver in receivers.iter() {
        let vertical = results
            .estimates
            .iter()
            .filter(|(_, e)| &e.receiver.to_string() == receiver)
            .map(|(_, e)| e.vertical_tecu)
            .collect::<Vec<_>>();
        println!(
            "{}: {} estimates, median VTEC {:.2} TECu (uncalibrated)",
            receiver,
            vertical.len(),
            median(vertical.clone())
        );
    }

    for arc in results.arcs.iter() {
        let key = (arc.receiver.to_string(), arc.sv, arc.arc);
        assert_eq!(samples[&key], arc.samples);

        // the running leveling of the last sample is the final leveling of the arc
        let last = last_of_arc[&key];
        assert_eq!(last.arc_samples, arc.samples);
        assert!((last.leveling_tecu - arc.leveling_tecu).abs() < 1.0E-6);
        assert!(arc.end >= arc.start);
    }

    // code noise of the leveling: a few TECu (code noise x 9.5 TECu/m for L1/L2)
    let noise = median(
        results
            .arcs
            .iter()
            .filter(|a| a.samples > 60)
            .map(|a| a.code_noise_tecu)
            .collect(),
    );
    println!("median code noise: {:.2} TECu", noise);
    assert!(noise > 0.2 && noise < 6.0, "{}", noise);

    // exact leveling (final mean of each arc): the slant TEC of one satellite seen by both
    // receivers (65 km apart, pierce points ~50 km apart) varies alike: their difference is
    // constant (differential code biases of the receivers) within a few TECu
    let leveling = results
        .arcs
        .iter()
        .map(|a| ((a.receiver.to_string(), a.sv, a.arc), a.leveling_tecu))
        .collect::<HashMap<_, _>>();

    let mut slant = BTreeMap::<(SV, Epoch), HashMap<String, f64>>::new();
    for (t, e) in results.estimates.iter() {
        let key = (e.receiver.to_string(), e.sv, e.arc);
        slant
            .entry((e.sv, *t))
            .or_default()
            .insert(e.receiver.to_string(), e.phase_slant_tecu + leveling[&key]);
    }

    let mut differences = BTreeMap::<SV, Vec<f64>>::new();
    for ((sv, _), values) in slant.iter() {
        if values.len() == 2 {
            let values = values.values().collect::<Vec<_>>();
            differences
                .entry(*sv)
                .or_default()
                .push(values[0] - values[1]);
        }
    }

    let mut dispersions = Vec::new();
    for (sv, values) in differences.iter() {
        if values.len() < 200 {
            continue;
        }
        let m = median(values.clone());
        let dispersion = median(values.iter().map(|v| (v - m).abs()).collect());
        println!(
            "{}: ESBC - MOJN slant TEC {:.2} ± {:.2} TECu",
            sv, m, dispersion
        );
        dispersions.push(dispersion);
    }

    let dispersion = median(dispersions);
    println!("ESBC - MOJN: median dispersion {:.2} TECu", dispersion);
    assert!(dispersion < 2.0, "{}", dispersion);
}

#[test]
fn obs_tec_selection() {
    let esbc = QcIndexing::Receiver("SEPT POLARX5 #3047937".to_string());

    let results = run(QcObsTecConfig {
        elevation_mask_deg: 30.0,
        ..QcObsTecConfig::default()
            .with_receiver(esbc.clone())
            .with_constellations(&[Constellation::Galileo])
    });

    assert!(!results.estimates.is_empty());

    for (_, e) in results.estimates.iter() {
        assert_eq!(e.receiver, esbc);
        assert_eq!(e.sv.constellation, Constellation::Galileo);
        assert!(e.elevation_deg >= 30.0);
        // E1 / E5a
        assert_eq!(e.code_1.to_string(), "C1C", "{:?}", e);
        assert_eq!(e.code_j.to_string(), "C5Q", "{:?}", e);
    }
}
