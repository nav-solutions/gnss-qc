//! Signal observations to GNSS-RTK candidates
use std::collections::BTreeMap;

use gnss_rtk::prelude::{Candidate, Carrier, Constellation, Epoch, Observation, SV};

use rinex::prelude::{obs::SignalObservation, Observable};

/// Tracking attributes preference (lower index preferred): civilian codes first,
/// then semi-codeless, then pilot/data combinations.
const ATTRIBUTES_PRIORITY: &str = "CWPQXLSIYMDZBAN";

fn attribute_rank(attribute: char) -> usize {
    ATTRIBUTES_PRIORITY
        .find(attribute)
        .unwrap_or(ATTRIBUTES_PRIORITY.len())
}

/// GNSS-RTK carrier of this observable, for this constellation
pub fn rtk_carrier(observable: &Observable, constellation: Constellation) -> Option<Carrier> {
    let carrier = observable.to_carrier(constellation).ok()?;
    let khz = (carrier.frequency_mega_hz() * 1.0E3).round() as u32;

    match khz {
        1_575_420 => Some(Carrier::L1),
        1_227_600 => Some(Carrier::L2),
        1_176_450 => Some(Carrier::L5),
        1_207_140 => Some(Carrier::E5b),
        1_191_795 => Some(Carrier::E5a5b),
        1_561_098 => Some(Carrier::B1),
        1_268_520 => Some(Carrier::B3),
        _ => None,
    }
}

#[derive(Default)]
struct CarrierSignals {
    /// (attribute rank, attribute, value)
    pseudo_range: Option<(usize, char, f64)>,
    phase_cycles: Vec<(char, f64)>,
    snr: Vec<(char, f64)>,
}

/// Primary signal carrier of each constellation (single frequency navigation):
/// GPS / QZSS L1, Galileo E1, BeiDou B1I. The broadcast group delays apply to these signals.
pub fn is_primary_carrier(constellation: Constellation, carrier: Carrier) -> bool {
    match constellation {
        Constellation::BeiDou => carrier == Carrier::B1,
        _ => carrier == Carrier::L1,
    }
}

/// Builds the GNSS-RTK [Candidate]s from the signals sampled at `t`,
/// for the selected constellations. Satellites without pseudo range are dropped.
/// When `primary_only` is set (single frequency navigation), only the primary signal
/// of each constellation is proposed (see [is_primary_carrier]).
pub fn build_candidates(
    t: Epoch,
    signals: &[SignalObservation],
    constellations: &[Constellation],
    primary_only: bool,
    candidates: &mut Vec<Candidate>,
) {
    candidates.clear();

    let mut sorted: BTreeMap<SV, BTreeMap<Carrier, CarrierSignals>> = BTreeMap::new();

    for signal in signals.iter() {
        if !constellations.contains(&signal.sv.constellation) {
            continue;
        }

        let carrier = match rtk_carrier(&signal.observable, signal.sv.constellation) {
            Some(carrier) => carrier,
            None => continue,
        };

        if primary_only && !is_primary_carrier(signal.sv.constellation, carrier) {
            continue;
        }

        let attribute = match signal.observable.code() {
            Some(code) => code.chars().nth(1).unwrap_or(' '),
            None => continue,
        };

        let entry = sorted
            .entry(signal.sv)
            .or_default()
            .entry(carrier)
            .or_default();

        match &signal.observable {
            Observable::PseudoRange(_) => {
                let rank = attribute_rank(attribute);
                if entry.pseudo_range.is_none_or(|(r, _, _)| rank < r) {
                    entry.pseudo_range = Some((rank, attribute, signal.value));
                }
            }
            Observable::PhaseRange(_) => entry.phase_cycles.push((attribute, signal.value)),
            Observable::SSI(_) => entry.snr.push((attribute, signal.value)),
            _ => {}
        }
    }

    for (sv, carriers) in sorted {
        // GNSS-RTK requires an L1 (or B1) pseudo range: it is the reference
        // signal of each candidate (bias compensation, single frequency..).
        // Without it, the satellite cannot be proposed (incomplete stream, signal loss..).
        let has_l1 = carriers.iter().any(|(carrier, signals)| {
            matches!(carrier, Carrier::L1 | Carrier::B1) && signals.pseudo_range.is_some()
        });

        if !has_l1 {
            continue;
        }

        let observations = carriers
            .into_iter()
            .filter_map(|(carrier, signals)| {
                let (_, attribute, range_m) = signals.pseudo_range?;

                // same tracking mode preferred
                let snr = signals
                    .snr
                    .iter()
                    .find(|(a, _)| *a == attribute)
                    .or(signals.snr.first())
                    .map(|(_, snr)| *snr);

                let mut observation = Observation::pseudo_range(carrier, range_m, snr);

                if let Some((_, cycles)) = signals
                    .phase_cycles
                    .iter()
                    .find(|(a, _)| *a == attribute)
                    .or(signals.phase_cycles.first())
                {
                    observation =
                        observation.with_ambiguous_phase_range_m(cycles * carrier.wavelength());
                }

                Some(observation)
            })
            .collect::<Vec<_>>();

        if !observations.is_empty() {
            candidates.push(Candidate::new(sv, t, observations));
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    fn signal(sv: &str, observable: Observable, value: f64) -> SignalObservation {
        SignalObservation {
            sv: SV::from_str(sv).unwrap(),
            value,
            observable,
            lli: None,
            snr: None,
        }
    }

    #[test]
    fn candidates_require_l1_pseudo_ranges() {
        let t = Epoch::from_str("2020-06-25T00:00:00 GPST").unwrap();

        let signals = [
            // complete
            signal("G01", Observable::PseudoRange("C1C".to_string()), 2.0E7),
            signal("G01", Observable::PseudoRange("C2W".to_string()), 2.0E7),
            // no L1 pseudo range (incomplete stream): not proposed
            signal("G02", Observable::PseudoRange("C5Q".to_string()), 2.1E7),
            signal("G02", Observable::PhaseRange("L1C".to_string()), 1.1E8),
            // BeiDou B1I
            signal("C10", Observable::PseudoRange("C2I".to_string()), 3.7E7),
        ];

        let mut candidates = Vec::new();

        build_candidates(
            t,
            &signals,
            &[Constellation::GPS, Constellation::BeiDou],
            false,
            &mut candidates,
        );

        let svs = candidates.iter().map(|cd| cd.sv).collect::<Vec<_>>();
        assert_eq!(
            svs,
            vec![SV::from_str("G01").unwrap(), SV::from_str("C10").unwrap()]
        );
    }
}
