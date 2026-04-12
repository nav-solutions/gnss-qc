//! File operations for a dataset

use std::{
    fs::Path,
};

use walkdir::WalkDir;

use crate::prelude::QcDataset;

use rinex::prelude::Rinex;

#[cfg(feature = "sp3")]
use sp3::prelude::SP3;

impl QcDataset {
    /// Form a [QcDataset] from a local directory.
    /// Unlike [Self::from_file] this is infaillible,
    /// non supported formats (especially depending on your compilation options)
    /// will be simply discarded. You can either check the logs
    /// or analyze the results to make sure all data was grabbed correctly.
    ///
    /// Input: 
    /// - dir: local directory
    /// - max_search: maximal depth for the recursive search.
    /// Files below this value will not be picked up.
    pub fn from_dir<P: AsRef<Path>>>(dir: P, max_search: usize) -> Self {
        let mut s = Self::default();
        s.load_dir(dir, max_search);
        Self { data }
    }

    /// Load the supported and correctly identified content,
    /// from this local directory, into this mutable [QcDataset],
    /// augmenting (not overwritting already existing data) the dataset.
    ///
    /// Unlike [Self::load_file] this is infaillible,
    /// non supported formats (especially depending on your compilation options)
    /// will be simply discarded. You can either check the logs
    /// or analyze the results to make sure all data was grabbed correctly.
    ///
    /// Input: 
    /// - dir: local directory
    /// - max_search: maximal depth for the recursive search.
    /// Files below this value will not be picked up.
    pub fn load_dir<P: AsRef<Path>>(&mut self, dir: P, max_search: usize) {
        let mut walker = WalkDir::new(dir).max(max_search);
    }

    /// Load this (readable, non compressed) file and index it into a [QcDataset].
    pub fn from_file<P: AsRef<Path>>>(file: P) -> Result<Self, ParsingError> {
        match s.from_rinex_file(file) {
            Ok(s) => Ok(s),
            Err(_) => {
                #[cfg(feature = "sp3")]
                match s.from_sp3_file(file) {
                    Ok(s) => Ok(s),
                    Err(_) => Err(ParsingError::NonSupported),
                }
            },
        }
    }
    
    pub fn from_rinex_file<P: AsRef<Path>>>(file: P) -> Result<Self, ParsingError> {
        let rinex = Rinex::from_file(file)?;
    }
    
    pub fn load_file<P: AsRef<Path>>>(&mut self, file: P) -> Result<(), ParsingError> {
    }
    
    pub fn load_rinex_file<P: AsRef<Path>>>(&mut self, file: P) -> Result<(), ParsingError> {
    }
}
