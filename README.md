# librptadvdtmf

Portable fixed-state 48 kHz DTMF detector extracted from `rpt_advanced` core
commit `c63db09`. Its ABI 1 descriptor processes arbitrary F32 partitions in
place, reports completed standard digits through a borrowed ASCII callback,
and accepts zero-length carrier-loss calls. Detector handles are allocated
before audio starts; `process` neither allocates nor blocks.

The detector preserves the controller's 612-sample analysis interval,
qualification, release, and muting behavior. Command collection, authorization,
and DTMF generation remain outside this initial provider release.

`make quality` runs portable source and documentation checks. `make test`
checks the Rust detector and C descriptor. `make coverage` audits production
line and branch coverage on native Debian 13 amd64. `make platform-verify`
checks staged installation, package artifacts, and source distribution.
