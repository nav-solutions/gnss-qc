use crate::prelude::{html, Constellation, Markup, Render};
use std::fmt::Display;

pub type ConstellationSelector = Selector<Constellation>;

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
        let mut selector = Selector::new(html_id, true);

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
        let mut selector = Selector::new(html_id, true);

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
    allows_all: bool,
    inner: Vec<T>,
}

impl<T: Default + Clone + PartialEq + Display> Selector<T> {
    pub fn new(html_id: &str, allows_all: bool) -> Self {
        Self {
            allows_all,
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

impl<T: Default + Clone + Display> Render for Selector<T> {
    fn render(&self) -> Markup {
        html! {
            form class="radio-group" id=(self.html_id) {
                @ if self.allows_all {
                    @ if self.inner.len() == 2 {
                        label class="radio-option" {
                            input type="radio" name=(self.html_id) value="both" checked {}
                            span {
                                "Both"
                            }
                        }
                    } @ else {
                        label class="radio-option" {
                            input type="radio" name=(self.html_id) value="both" checked {}
                            span {
                                "All"
                            }
                        }
                    }
                }
                @ for (index, item) in self.inner.iter().enumerate() {
                    @ if index == 0 {
                        @ if self.allows_all {
                            label class="radio-option" {
                                input type="radio" name=(self.html_id) value=(item) {}
                                span {
                                    (item.to_string())
                                }
                            }
                        } @ else {
                            label class="radio-option" {
                                input type="radio" name=(self.html_id) value=(item) checked {}
                                span {
                                    (item.to_string())
                                }
                            }
                        }
                    } @ else {
                        label class="radio-option" {
                            input type="radio" name=(self.html_id) value=(item) {}
                            span {
                                (item.to_string())
                            }
                        }
                    }
                }
            }
        }
    }
}
