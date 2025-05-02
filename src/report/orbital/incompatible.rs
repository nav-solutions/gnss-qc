use crate::{
    context::QcContext,
    plot::Plot,
    prelude::{html, Markup, Render},
};

pub struct OrbitalProjections {
    pub not_empty: bool,
}

impl OrbitalProjections {
    pub fn new(context: &QcContext) -> Self {
        Self { not_empty: false }
    }
}

impl Render for OrbitalProjections {
    fn render(&self) -> Markup {
        html! {}
    }
}
