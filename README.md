# jp2k

[![Build status](https://github.com/dskkato/jp2k/actions/workflows/ci.yml/badge.svg)](https://github.com/dskkato/jp2k/actions/workflows/ci.yml)

## Rust bindings to OpenJPEG

Supports JPEG2000 decoding

Forked from https://github.com/kardeiz/jp2k, and https://github.com/kardeiz/jp2k is a fork
from https://framagit.org/leoschwarz/jpeg2000-rust, before its GPL-v3 relicensing, with some
additional features:

* Specify decoding area and quality layers in addition to reduction factor
* Improved OpenJPEG -> DynamicImage loading process
* Get basic metadata from JPEG2000 headings
* Docs (albeit minimal ones)

This library [openjpeg-sys](https://crates.io/crates/openjpeg-sys) as ffi.

## Usage

```rust
let bytes = include_bytes!("rust-logo-512x512-blk.jp2");

let codec = jp2k::Codec::jp2();
let stream = jp2k::Stream::from_bytes(bytes).unwrap();
// let stream = jp2k::Stream::from_file("rust-logo-512x512-blk.jp2").unwrap();

let jp2k::ImageBuffer {
    buffer,
    width,
    height,
    num_bands,
} = jp2k::ImageBuffer::build(
    codec,
    stream,
    jp2k::DecodeParams::default().with_reduce_factor(1),
)
.unwrap();

let color_type = match num_bands {
    1 => image::ColorType::L8,
    2 => image::ColorType::La8,
    3 => image::ColorType::Rgb8,
    4 => image::ColorType::Rgba8,
    _ => panic!(format!("unsupported num_bands found : {}", num_bands)),
};
image::save_buffer(
    "examples/output/image.png",
    &buffer,
    width,
    height,
    color_type,
)
.unwrap();
```

## Original warnings and license statement

### Warning
Please be advised that using C code means this crate is likely vulnerable to various memory exploits, e.g. see [http://cve.mitre.org/cgi-bin/cvename.cgi?name=CVE-2016-8332](CVE-2016-8332) for an actual example from the past.

As soon as someone writes an efficient JPEG2000 decoder in pure Rust you should probably switch over to that.

### License
You can use the Rust code in the directories `src` and `openjp2-sys/src` under the terms of either the MIT license (`LICENSE-MIT` file) or the Apache license (`LICENSE-APACHE` file). Please note that this will link statically to OpenJPEG, which has its own license which you can find at `openjpeg-sys/libopenjpeg/LICENSE` (you might have to check out the git submodule first).

License: MIT OR Apache-2.0
