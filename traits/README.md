GNSS Quality Control Traits
===========================

The Quality Control traits library (`gnss-qc-traits`) is a small library
that offers all traits that need to be shared by all types involved in the GNSS
Quality Control library. 

As an example, this crate is implemented in the GeoRust/RINEX and GeoRust/SP3 libraries

## Existing Modules

- html: HTML report rendition
- merge: describes how we stack data into an already existing context
- processing: available on crate feature only,
describes a filter designer and processing ops
