use crate::{
    context::QcContext,
    plot::Plot,
    prelude::{html, Constellation, Epoch, MarkerSymbol, Markup, Mode, Render, SV},
    report::{AxisSelector, ConstellationSelector},
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
    #[cfg(feature = "sp3")]
    residuals: Plot,
    axis_sel: AxisSelector,
    constellation_sel: ConstellationSelector,
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
            residuals: {
                Plot::timedomain_plot("orbit-residual", "BRDC/SP3 Residual", "Error [km]", true)
            },
            axis_sel: AxisSelector::new("orbit-proj"),
            constellation_sel: ConstellationSelector::new("orbit-proj"),
        }
    }
}

impl OrbitalProjections {
    pub fn new(context: &QcContext) -> Self {

        let mut constell_selector = ConstellationSelector::new("orbit-proj");

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

            let mut sp3_epochs = HashMap::<SV, Vec<Epoch>>::new();

            let mut sp3_x_data = HashMap::<SV, Vec<f64>>::new();
            let mut sp3_y_data = HashMap::<SV, Vec<f64>>::new();
            let mut sp3_z_data = HashMap::<SV, Vec<f64>>::new();
            
            let mut sp3_velx_data = HashMap::<SV, Vec<f64>>::new();
            let mut sp3_vely_data = HashMap::<SV, Vec<f64>>::new();
            let mut sp3_velz_data = HashMap::<SV, Vec<f64>>::new();

            if let Some(brdc) = &context.brdc_navigation {

                let mut err_epochs = HashMap::<SV, Vec<Epoch>>::new();

                let mut x_err_data = HashMap::<SV, Vec<f64>>::new();
                let mut y_err_data = HashMap::<SV, Vec<f64>>::new();
                let mut z_err_data = HashMap::<SV, Vec<f64>>::new();

                let mut velx_err_data = HashMap::<SV, Vec<f64>>::new();
                let mut vely_err_data = HashMap::<SV, Vec<f64>>::new();
                let mut velz_err_data = HashMap::<SV, Vec<f64>>::new();

                let mut iter = if sp3.has_satellite_velocity() {
                    SatellitesPosVelIter::PositionVelocity(sp3.satellites_pos_vel_km_iter())
                } else {
                    SatellitesPosVelIter::PositionOnly(sp3.satellites_position_km_iter())
                };

                while let Some((t, sv, pos_km, vel_km)) = iter.next() {

                    constell_selector.add(&sv.constellation);

                    if let Some(epochs) = sp3_epochs.get_mut(&sv) {
                        epochs.push(t);
                    } else {
                        sp3_epochs.insert(sv, vec![t]);
                    }

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
                            if let Some(epochs) = err_epochs.get_mut(&sv) {
                                epochs.push(t);
                            } else {
                                err_epochs.insert(sv, vec![t]);
                            }

                            let (x_err_m, y_err_m, z_err_m) = (
                                kepler_pos_km[0] * 1e3 - pos_km.0 * 1e3,
                                kepler_pos_km[1] * 1e3 - pos_km.1 * 1e3,
                                kepler_pos_km[2] * 1e3 - pos_km.2 * 1e3,
                            );

                            if let Some(x_err_data) = x_err_data.get_mut(&sv) {
                                x_err_data.push(x_err_m);
                                y_err_data.get_mut(&sv).unwrap().push(y_err_m);
                                z_err_data.get_mut(&sv).unwrap().push(z_err_m);
                            } else {
                                x_err_data.insert(sv, vec![x_err_m]);
                                y_err_data.insert(sv, vec![y_err_m]);
                                z_err_data.insert(sv, vec![z_err_m]);
                            }

                            if let Some(vel_km) = vel_km {
                                let (velx_err_m, vely_err_m, velz_err_m) = (
                                    kepler_vel_km[0] * 1e3 - vel_km.0 * 1e3,
                                    kepler_vel_km[1] * 1e3 - vel_km.1 * 1e3,
                                    kepler_vel_km[2] * 1e3 - vel_km.2 * 1e3,
                                );

                                if x_err_data.get(&sv).is_some() {
                                    velx_err_data.get_mut(&sv).unwrap().push(velx_err_m);
                                    vely_err_data.get_mut(&sv).unwrap().push(vely_err_m);
                                    velz_err_data.get_mut(&sv).unwrap().push(velz_err_m);
                                } else {
                                    velx_err_data.insert(sv, vec![velx_err_m]);
                                    vely_err_data.insert(sv, vec![vely_err_m]);
                                    velz_err_data.insert(sv, vec![velz_err_m]);
                                }
                            }
                        }
                    }

                    if let Some(x_data) = sp3_x_data.get_mut(&sv) {
                        x_data.push(pos_km.0);
                        sp3_y_data.get_mut(&sv).unwrap().push(pos_km.1);
                        sp3_z_data.get_mut(&sv).unwrap().push(pos_km.2);

                        if let Some(vel_km) = vel_km {
                            sp3_velx_data.get_mut(&sv).unwrap().push(vel_km.0);
                            sp3_vely_data.get_mut(&sv).unwrap().push(vel_km.1);
                            sp3_velz_data.get_mut(&sv).unwrap().push(vel_km.2);
                        }
                    } else {
                        sp3_x_data.insert(sv, vec![pos_km.0]);
                        sp3_y_data.insert(sv, vec![pos_km.1]);
                        sp3_z_data.insert(sv, vec![pos_km.2]);

                        if let Some(vel_km) = vel_km {
                            sp3_velx_data.insert(sv, vec![vel_km.0]);
                            sp3_vely_data.insert(sv, vec![vel_km.1]);
                            sp3_velz_data.insert(sv, vec![vel_km.2]);
                        }
                    }
                }

