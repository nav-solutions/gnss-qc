use crate::{
    context::{data::QcSourceDescriptor, QcProductType},
    error::QcError,
    processing::analysis::{QcAnalysis, QcAnalysisBuilder},
    report::{
        nav::QcNavReport, observations::QcObservationsReport, orbit_proj::QcOrbitProjections,
        rtk::QcRTKSummary, summaries::QcContextSummary, QcRunReport,
    },
    serializer::data::{QcSerializedItem, QcSerializedPreciseState},
};

mod signals_buf;
mod temporal_data;

use signals_buf::SignalsBuffer;

#[cfg(feature = "navigation")]
use crate::prelude::Frame;

#[cfg(feature = "navigation")]
mod ephemeris_buf;

#[cfg(feature = "navigation")]
use ephemeris_buf::EphemerisBuffer;

#[cfg(feature = "sp3")]
mod precise_states;

#[cfg(feature = "sp3")]
use precise_states::PreciseStateBuffer;

pub struct QcRunner<'a> {
    /// [QcAnalysisBuilder]
    builder: QcAnalysisBuilder,

    /// [QcRunReport] being redacted
    report: &'a mut QcRunReport,

    /// Reference [Frame]
    #[cfg(feature = "navigation")]
    frame: Frame,

    /// [SignalsBuffer] deployed & used if needed
    signals_buffer: Option<SignalsBuffer>,

    /// [EphemerisBuffer] deployed & used if needed
    ephemeris_buffer: Option<EphemerisBuffer>,
}

impl<'a> QcRunner<'a> {
    /// Deploy the [QcRunner]
    pub fn new(
        builder: &QcAnalysisBuilder,
        report: &'a mut QcRunReport,
        frame: Frame,
    ) -> Self {

        let signals_buffer = if builder.needs_signals_buffering() {
            Some(SignalsBuffer::new())
        } else {
            None
        };

        let ephemeris_buffer = if builder.needs_ephemeris_buffering() {
            Some(EphemerisBuffer::new())
        } else {
            None
        };

        Self {
            frame,
            report,
            builder,
            signals_buffer,
            ephemeris_buffer,
        }
    }

    pub fn consume(&mut self, item: QcSerializedItem) {
        match item {
            QcSerializedItem::Ephemeris(item) => {
                
                if self.builder.needs_ephemeris_buffering() {
                    let mut buffer = self.ephemeris_buffer.unwrap();
                    buffer.latch(item);
                }
            }

            QcSerializedItem::RINEXHeader(item) => {

                match item.product_type {
                    QcProductType::Observation => {
                        if self.builder.rtk_summary {
                            if let Some(summary) = &mut self.report.rtk_summary {
                                summary.latch_base_header(item);
                            } else {
                                let mut summary = QcRTKSummary::new(self.frame);
                                summary.latch_base_header(item);
                                self.report.rtk_summary = Some(summary);
                            }
                        }
                    },
                    _ => {},
                }

                if self.builder.rinex_summary {
                    let descriptor = QcSourceDescriptor {
                        indexing: item.indexing,
                        filename: item.filename,
                        product_type: item.product_type,
                    };

                    if let Some(summary) = &mut self.report.summary {
                        summary.latch_rinex(descriptor, item.data);
                    } else {
                        let mut summary = QcContextSummary::new(self.frame);
                        summary.latch_rinex(descriptor, item.data);
                        self.report.summary = Some(summary);
                    }
                }
            }

            #[cfg(feature = "sp3")]
            QcSerializedItem::SP3Header(header) => {
                if self.builder.sp3_summary {
                    let descriptor = QcSourceDescriptor {
                        indexing: header.indexing,
                        filename: header.filename,
                        product_type: header.product_type,
                    };

                    if let Some(summary) = &mut self.report.summary {
                        summary.latch_sp3(descriptor, header.data);
                    } else {
                        let mut summary = QcContextSummary::new(self.frame);
                        summary.latch_sp3(descriptor, header.data);
                        self.report.summary = Some(summary);
                    }
                }
            }

            #[cfg(feature = "sp3")]
            QcSerializedItem::PreciseState(item) => {
                if self.builder.sp3_orbit_proj {
                    
                    if let Some(report) = &mut self.report.sp3_orbit_proj {
                        report.latch(item);
                    } else {
                        let mut report = QcOrbitProjection::new();
                        report.latch(item);
                        self.report.sp3_orbit_proj = Some(report);
                    }
                }
            }

            QcSerializedItem::Signal(item) => {
                match item.data.observation {
                    QcSignalObservation::PseudoRange(value) => {
                        if self.builder.pseudo_range_observations {
                            if let Some(report) = &mut self.report.pseudo_range_observations {
                                report.latch_value(item.indexing, item.filename, item.data.sv, item.data.carrier, item.data.epoch, value); 
                            } else {
                                let mut report = QcObservationsReport::new("Pseudo Range");
                                report.latch_value(item.indexing, item.filename, item.data.sv, item.data.carrier, item.data.epoch, value);
                            }
                        }

                        if self.builder.needs_signals_buffering() {
                            let mut buffer = self.signals_buffer.unwrap();
                            buffer.latch(item);
                            
                            if self.builder.pseudo_range_residuals {

                            }
                            if self.builder.phase_range_residuals {

                            }
                            if self.builder.pseudo_range_if_combination {

                            }
                            if self.builder.pseudo_range_gf_combination {
                                for combination in buffer.pseudorange_gf_combinations() {
                                }
                            }
                        }
                    },
                    QcSignalObservation::PhaseRange(value) => {
                        if self.builder.carrier_phase_observations {
                            if let Some(report) = &mut self.report.carrier_phase_observations {
                                report.latch_value(item.indexing, item.filename, item.data.sv, item.data.carrier, item.data.epoch, value); 
                            } else {
                                let mut report = QcObservationsReport::new("Carrier Phase");
                                report.latch_value(item.indexing, item.filename, item.data.sv, item.data.carrier, item.data.epoch, value);
                            }
                        }
                    },
                    QcSignalObservation::Doppler(value) => {
                        if self.builder.signal_power_observations {
                            if let Some(report) = &mut self.report.doppler_observations {
                                report.latch_value(item.indexing, item.filename, item.data.sv, item.data.carrier, item.data.epoch, value); 
                            } else {
                                let mut report = QcObservationsReport::new("Doppler Shifts");
                                report.latch_value(item.indexing, item.filename, item.data.sv, item.data.carrier, item.data.epoch, value);
                            }
                        }
                    },
                    QcSignalObservation::SSI(value) => {
                        if self.builder.signal_power_observations {
                            if let Some(report) = &mut self.report.pseudo_range_observations {
                                report.latch_value(item.indexing, item.filename, item.data.sv, item.data.carrier, item.data.epoch, value); 
                            } else {
                                let mut report = QcObservationsReport::new("Signal Power");
                                report.latch_value(item.indexing, item.filename, item.data.sv, item.data.carrier, item.data.epoch, value);
                            }
                        }
                    },
                }
            }
        }
    }
}
