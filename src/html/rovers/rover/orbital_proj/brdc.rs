use crate::{
    prelude::{html, Marker, Markup, Render, Plot},
    context::{QcContext, QcIndexing},
    report::{AxisSelector, StringSelector},
};

pub struct Projection {
    plot_3d: Plot, 
    plot_2d: Plot, 
    skyplot: Plot,
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            plot_2d: Plot::timedomain_plot("brdc-proj", "BRDC Projection", "Coordinates [km]", true),
            plot_3d: Plot::plot_3d("brdc-proj", "BRDC (3D) Projection", "x [km]", "y [km]", "z [km]", true),
            skyplot: Plot::sky_plot("brdc-proj", "Skyplot", true),
        }
    }
}

impl Projection {
    pub fn new(ctx: &QcContext, rover: &QcIndexing) -> Self {



    }
}

impl Render for Projection {
    fn render(&self) -> Markup {
        html! {

        }
    }
}