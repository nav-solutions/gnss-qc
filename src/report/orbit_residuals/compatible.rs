use crate::{
    context::QcContext,
    plot::Plot,
    prelude::{html, Epoch, MarkerSymbol, Markup, Mode, Render, SV},
    report::{selector::PosVel, AxisSelector, ConstellationSelector, PosVelSelector},
};

use log::error;
use std::collections::HashMap;

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

pub struct Projection {
    position_plot: Plot,
    velocity_plot: Plot,
    axis_sel: AxisSelector,
    pos_vel_sel: PosVelSelector,
    constellation_sel: ConstellationSelector,
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            position_plot: {
                Plot::timedomain_plot("position-residuals", "BRDC/SP3 Residual", "Error [m]", true)
            },
            velocity_plot: {
                Plot::timedomain_plot("velocity-residuals", "BRDC/SP3 Residual", "Error [m]", true)
            },
            axis_sel: AxisSelector::new("orbit-residuals"),
            pos_vel_sel: PosVelSelector::new("orbit-residuals"),
            constellation_sel: ConstellationSelector::new("orbit-residuals"),
        }
    }
}

impl Projection {
    pub fn new(context: &QcContext) -> Self {
        let pos_vel_selector = PosVelSelector::new("orbit-residual-posvel");

        let mut constell_selector = ConstellationSelector::new("orbit-residuals-constellation");

        let mut pos_plot = Plot::timedomain_plot(
            "position-residuals",
            "SP3/BRDC Residuals",
            "Error [m]",
            true,
        );

        let mut vel_plot = Plot::timedomain_plot(
            "velocity-residuals",
            "SP3/BRDC Residuals",
            "Error [m]",
            true,
        );

        if let Some(sp3) = &context.sp3 {
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
                            constell_selector.add(&sv.constellation);

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

                    pos_plot.add_trace(trace);

                    let trace = Plot::timedomain_chart(
                        &format!("{}(y)", sv),
                        Mode::Markers,
                        MarkerSymbol::Diamond,
                        epochs,
                        y_err_m.to_vec(),
                        index == 0,
                    );

                    pos_plot.add_trace(trace);

                    let trace = Plot::timedomain_chart(
                        &format!("{}(z)", sv),
                        Mode::Markers,
                        MarkerSymbol::Diamond,
                        epochs,
                        z_err_m.to_vec(),
                        index == 0,
                    );

                    pos_plot.add_trace(trace);

                    if let Some(velx_err_m) = velx_err_data.get(&sv) {
                        let trace = Plot::timedomain_chart(
                            &format!("{}(velx)", sv),
                            Mode::Markers,
                            MarkerSymbol::Diamond,
                            epochs,
                            velx_err_m.to_vec(),
                            index == 0,
                        );

                        vel_plot.add_trace(trace);
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

                        vel_plot.add_trace(trace);
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

                        vel_plot.add_trace(trace);
                    }
                }
            }
        }

        Self {
            position_plot: pos_plot,
            velocity_plot: vel_plot,
            pos_vel_sel: pos_vel_selector,
            constellation_sel: constell_selector,
            axis_sel: AxisSelector::new("orbit-proj"),
        }
    }

    pub fn has_content(&self) -> bool {
        self.constellation_sel.has_content()
    }
}

impl Render for Projection {
    fn render(&self) -> Markup {
        html! {
            // pos/vel selector
            div class="data-sel filter" name="constellation" {
                (self.pos_vel_sel.render())
            }

            // axis selector
            div class="data-sel filter" name="axis" {
                (self.axis_sel.render())
            }

            // constellations selector
            div class="data-sel filter" name="constellation" {
                (self.constellation_sel.render())
            }

            // residuals
            div class="data" id="position-residuals" style="display: none" {
                (self.position_plot.render())
            }

            // residuals
            div class="data" id="velocity-residuals" style="display: none" {
                (self.velocity_plot.render())
            }
        }
    }
}
