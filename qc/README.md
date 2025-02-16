GNSS Quality Control
====================

The GNSS Quality Control library (`gnss-qc`) offers high level and performant
API and operations that are required in the processing of GNSS.

The origin of this topic is the need to combine both RINEX and SP3 formats
to answer the basic requirement of (post processed) precise navigation 

The Qc library generates a `QcReport` (also refered to as output product), from the input context.
The report content depends on the provided combination of input files (also refered
to as, input products). 
QC standing for Quality Control, as it is a widely spread term in preprocessing
applications, the QC may apply to navigation applications, atmosphere analysis
and timing applications.

The `QcReport` comprises one tab per input product (dedicated tab),
may have tabs depending on the operations that the input context allows.
For example SP3 and/or BRDC RINEX will enable the `Orbit Projection tab`.

The report is render in HTML and that is currently the only format we can render.

`QcReport` allows customization with extra chapters, so you can append
as many chapters as you need, depending on your requirements and capabilities,
as long as you can implement the rendition Trait.

## Create features

- activate the `sp3` feature to support SP3 format
- activate the `plot` feature for your reports to integrate graphs analysis
- activate the `flate2` feature to directly load Gzip compressed input products
