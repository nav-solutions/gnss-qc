use crate::{
    context::QcContext,
    plot::Plot,
    prelude::{Constellation, Epoch, MarkerSymbol, Mode, SV},
    report::{AxisSelector, ConstellationSelector, PosVelSelector},
};

use itertools::Itertools;
use maud::{html, Markup, PreEscaped, Render};

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
    axis_sel: AxisSelector,
    pos_vel_sel: PosVelSelector,
    constellation_sel: ConstellationSelector,
    position_plots: HashMap<Constellation, Plot>,
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            position_plots: Default::default(),
            axis_sel: AxisSelector::new("orbit-residuals"),
            pos_vel_sel: PosVelSelector::new("orbit-residuals"),
            constellation_sel: ConstellationSelector::new("orbit-residuals", true),
        }
    }
}

impl Projection {
    pub fn new(context: &QcContext) -> Self {
        let pos_vel_selector = PosVelSelector::new("orbit-residual-posvel");

        let mut constell_selector =
            ConstellationSelector::new("orbit-residuals-constellation", true);

        let mut pos_plots = HashMap::<Constellation, Plot>::new();

        if let Some(sp3) = &context.sp3 {
            if let Some(brdc) = &context.brdc_navigation {
                let mut err_epochs = HashMap::<SV, Vec<Epoch>>::new();

                let mut x_err_data = HashMap::<SV, Vec<f64>>::new();
                let mut y_err_data = HashMap::<SV, Vec<f64>>::new();
                let mut z_err_data = HashMap::<SV, Vec<f64>>::new();

                let mut iter = if sp3.has_satellite_velocity() {
                    SatellitesPosVelIter::PositionVelocity(sp3.satellites_pos_vel_km_iter())
                } else {
                    SatellitesPosVelIter::PositionOnly(sp3.satellites_position_km_iter())
                };

                while let Some((t, sv, pos_km, _)) = iter.next() {
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

                            if pos_plots.get(&sv.constellation).is_none() {
                                pos_plots.insert(
                                    sv.constellation,
                                    Plot::plot_3d(
                                        "position-residuals",
                                        "SP3/BRDC Positions Residuals",
                                        "Error [m]",
                                        "Error [m]",
                                        "Error [m]",
                                        true,
                                    ),
                                );
                            }

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
                        }
                    }
                }

                for (constellation, plots) in pos_plots.iter_mut() {
                    for (index, (sv, epochs)) in err_epochs.iter().enumerate() {
                        if sv.constellation != *constellation {
                            continue;
                        }

                        let x_err_m = x_err_data.get(&sv).unwrap();
                        let y_err_m = y_err_data.get(&sv).unwrap();
                        let z_err_m = z_err_data.get(&sv).unwrap();

                        let trace = Plot::chart_3d(
                            &format!("{}", sv),
                            Mode::Markers,
                            MarkerSymbol::Diamond,
                            epochs,
                            x_err_m.to_vec(),
                            y_err_m.to_vec(),
                            z_err_m.to_vec(),
                            true,
                        );

                        plots.add_trace(trace);
                    }
                }
            }
        }

        Self {
            position_plots: pos_plots,
            pos_vel_sel: pos_vel_selector,
            constellation_sel: constell_selector,
            axis_sel: AxisSelector::new("orbit-proj"),
        }
    }

    pub fn has_content(&self) -> bool {
        self.constellation_sel.has_content()
    }

    fn javascript(&self) -> &str {
        "
        const constell_sel = document.getElementById('orbit-residuals-constell');
        const position_residuals_plots = document.querySelectorAll('.orbit-position-residuals');

        // constellation listener
        constell_sel.addEventListener('change', (event) => {
            console.log('selected constell: ' + event.target.value);

            if (event.target.value == 'All' || event.target.value == 'Both') {
                position_residuals_plots.forEach(plot => {
                    plot.style = 'display: block';
                });
            }
        });
        "
    }
}

impl Render for Projection {
    fn render(&self) -> Markup {
        html! {

            // pos/vel selector
            div id="orbit-residuals-posvel" {
                p {
                    (self.pos_vel_sel.render())
                }
            }

            // axis selector
            div id="orbit-residuals-axis" {
                p {
                    (self.axis_sel.render())
                }
            }

            // constellations selector
            div id="orbit-residuals-constell" {
                p {
                    (self.constellation_sel.render())
                }
            }

            // positions
            @ for (index, constellation) in self.position_plots.keys().sorted().enumerate() {
                @ if let Some(plot) = self.position_plots.get(&constellation) {
                    div id="{}-orbit-position-residuals" class="data orbit-position-residuals" style="display: block" {
                        p {
                            (plot.render())
                        }
                    }
                }
            }


            script {
                (PreEscaped(self.javascript()))
            }

        }
    }
}
