use crate::prelude::{Preprocessing, QcContext};

mod decimation;
mod masking;
mod split;
mod timeshift;

impl Preprocessing for QcContext {}
