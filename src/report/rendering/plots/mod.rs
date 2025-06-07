//! Plots and data visualization helpers

use plotters::coord::ranged1d::AsRangedCoord;
use plotters::prelude::*;

// use plotters::coord::ranged1d::Ranged;

use plotters::style::Color;

use image::DynamicImage;
use image::ImageBuffer;

/// Plot wrapper for easy data visualization
pub struct QcPlot {}

impl QcPlot {
    /// ## Input
    /// - x: data that implements [AsRangedCoord]
    /// - y: data that implements [AsRangedCoord]
    /// - plot_title: readable [str]
    /// - width: plot width in pxl (as [u32])
    /// - height: plot height in pxl (as [u32])
    /// - background_color: [Color]
    pub fn cartesian_2d<X: AsRangedCoord, Y: AsRangedCoord, C: Color>(
        x: X,
        y: Y,
        plot_title: &str,
        width: u32,
        height: u32,
        background_color: C,
    ) -> DynamicImage {
        let mut buffer = ImageBuffer::new(width, height);

        {
            let mut backend =
                BitMapBackend::with_buffer(&mut buffer, (width, height)).into_drawing_area();

            backend.fill(&background_color).unwrap();

            let mut chart = ChartBuilder::on(&backend)
                .margin(10)
                .caption(plot_title, ("sans-serif", 30))
                .x_label_area_size(30)
                .y_label_area_size(30)
                .build_cartesian_2d(x, y)
                .unwrap();

            backend.present().unwrap();
        }

        DynamicImage::ImageRgb8(buffer)
    }

    /// ## Input
    /// - x: data that implements [AsRangedCoord]
    /// - y: data that implements [AsRangedCoord]
    /// - z: data that implements [AsRangedCoord]
    /// - plot_title: readable [str]
    /// - width: plot width in pxl (as [u32])
    /// - height: plot height in pxl (as [u32])
    /// - background_color: [Color]
    pub fn cartesian_3d<X: AsRangedCoord, Y: AsRangedCoord, Z: AsRangedCoord, C: Color>(
        x: X,
        y: Y,
        z: Z,
        plot_title: &str,
        width: u32,
        height: u32,
        background_color: C,
    ) -> DynamicImage {
        let mut buffer = ImageBuffer::new(width, height);

        {
            let mut backend =
                BitMapBackend::with_buffer(&mut buffer, (width, height)).into_drawing_area();

            backend.fill(&background_color).unwrap();

            let mut chart = ChartBuilder::on(&backend)
                .margin(10)
                .caption(plot_title, ("sans-serif", 30))
                .x_label_area_size(30)
                .y_label_area_size(30)
                .build_cartesian_3d(x, y, z)
                .unwrap();

            backend.present().unwrap();
        }

        DynamicImage::ImageRgb8(buffer)
    }
}
