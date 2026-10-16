# librptadvdtmf

Portable DTMF detection and generation for 48 kHz mono F32 audio. The library
supports all 16 digits: `0`–`9`, `A`–`D`, `*`, and `#`. It has no radio hardware
or Asterisk dependency.

Use `rptadv_dtmf_descriptor()` from `rptadvdtmf/rptadvdtmf.h`. Check the
descriptor identity, ABI version and size before using its functions.

The detector accepts audio in arbitrary-sized blocks and reports completed
digits through an ASCII callback. It can mute qualified tones. Pass a zero
receive flag when carrier drops, even when no samples are available.

The generator produces one digit followed by silence. Set the tone and silence
lengths from 1 through 1000 ms each. Set the combined peak level from -60
through 0 dBFS. Both tone components have equal levels. Render into successive
output buffers until a nonempty buffer receives zero samples. The returned
sample count includes the trailing silence.

Create and destroy handles outside the audio callback. Processing and rendering
do not allocate or block. The generator dynamically uses `librptadvtone.so.1`.
Command collection and authorization belong to the calling application.

## Build and check

Install the Rust toolchain, a C compiler, pkg-config and `librptadvtone-dev`.
Run `make` to build the shared library, then `make test`. Use `make install`
with `PREFIX` and `DESTDIR` as needed. Link applications with the flags from
`pkg-config --cflags --libs rptadvdtmf`.

`make quality` runs portable source and documentation checks. `make test`
checks the detector, generator and C descriptor. `make coverage` audits production
line and branch coverage on native Debian 13 amd64. `make platform-verify`
checks staged installation, package artifacts, and source distribution.
