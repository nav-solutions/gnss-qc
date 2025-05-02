use crate::{
    context::QcContext,
    plot::Plot,
    prelude::{html, Epoch, MarkerSymbol, Markup, Mode, Render, SV},
};

use std::collections::HashMap;

pub struct OrbitalProjections {
    orbits: Plot,
    sky_plot: Plot,
    #[cfg(feature = "sp3")]
    residuals: Plot,
    pub not_empty: bool,
}

impl Default for OrbitalProjections {
    fn default() -> Self {
        Self {
            not_empty: false,
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
        }
    }
}

impl OrbitalProjections {
    pub fn new(context: &QcContext) -> Self {
        let mut not_empty = false;

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
                not_empty = true;

                let mut epochs = HashMap::<SV, Vec<Epoch>>::new();

                let mut x_data = HashMap::<SV, Vec<f64>>::new();
                let mut y_data = HashMap::<SV, Vec<f64>>::new();
                let mut z_data = HashMap::<SV, Vec<f64>>::new();

                let mut velx_data = HashMap::<SV, Vec<f64>>::new();
                let mut vely_data = HashMap::<SV, Vec<f64>>::new();
                let mut velz_data = HashMap::<SV, Vec<f64>>::new();

                for (t, sv, position_km, velocity_km) in sp3.satellites_pos_vel_km_iter() {
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
                            let (x_err_m, y_err_m, z_err_m, vel_x_err_m, vel_y_err_m, vel_z_err_m) = (
                                kepler_pos_km[0] * 1e3 - position_km.0 * 1e3,
                                kepler_pos_km[1] * 1e3 - position_km.1 * 1e3,
                                kepler_pos_km[2] * 1e3 - position_km.2 * 1e3,
                                kepler_vel_km[0] * 1e3 - velocity_km.0 * 1e3,
                                kepler_vel_km[1] * 1e3 - velocity_km.1 * 1e3,
                                kepler_vel_km[2] * 1e3 - velocity_km.2 * 1e3,
                            );

                            if let Some(epochs) = epochs.get_mut(&sv) {
                                epochs.push(t);
                            } else {
                                epochs.insert(sv, vec![t]);
                            }

                            if let Some(x_data) = x_data.get_mut(&sv) {
                                x_data.push(x_err_m);
                                y_data.get_mut(&sv).unwrap().push(y_err_m);
                                z_data.get_mut(&sv).unwrap().push(z_err_m);
                                velx_data.get_mut(&sv).unwrap().push(vel_x_err_m);
                                vely_data.get_mut(&sv).unwrap().push(vel_y_err_m);
                                velz_data.get_mut(&sv).unwrap().push(vel_z_err_m);
                            } else {
                                x_data.insert(sv, vec![x_err_m]);
                                y_data.insert(sv, vec![y_err_m]);
                                z_data.insert(sv, vec![z_err_m]);
                                velx_data.insert(sv, vec![vel_x_err_m]);
                                vely_data.insert(sv, vec![vel_y_err_m]);
                                velz_data.insert(sv, vec![vel_z_err_m]);
                            }
                        }
                    } else {
                        debug!("{}({}) - no valid ephemeris", t, sv);
                    }
                }

                panic!("{:?}", epochs);

                for (index, (sv, epochs)) in epochs.iter().enumerate() {
                    let x_err_m = x_data.get(&sv).unwrap();
                    let y_err_m = y_data.get(&sv).unwrap();
                    let z_err_m = z_data.get(&sv).unwrap();

                    let velx_err_m = velx_data.get(&sv).unwrap();
                    let vely_err_m = vely_data.get(&sv).unwrap();
                    let velz_err_m = velz_data.get(&sv).unwrap();

                    let trace = Plot::timedomain_chart(
                        &format!("{}(x)", sv),
                        Mode::LinesMarkers,
                        MarkerSymbol::Diamond,
                        epochs,
                        x_err_m.to_vec(),
                        index == 0,
                    );

                    residuals.add_trace(trace);

                    let trace = Plot::timedomain_chart(
                        &format!("{}(y)", sv),
                        Mode::LinesMarkers,
                        MarkerSymbol::Diamond,
                        epochs,
                        y_err_m.to_vec(),
                        index == 0,
                    );

                    residuals.add_trace(trace);

                    let trace = Plot::timedomain_chart(
                        &format!("{}(z)", sv),
                        Mode::LinesMarkers,
                        MarkerSymbol::Diamond,
                        epochs,
                        z_err_m.to_vec(),
                        index == 0,
                    );

                    residuals.add_trace(trace);

                    let trace = Plot::timedomain_chart(
                        &format!("{}(velx)", sv),
                        Mode::LinesMarkers,
                        MarkerSymbol::Diamond,
                        epochs,
                        velx_err_m.to_vec(),
                        index == 0,
                    );

                    residuals.add_trace(trace);

                    let trace = Plot::timedomain_chart(
                        &format!("{}(vely)", sv),
                        Mode::LinesMarkers,
                        MarkerSymbol::Diamond,
                        epochs,
                        vely_err_m.to_vec(),
                        index == 0,
                    );

                    residuals.add_trace(trace);

                    let trace = Plot::timedomain_chart(
                        &format!("{}(velz)", sv),
                        Mode::LinesMarkers,
                        MarkerSymbol::Diamond,
                        epochs,
                        velz_err_m.to_vec(),
                        index == 0,
                    );

                    residuals.add_trace(trace);
                }
            }
        }

        if let Some(brdc) = &context.brdc_navigation {
            not_empty = true;
        }

        Self {
            orbits,
            sky_plot,
            not_empty,
            #[cfg(feature = "sp3")]
            residuals,
        }
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
            div class="tabs" id="orbit-proj" {
                div class="tab active" data-target="orbits" {
                    "Orbits"
                }
                div class="tab" data-target="skyplot" {
                    "Skyplot"
                }
                div class="tab" data-target="residuals" {
                    "SP3/BRDC residuals"
                }
            }

            // orbit proj
            div class="section" id="orbits" style="display: block" {
                (self.orbits.render())
            }
            // skyplot
            div class="section" id="skyplot" style="display: block" {
                (self.sky_plot.render())
            }
            // residuals
            div class="section" id="residuals" style="display: block" {
                (self.residuals.render())
            }
        }
    }
}
