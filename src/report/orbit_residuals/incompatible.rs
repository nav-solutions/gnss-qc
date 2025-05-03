use crate::{
    context::QcContext,
    plot::Plot,
    prelude::{html, Markup, Render},
};

pub struct Projection {}

impl Projection {
    pub fn new(_: &QcContext) -> Self {
        Self {}
    }

    pub fn has_content(&self) -> bool {
        false
    }
}

impl Render for Projection {
    fn render(&self) -> Markup {
        html! {}
    }
}