                for (index, (sv, epochs)) in sp3_epochs.iter().enumerate() {
                    let x_km = sp3_x_data.get(&sv).unwrap();
                    let y_km = sp3_y_data.get(&sv).unwrap();
                    let z_km = sp3_z_data.get(&sv).unwrap();

                    let trace = Plot::chart_3d(
                        &format!("{}", sv),
                        Mode::Markers,
                        MarkerSymbol::Diamond,
                        epochs,
                        x_km.to_vec(),
                        y_km.to_vec(),
                        z_km.to_vec(),
                        index == 0,
                    );

                    orbits.add_trace(trace);

                }

                for (index, (sv, epochs)) in err_epochs.iter().enumerate() {
                    let x_err_m = x_err_data.get(&sv).unwrap();
                    let y_err_m = y_err_data.get(&sv).unwrap();
                    let z_err_m = z_err_data.get(&sv).unwrap();

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

                    if let Some(velx_err_m) = velx_err_data.get(&sv) {
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

                    if let Some(vely_err_m) = vely_err_data.get(&sv) {
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

                    if let Some(velz_err_m) = velz_err_data.get(&sv) {
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

        Self {
            orbits,
            #[cfg(feature = "sp3")]
            residuals,
            constellation_sel: constell_selector,
            axis_sel: AxisSelector::new("orbit-proj"),
        }
    }

    pub fn has_content(&self) -> bool {
        self.constellation_sel.has_content()
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
            }
            // orbit proj
            div class="section" id="orbits" {
                (self.orbits.render())
            }
        }
    }

    #[cfg(feature = "sp3")]
    fn render(&self) -> Markup {
        html! {
            // one tab to select the projection
            div class="tabs" {
                div class="tab active" data-target="orbits" {
                    "Orbits"
                }
                div class="tab" data-target="residuals" {
                    "Residuals"
                }
            }

            // constellations selector
            div class="data-sel filter" name="constellation" {
                (self.constellation_sel.render())
            }

            // axis selector
            div class="data-sel filter" name="axis" {
                (self.axis_sel.render())
            }

            // orbit proj
            div class="data" id="orbits" style="display: block" {
                (self.orbits.render())
            }
            // residuals
            div class="data" id="residuals" style="display: none" {
                (self.residuals.render())
            }
        }
    }
}
