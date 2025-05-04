use crate::{
    context::QcContext,
    plot::{MarkerSymbol, Mode, Plot},
    prelude::{Constellation, Epoch, SV},
    report::{ConstellationSelector, PosVelSelector},
};

use itertools::Itertools;
use maud::{html, Markup, PreEscaped, Render};

use log::error;
use std::collections::HashMap;

pub struct Projection {
    plots: HashMap<Constellation, Plot>,
    constellation_sel: ConstellationSelector,
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            plots: Default::default(),
            constellation_sel: ConstellationSelector::new("orbit-residuals", true),
        }
    }
}

impl Projection {
    pub fn new(context: &QcContext) -> Self {
        let mut constell_selector =
            ConstellationSelector::new("orbit-residuals-constellation", true);

        let mut plots = HashMap::<Constellation, Plot>::new();

        if let Some(clock) = &context.precise_clocks {
            if let Some(brdc) = &context.brdc_navigation {
                let mut err_epochs = HashMap::<SV, Vec<Epoch>>::new();
                let mut err_data = HashMap::<SV, Vec<f64>>::new();

                for (t, sv, clock_type, clock_profile) in clock.precise_sv_clock() {
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
                        if let Some(data) = ephemeris.clock_correction(toc, t, sv, 10) {
                            constell_selector.add(&sv.constellation);

                            if plots.get(&sv.constellation).is_none() {
                                plots.insert(
                                    sv.constellation,
                                    Plot::timedomain_plot(
                                        &format!("{}-temporal-residuals-plotly", sv.constellation),
                                        &format!(
                                            "{} Clock/BRDC Offsets Residuals",
                                            sv.constellation
                                        ),
                                        "Error [s]",
                                        true,
                                    ),
                                );
                            }

                            if let Some(epochs) = err_epochs.get_mut(&sv) {
                                epochs.push(t);
                            } else {
                                err_epochs.insert(sv, vec![t]);
                            }

                            let err_s = data.to_seconds() - clock_profile.bias;

                            if let Some(err_data) = err_data.get_mut(&sv) {
                                err_data.push(err_s);
                            } else {
                                err_data.insert(sv, vec![err_s]);
                            }
                        }
                    }
                }

                for (constellation, plots) in plots.iter_mut() {
                    for (sv, epochs) in err_epochs.iter() {
                        if sv.constellation != *constellation {
                            continue;
                        }

                        let err_s = err_data.get(&sv).unwrap();

                        let trace = Plot::timedomain_chart(
                            &format!("{}", sv),
                            Mode::Markers,
                            MarkerSymbol::Diamond,
                            epochs,
                            err_s.to_vec(),
                            true,
                        );

                        plots.add_trace(trace);
                    }
                }
            }
        }

        Self {
            plots,
            constellation_sel: constell_selector,
        }
    }

    pub fn has_content(&self) -> bool {
        self.constellation_sel.has_content()
    }

    fn javascript(&self) -> &str {
        "
        const constell_sel = document.getElementById('temporal-residuals-constell');

        // constellation listener
        constell_sel.addEventListener('change', (event) => {
            console.log('selected constell: ' + event.target.value);

            const plots = document.getElementsByClassName('data temporal-residuals-position-plot');
            console.log('items: ' + plots.length);

            if (event.target.value == 'All' || event.target.value == 'Both') {
                for (let i = 0;  i < position_plots.length; i++) {
                    plots[i].style.display = 'block';
                }
            } else {
                for (let i = 0;  i < plots.length; i++) {
                    if (plots[i].id == event.target.value) {
                        plots[i].style.display = 'block';
                    } else {
                        plots[i].style.display = 'none';
                    }
                }
            }
        });
        "
    }
}

impl Render for Projection {
    fn render(&self) -> Markup {
        html! {

            // constellations selector
            div id="temporal-residuals-constell" {
                p {
                    (self.constellation_sel.render())
                }
            }

            // positions
            @ for constellation in self.plots.keys().sorted() {
                @ if let Some(plot) = self.plots.get(&constellation) {
                    div id=(constellation.to_string()) class="data temporal-residuals-position-plot" style="display: block" {
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
