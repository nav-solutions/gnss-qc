GNSS Quality Control
====================

This workspace offers high level software and API to post process GNSS data
in Rust language. GNSS post processing is complex and usually involves
several formats to deploy a processing pipeline, this library offers just that.

## Libraries

- the [GNSS-Qc library](qc/) is a high level API that answers the requirement
of GNSS data post processing
- the [Qc Traits](traits/) are implemented by all formats that may contribute
to a processing pipeline

## Licensing

These libraries is part of the [RTK-rs framework](https://github.com/rtk-rs) which
is delivered under the [Mozilla Public v2](https://www.mozilla.org/en-US/MPL/2.0) license.
