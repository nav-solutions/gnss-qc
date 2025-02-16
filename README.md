GNSS Quality Control
====================

The `GNSS-Qc` (Quality Control) library answers the demanding task
of GNSS data (post) processing. This topic usually involves several different format
at the input of a processing pipeline. A processing pipeline

This library offers to form a procesing pipeline for all formats that implement
our [Qc Traits](https://github.com/rtk-rs/qc-traits) (low level traits).

The Qc library offers several important features

- the definition of an Almanac by means of the
the [ANISE](https://github.com/nyx-space/anise)
- the definition of a precise Earth centered Reference [Frame](https://github.com/nyx-space/anise)
- the possibility to inject [RINEX](https://github.com/georust/rinex) data at the input
of the processing pipeline, which one of the most convenient format for GNSS post processing
- the possibility to inject [SP3](https://github.com/georust/rinex/tree/main/sp3) data
at the input of the pipeline, which is the standard format for post processed high precision GNSS navigation.
- precise timing thanks to the [Hifitime](https://github.com/nyx-space/hifitime) library

## Licensing

These libraries is part of the [RTK-rs framework](https://github.com/rtk-rs) which
is delivered under the [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.
