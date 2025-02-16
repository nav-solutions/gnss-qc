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
