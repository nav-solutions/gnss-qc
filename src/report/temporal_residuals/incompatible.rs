use crate::{
    context::QcContext,
    plot::Plot,
    prelude::{html, Markup, Render},
};

pub struct OrbitResidualProjects {
    pub not_empty: bool,
}

impl OrbitResidualProjects {
    pub fn new(context: &QcContext) -> Self {
        Self { not_empty: false }
    }
}

impl Render for OrbitResidualProjects {
    fn render(&self) -> Markup {
        html! {}
    }
}
