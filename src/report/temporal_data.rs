use plotters::{
    coord::types::RangedCoordf64,
    prelude::{
        Cartesian2d, ChartContext, Circle, DrawingBackend, EmptyElement, LineSeries, PathElement,
        PointSeries,
    },
    style::{Color, BLACK},
};

use crate::prelude::Epoch;

#[derive(Debug, Clone, Default)]
pub struct TemporalData {
    size: usize,
    pub data: Vec<f64>,
    pub epochs: Vec<Epoch>,
}

impl TemporalData {
    pub fn xmin(&self) -> f64 {
        self.first_epoch().to_utc_days()
    }

    pub fn xmax(&self) -> f64 {
        self.last_epoch().to_utc_days()
    }

    pub fn first_epoch(&self) -> Epoch {
        self.epochs[0]
    }

    pub fn last_epoch(&self) -> Epoch {
        self.epochs[self.size - 1]
    }

    pub fn ymin(&self) -> f64 {
        self.data[0]
    }

    pub fn ymax(&self) -> f64 {
        self.data[self.size - 1]
    }

    pub fn new(t: Epoch, data: f64) -> Self {
        let mut x = Vec::with_capacity(8);
        let mut y = Vec::with_capacity(8);

        x.push(t);
        y.push(data);

        Self {
            size: 1,
            epochs: x,
            data: y,
        }
    }

    pub fn push(&mut self, t: Epoch, data: f64) {
        self.size += 1;
        self.epochs.push(t);
        self.data.push(data);
    }

    pub fn epochs(&self) -> &[Epoch] {
        &self.epochs
    }

    pub fn data(&self) -> &[f64] {
        &self.data
    }

    /// Draws this [TemporalData] on provided graphical backend.
    /// ## Input
    /// - backend: mutable [DrawingArea]
    /// - curve_title: legend as [str]
    /// - curve_color: [Color] implementation
    // pub fn draw<'a, DB: DrawingBackend, C: Color + 'a>(
    pub fn draw<'a, DB: DrawingBackend, C: Color>(
        &self,
        ctx: &mut ChartContext<'a, DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
        curve_title: &str,
        curve_color: C,
        curve_point_size: u32,
    ) {
        ctx.draw_series(PointSeries::of_element(
            self.epochs
                .iter()
                .map(|t| t.to_utc_days())
                .enumerate()
                .map(|(index, x)| (x, self.data[index])),
            curve_point_size,
            &curve_color,
            &|c, s, st| return EmptyElement::at(c) + Circle::new((0, 0), s, st.filled()),
        ))
        .unwrap()
        .label(curve_title)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLACK));
    }
}
