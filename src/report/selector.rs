use crate::prelude::{html, Constellation, Markup, Render};
use std::fmt::Display;

pub type ConstellationSelector = Selector<Constellation>;
pub type StringSelector = Selector<String>;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum Axis {
    #[default]
    X,
    Y,
    Z,
}

impl Display for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X => write!(f, "X"),
            Self::Y => write!(f, "Y"),
            Self::Z => write!(f, "Z"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum PosVel {
    #[default]
    Position,
    Velocity,
}

impl Display for PosVel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Position => write!(f, "Position"),
            Self::Velocity => write!(f, "Velocity"),
        }
    }
}

pub struct PosVelSelector {
    selector: Selector<PosVel>,
}

impl PosVelSelector {
    pub fn new(html_id: &str) -> Self {
        let mut selector = Selector::new(html_id);

        selector.add(&PosVel::Position);
        selector.add(&PosVel::Velocity);

        Self { selector }
    }
}

impl Render for PosVelSelector {
    fn render(&self) -> Markup {
        html! {
            (self.selector.render())
        }
    }
}

pub struct AxisSelector {
    selector: Selector<Axis>,
}

impl AxisSelector {
    pub fn new(html_id: &str) -> Self {
        let mut selector = Selector::new(html_id);

        selector.add(&Axis::X);
        selector.add(&Axis::Y);
        selector.add(&Axis::Z);

        Self { selector }
    }
}

impl Render for AxisSelector {
    fn render(&self) -> Markup {
        html! {
            (self.selector.render())
        }
    }
}

pub struct Selector<T: Default + Clone + Display> {
    html_id: String,
    inner: Vec<T>,
}

impl<T: Default + Clone + PartialEq + Display> Selector<T> {
    pub fn new(html_id: &str) -> Self {
        Self {
            html_id: html_id.to_string(),
            inner: Default::default(),
        }
    }

    pub fn has_content(&self) -> bool {
        !self.inner.is_empty()
    }

    pub fn add(&mut self, item: &T) {
        if !self.inner.contains(&item) {
            self.inner.push(item.clone());
        }
    }
}

impl<T: Default + Copy + Clone + Display> Render for Selector<T> {
    fn render(&self) -> Markup {
        html! {
            div class="tabs" id=(self.html_id) {
                div class="tab active" data-target="all" {
                    ("All")
                }
                @ for item in self.inner.iter() {
                    div class="tab" data-target=(item.to_string()) {
                        (item.to_string())
                    }
                }
            }
        }
    }
}
