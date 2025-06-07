use plotters::coord::ranged1d::AsRangedCoord;
use plotters::prelude::*;

// use plotters::coord::ranged1d::Ranged;

use plotters::style::Color;

use image::DynamicImage;
use image::ImageBuffer;

use crate::prelude::Epoch;

#[derive(Debug, Clone, Default)]
pub struct TemporalData {
    size: usize,
    pub data: Vec<f64>,
    pub epochs: Vec<Epoch>,
}

impl TemporalData {
    pub fn first_epoch(&self) -> Epoch {
        self.epochs[0]
    }

    pub fn last_epoch(&self) -> Epoch {
        self.epochs[self.size - 1]
    }

    fn xmin(&self) -> f64 {
        self.data[0]
    }

    fn xmax(&self) -> f64 {
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

    /// ## Input
    /// - x: data that implements [AsRangedCoord]
    /// - y: data that implements [AsRangedCoord]
    /// - plot_title: readable [str]
    /// - width: plot width in pxl (as [u32])
    /// - height: plot height in pxl (as [u32])
    /// - bg_color: [Color] implementation
    pub fn to_cartesian_2d<C: Color>(
        &self,
        plot_title: &str,
        width: u32,
        height: u32,
        bg_color: C,
    ) -> DynamicImage {
        let mut buffer = ImageBuffer::new(width, height);

        {
            let mut backend =
                BitMapBackend::with_buffer(&mut buffer, (width, height)).into_drawing_area();

            backend.fill(&bg_color).unwrap();

            let x_spec = self.first_epoch().to_mjd_utc_days()..self.last_epoch().to_mjd_utc_days();
            let y_spec = self.xmin()..self.xmax();

            let mut chart = ChartBuilder::on(&backend)
                .margin(10)
                .caption(plot_title, ("sans-serif", 30))
                .x_label_area_size(30)
                .y_label_area_size(30)
                .build_cartesian_2d(x_spec, y_spec)
                .unwrap();

            chart.draw_series(LineSeries::new(
                self.epochs
                    .iter()
                    .map(|t| t.to_utc_days())
                    .enumerate()
                    .map(|(index, x)| (x, self.data[index])),
                &RED,
            ));

            backend.present().unwrap();
        }

        DynamicImage::ImageRgb8(buffer)
    }
}
