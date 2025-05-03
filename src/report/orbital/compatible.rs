use crate::{
    context::QcContext,
    plot::Plot,
    prelude::{html, Constellation, Epoch, MarkerSymbol, Markup, Mode, Render, SV},
    report::ConstellationsSelector,
};

use log::error;
use std::collections::HashMap;

#[cfg(feature = "sp3")]
enum SatellitesPosVelIter<'a> {
    PositionOnly(Box<dyn Iterator<Item = (Epoch, SV, (f64, f64, f64))> + 'a>),
    PositionVelocity(Box<dyn Iterator<Item = (Epoch, SV, (f64, f64, f64), (f64, f64, f64))> + 'a>),
}

impl<'a> Iterator for SatellitesPosVelIter<'a> {
    type Item = (Epoch, SV, (f64, f64, f64), Option<(f64, f64, f64)>);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::PositionOnly(iter) => {
                let (t, sv, pos_km) = iter.next()?;
                Some((t, sv, pos_km, None))
            }
            Self::PositionVelocity(iter) => {
                let (t, sv, pos_km, vel_km) = iter.next()?;
                Some((t, sv, pos_km, Some(vel_km)))
            }
        }
    }
}

pub struct OrbitalProjections {
    orbits: Plot,
    sky_plot: Plot,
    #[cfg(feature = "sp3")]
    residuals: Plot,
    constellations_sel: ConstellationsSelector,
}

impl Default for OrbitalProjections {
    fn default() -> Self {
        Self {
            orbits: {
                Plot::plot_3d(
                    "orbits",
                    "Orbital Projections",
                    "x [km]",
                    "y [km]",
                    "z [km]",
                    true,
                )
            },
            sky_plot: { Plot::sky_plot("sky-plot", "Sky plot", true) },
            residuals: {
                Plot::timedomain_plot("orbit-residual", "BRDC/SP3 Residual", "Error [km]", true)
            },
            constellations_sel: ConstellationsSelector::new("orbit-proj"),
        }
    }
}

impl OrbitalProjections {
    pub fn new(context: &QcContext) -> Self {
        let mut selector = ConstellationsSelector::new("orbit-proj");

        let mut sky_plot = Plot::sky_plot("sky-plot", "Sky plot", true);

        let mut orbits = Plot::plot_3d(
            "orbits",
            "Orbital Projections",
            "x [km]",
            "y [km]",
            "z [km]",
            true,
        );

        #[cfg(feature = "sp3")]
        let mut residuals =
            Plot::timedomain_plot("residual", "SP3/BRDC Residual", "Error [m]", true);

        #[cfg(feature = "sp3")]
        if let Some(sp3) = &context.sp3 {
            if let Some(brdc) = &context.brdc_navigation {
                let mut epochs = HashMap::<SV, Vec<Epoch>>::new();
                let mut x_data = HashMap::<SV, Vec<f64>>::new();
                let mut y_data = HashMap::<SV, Vec<f64>>::new();
                let mut z_data = HashMap::<SV, Vec<f64>>::new();

                let mut velx_data = HashMap::<SV, Vec<f64>>::new();
                let mut vely_data = HashMap::<SV, Vec<f64>>::new();
                let mut velz_data = HashMap::<SV, Vec<f64>>::new();

                let mut iter = if sp3.has_satellite_velocity() {
                    SatellitesPosVelIter::PositionVelocity(sp3.satellites_pos_vel_km_iter())
                } else {
                    SatellitesPosVelIter::PositionOnly(sp3.satellites_position_km_iter())
                };

                while let Some((t, sv, pos_km, vel_km)) = iter.next() {
                    if let Some((toc, _, ephemeris)) = brdc
                        .nav_ephemeris_frames_iter()
                        .filter_map(|(k, eph)| {
                            if k.sv == sv {
                                if let Some(ts) = k.sv.constellation.timescale() {
                                    if let Some(toe) = eph.toe(ts) {
                                        if eph.is_valid(sv, t, toe) {
                                            Some((k.epoch, toe, eph))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    error!("{}({}) - timescale is not supported!", t, sv);
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .min_by_key(|(_, toe, _)| t - *toe)
                    {
                        if let Some((kepler_pos_km, kepler_vel_km)) =
                            ephemeris.kepler2position_velocity(sv, toc, t)
                        {
                            selector.add_sv(&sv);

                            if let Some(epochs) = epochs.get_mut(&sv) {
                                epochs.push(t);
                            } else {
                                epochs.insert(sv, vec![t]);
                            }

                            let (x_err_m, y_err_m, z_err_m) = (
                                kepler_pos_km[0] * 1e3 - pos_km.0 * 1e3,
                                kepler_pos_km[1] * 1e3 - pos_km.1 * 1e3,
                                kepler_pos_km[2] * 1e3 - pos_km.2 * 1e3,
                            );

                            if let Some(x_data) = x_data.get_mut(&sv) {
                                x_data.push(x_err_m);
                                y_data.get_mut(&sv).unwrap().push(y_err_m);
                                z_data.get_mut(&sv).unwrap().push(z_err_m);
                            } else {
                                x_data.insert(sv, vec![x_err_m]);
                                y_data.insert(sv, vec![y_err_m]);
                                z_data.insert(sv, vec![z_err_m]);
                            }

                            if let Some(vel_km) = vel_km {
                                let (velx_err_m, vely_err_m, velz_err_m) = (
                                    kepler_vel_km[0] * 1e3 - vel_km.0 * 1e3,
                                    kepler_vel_km[1] * 1e3 - vel_km.1 * 1e3,
                                    kepler_vel_km[2] * 1e3 - vel_km.2 * 1e3,
                                );

                                if x_data.get(&sv).is_some() {
                                    velx_data.get_mut(&sv).unwrap().push(velx_err_m);
                                    vely_data.get_mut(&sv).unwrap().push(vely_err_m);
                                    velz_data.get_mut(&sv).unwrap().push(velz_err_m);
                                } else {
                                    velx_data.insert(sv, vec![velx_err_m]);
                                    vely_data.insert(sv, vec![vely_err_m]);
                                    velz_data.insert(sv, vec![velz_err_m]);
                                }
                            }
                        }
                    }
                }

                for (index, (sv, epochs)) in epochs.iter().enumerate() {
                    let x_err_m = x_data.get(&sv).unwrap();
                    let y_err_m = y_data.get(&sv).unwrap();
                    let z_err_m = z_data.get(&sv).unwrap();

                    let trace = Plot::timedomain_chart(
                        &format!("{}(x)", sv),
                        Mode::Markers,
                        MarkerSymbol::Diamond,
                        epochs,
                        x_err_m.to_vec(),
                        index == 0,
                    );

                    residuals.add_trace(trace);

                    let trace = Plot::timedomain_chart(
                        &format!("{}(y)", sv),
                        Mode::Markers,
                        MarkerSymbol::Diamond,
                        epochs,
                        y_err_m.to_vec(),
                        index == 0,
                    );

                    residuals.add_trace(trace);

                    let trace = Plot::timedomain_chart(
                        &format!("{}(z)", sv),
                        Mode::Markers,
                        MarkerSymbol::Diamond,
                        epochs,
                        z_err_m.to_vec(),
                        index == 0,
                    );

                    residuals.add_trace(trace);

                    if let Some(velx_err_m) = velx_data.get(&sv) {
                        let trace = Plot::timedomain_chart(
                            &format!("{}(velx)", sv),
                            Mode::Markers,
                            MarkerSymbol::Diamond,
                            epochs,
                            velx_err_m.to_vec(),
                            index == 0,
                        );

                        residuals.add_trace(trace);
                    }

                    if let Some(vely_err_m) = vely_data.get(&sv) {
                        let trace = Plot::timedomain_chart(
                            &format!("{}(vely)", sv),
                            Mode::Markers,
                            MarkerSymbol::Diamond,
                            epochs,
                            vely_err_m.to_vec(),
                            index == 0,
                        );

                        residuals.add_trace(trace);
                    }

                    if let Some(velz_err_m) = velz_data.get(&sv) {
                        let trace = Plot::timedomain_chart(
                            &format!("{}(velz)", sv),
                            Mode::Markers,
                            MarkerSymbol::Diamond,
                            epochs,
                            velz_err_m.to_vec(),
                            index == 0,
                        );

                        residuals.add_trace(trace);
                    }
                }
            }
        }

        if let Some(brdc) = &context.brdc_navigation {}

        Self {
            orbits,
            sky_plot,
            #[cfg(feature = "sp3")]
            residuals,
            constellations_sel: selector,
        }
    }

    pub fn has_content(&self) -> bool {
        self.constellations_sel.has_content()
    }
}

impl Render for OrbitalProjections {
    #[cfg(not(feature = "sp3"))]
    fn render(&self) -> Markup {
        html! {
            // one tab to select the projection
            div class="tabs" id="orbit-proj" {
                div class="tab active" data-target="orbits" {
                    "Orbits"
                }
                div class="tab" data-target="skyplot" {
                    "Skyplot"
                }
            }
            // orbit proj
            div class="section" id="orbits" {
                (self.orbits.render())
            }
            // skyplot
            div class="section" id="skyplot" {
                (self.sky_plot.render())
            }
        }
    }

    #[cfg(feature = "sp3")]
    fn render(&self) -> Markup {
        html! {
            // one tab to select the projection
            div class="tabs" id="data-sel" {
                div class="tab active" data-target="orbits" {
                    "Orbits"
                }
                div class="tab" data-target="skyplot" {
                    "Skyplot"
                }
                div class="tab" data-target="residuals" {
                    "Residuals"
                }
            }

            // constellations selector
            div id="constell-sel" {
                (self.constellations_sel.render())
            }

            // orbit proj
            div class="data" id="orbits" style="display: block" {
                (self.orbits.render())
            }
            // skyplot
            div class="data" id="skyplot" style="display: none" {
                (self.sky_plot.render())
            }
            // residuals
            div class="data" id="residuals" style="display: none" {
                (self.residuals.render())
            }
        }
    }
}
