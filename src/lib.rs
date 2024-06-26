/*!

# Rust bindings to OpenJPEG

Supports loading JPEG2000 images into `image::DynamicImage`.

Forked from https://framagit.org/leoschwarz/jpeg2000-rust before its GPL-v3 relicensing, with some additional features:

* Specify decoding area and quality layers in addition to reduction factor
* Improved OpenJPEG -> DynamicImage loading process
* Get basic metadata from JPEG2000 headings
* Docs (albeit minimal ones)

This library brings its own libopenjpeg, which is statically linked. If you just need raw FFI bindings, see
[openjpeg2-sys](https://crates.io/crates/openjpeg2-sys) or [openjpeg-sys](https://crates.io/crates/openjpeg-sys).


## Usage

```rust,no_run
let bytes = include_bytes!("rust-logo-512x512-blk.jp2");
let codec = jp2k::Codec::jp2();
let stream = jp2k::Stream::from_bytes(bytes).unwrap();

let jp2k::ImageBuffer {
    buffer,
    width,
    height,
    num_bands,
    precision,
} = stream.decode(codec, jp2k::DecodeParams::default()).unwrap();

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
*/

pub mod err;

use std::io::{Cursor, Read};
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr::{self, NonNull};

use openjpeg_sys as ffi;

pub use ffi::COLOR_SPACE;
use ffi::OPJ_TRUE;

struct InnerDecodeParams(ffi::opj_dparameters);

impl Default for InnerDecodeParams {
    fn default() -> Self {
        let mut new = unsafe { std::mem::zeroed::<ffi::opj_dparameters>() };
        unsafe {
            ffi::opj_set_default_decoder_parameters(&mut new as *mut _);
        }
        InnerDecodeParams(new)
    }
}

#[derive(Debug, Clone, Default)]
struct DecodingArea {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

/// Parameters used to decode JPEG2000 image
#[derive(Debug, Clone, Default)]
pub struct DecodeParams {
    default_color_space: Option<COLOR_SPACE>,
    reduce_factor: Option<u32>,
    decoding_area: Option<DecodingArea>,
    quality_layers: Option<u32>,
    num_threads: Option<i32>,
}

impl DecodeParams {
    /// Used when the library cannot determine color space
    pub fn with_default_color_space(mut self, color_space: COLOR_SPACE) -> Self {
        self.default_color_space = Some(color_space);
        self
    }

    /// Image will be "scaled" to dim / (2 ^ reduce_factor)
    pub fn with_reduce_factor(mut self, reduce_factor: u32) -> Self {
        self.reduce_factor = Some(reduce_factor);
        self
    }

    pub fn with_num_threads(mut self, num: i32) -> Self {
        self.num_threads = Some(num);
        self
    }

    /// Image will be "cropped" to the specified decoding area, with width = x1 - x0 and height y1 - y0
    pub fn with_decoding_area(mut self, x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        self.decoding_area = Some(DecodingArea { x0, y0, x1, y1 });
        self
    }

    /// Will only use the specified number of quality layers
    pub fn with_quality_layers(mut self, quality_layers: u32) -> Self {
        self.quality_layers = Some(quality_layers);
        self
    }

    fn value_for_discard_level(u: u32, discard_level: u32) -> u32 {
        let div = 1 << discard_level;
        let quot = u / div;
        let rem = u % div;
        if rem > 0 {
            quot + 1
        } else {
            quot
        }
    }
}

pub struct Stream<'a> {
    ptr: *mut ffi::opj_stream_t,
    phantom: PhantomData<&'a ()>,
}

impl<'a> Drop for Stream<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::opj_stream_destroy(self.ptr);
        }
    }
}

impl<'a> Stream<'a> {
    pub fn from_bytes(buf: &'a [u8]) -> err::Result<Self> {
        let cur = Box::new(Cursor::new(buf));
        let ptr = unsafe {
            let jp2_stream = ffi::opj_stream_default_create(OPJ_TRUE as i32); // input stream
            ffi::opj_stream_set_read_function(jp2_stream, Some(Self::opj_stream_read_fn));
            ffi::opj_stream_set_user_data_length(jp2_stream, buf.len() as u64);
            ffi::opj_stream_set_user_data(
                jp2_stream,
                Box::into_raw(cur) as *mut c_void,
                Some(Self::opj_stream_free_user_data_fn),
            );
            jp2_stream
        };

        Ok(Stream {
            ptr,
            phantom: PhantomData,
        })
    }

    unsafe extern "C" fn opj_stream_read_fn(
        p_buffer: *mut c_void,
        p_nb_bytes: usize,
        p_user_data: *mut c_void,
    ) -> usize {
        let cur = p_user_data as *mut Cursor<&[u8]>;
        let dst = std::slice::from_raw_parts_mut(p_buffer as *mut u8, p_nb_bytes);
        let n = (*cur).read(dst);
        n.expect("Failed to read from buffer") // nothing can be done here
    }

    unsafe extern "C" fn opj_stream_free_user_data_fn(p_user_data: *mut c_void) {
        Box::from_raw(p_user_data as *mut Cursor<&[u8]>);
    }

    /// Decode a JPEG2000
    pub fn decode(self, codec: Codec, params: DecodeParams) -> err::Result<ImageBuffer> {
        let stream = self.ptr;
        let mut inner_params = InnerDecodeParams::default();

        if let Some(reduce_factor) = params.reduce_factor {
            inner_params.0.cp_reduce = reduce_factor;
        }

        if let Some(quality_layers) = params.quality_layers {
            inner_params.0.cp_layer = quality_layers;
        }

        if unsafe { ffi::opj_setup_decoder(codec.0.as_ptr(), &mut inner_params.0) } != 1 {
            return Err(err::Error::boxed("Setting up the decoder failed."));
        }

        if let Some(num_threads) = params.num_threads {
            if unsafe { ffi::opj_codec_set_threads(codec.0.as_ptr(), num_threads) } != 1 {
                return Err(err::Error::boxed("Could not set specified threads."));
            }
        }

        let mut img = Image::new();

        if unsafe { ffi::opj_read_header(stream, codec.0.as_ptr(), &mut img.0) } != 1 {
            return Err(err::Error::boxed("Failed to read header."));
        }

        if let Some(DecodingArea { x0, y0, x1, y1 }) = params.decoding_area {
            if unsafe { ffi::opj_set_decode_area(codec.0.as_ptr(), img.0, x0, y0, x1, y1) } != 1 {
                return Err(err::Error::boxed("Setting up the decoding area failed."));
            }
        }

        if unsafe { ffi::opj_decode(codec.0.as_ptr(), stream, img.0) } != 1 {
            return Err(err::Error::boxed("Failed to read image."));
        }

        // if unsafe { ffi::opj_end_decompress(codec.0.as_ptr(), stream.0) } != 1 {
        //     return Err(err::Error::boxed("Ending decoding failed."));
        // }

        let width = img.width();
        let height = img.height();
        let factor = img.factor();

        let width = DecodeParams::value_for_discard_level(width, factor);
        let height = DecodeParams::value_for_discard_level(height, factor);

        let num_bands;

        let (buffer, precision) = unsafe {
            match img.components() {
                [comp_r] => {
                    num_bands = 1;

                    if comp_r.prec == 8 {
                        let buffer =
                            std::slice::from_raw_parts(comp_r.data, (width * height) as usize)
                                .iter()
                                .map(|x| *x as u8)
                                .collect::<Vec<_>>();
                        (buffer, 8)
                    } else if comp_r.prec == 16 {
                        let buffer =
                            std::slice::from_raw_parts(comp_r.data, (width * height) as usize)
                                .iter()
                                .flat_map(|x| (*x as u16).to_ne_bytes())
                                .collect::<Vec<_>>();
                        (buffer, 16)
                    } else {
                        return Err(err::Error::boxed(format!(
                            "Unsupported precision for grayscale: {}",
                            comp_r.prec
                        )));
                    }
                }

                [comp_r, comp_g, comp_b] => {
                    if comp_r.prec != 8 {
                        return Err(err::Error::boxed(format!(
                            "Unsupported precision for RGB: {}",
                            comp_r.prec
                        )));
                    }
                    let r = std::slice::from_raw_parts(comp_r.data, (width * height) as usize);
                    let g = std::slice::from_raw_parts(comp_g.data, (width * height) as usize);
                    let b = std::slice::from_raw_parts(comp_b.data, (width * height) as usize);

                    num_bands = 3;

                    let buffer = Vec::with_capacity((width * height * num_bands) as usize);

                    (
                        r.iter().zip(g.iter()).zip(b.iter()).fold(
                            buffer,
                            |mut acc, ((r, g), b)| {
                                acc.extend_from_slice(&[*r as u8, *g as u8, *b as u8]);
                                acc
                            },
                        ),
                        8,
                    )
                }
                [comp_r, comp_g, comp_b, comp_a] => {
                    if comp_r.prec != 8 {
                        return Err(err::Error::boxed(format!(
                            "Unsupported precision for RGBA: {}",
                            comp_r.prec
                        )));
                    }
                    let r = std::slice::from_raw_parts(comp_r.data, (width * height) as usize);
                    let g = std::slice::from_raw_parts(comp_g.data, (width * height) as usize);
                    let b = std::slice::from_raw_parts(comp_b.data, (width * height) as usize);
                    let a = std::slice::from_raw_parts(comp_a.data, (width * height) as usize);

                    num_bands = 4;

                    let buffer = Vec::with_capacity((width * height * num_bands) as usize);

                    (
                        r.iter().zip(g.iter()).zip(b.iter()).zip(a.iter()).fold(
                            buffer,
                            |mut acc, (((r, g), b), a)| {
                                acc.extend_from_slice(&[*r as u8, *g as u8, *b as u8, *a as u8]);
                                acc
                            },
                        ),
                        8,
                    )
                }
                _ => {
                    return Err(err::Error::boxed(
                        "Operation not supported for that number of components",
                    ));
                }
            }
        };

        Ok(ImageBuffer {
            buffer,
            width,
            height,
            num_bands: num_bands as usize,
            precision,
        })
    }
}

impl Drop for Codec {
    fn drop(&mut self) {
        unsafe {
            ffi::opj_destroy_codec(self.0.as_ptr());
        }
    }
}

/// Thin wrapper around the `opj_codec_t` struct
pub struct Codec(NonNull<ffi::opj_codec_t>);

impl Codec {
    fn create(format: ffi::CODEC_FORMAT) -> Self {
        // following unwrap is safe since unknown format is never used.
        let ptr = unsafe { ffi::opj_create_decompress(format) };
        Codec(NonNull::new(ptr).unwrap())
    }

    /// JPEG-2000 codestream : read/write
    pub fn j2k() -> Self {
        Self::create(ffi::CODEC_FORMAT::OPJ_CODEC_J2K)
    }

    /// JPT-stream (JPEG 2000, JPIP) : read only
    pub fn jpt() -> Self {
        Self::create(ffi::CODEC_FORMAT::OPJ_CODEC_JPT)
    }

    /// JP2 file format : read/write
    pub fn jp2() -> Self {
        Self::create(ffi::CODEC_FORMAT::OPJ_CODEC_JP2)
    }

    /// JPP-stream (JPEG 2000, JPIP) : to be coded
    pub fn jpp() -> Self {
        Self::create(ffi::CODEC_FORMAT::OPJ_CODEC_JPP)
    }

    /// JPX file format (JPEG 2000 Part-2) : to be coded
    pub fn jpx() -> Self {
        Self::create(ffi::CODEC_FORMAT::OPJ_CODEC_JPX)
    }

    // unknown format should not be defined
}

pub struct Info {
    pub width: u32,
    pub height: u32,
}

impl Info {
    pub fn build(codec: Codec, stream: Stream) -> err::Result<Self> {
        let mut params = InnerDecodeParams::default();

        params.0.flags |= ffi::OPJ_DPARAMETERS_DUMP_FLAG;

        if unsafe { ffi::opj_setup_decoder(codec.0.as_ptr(), &mut params.0) } != 1 {
            return Err(err::Error::boxed("Setting up the decoder failed."));
        }

        let mut img = Image::new();

        if unsafe { ffi::opj_read_header(stream.ptr, codec.0.as_ptr(), &mut img.0) } != 1 {
            return Err(err::Error::boxed("Failed to read header."));
        }

        Ok(Info {
            width: img.width(),
            height: img.height(),
        })
    }
}

#[derive(Debug)]
pub struct Image(pub *mut ffi::opj_image_t);

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            ffi::opj_image_destroy(self.0);
        }
    }
}

impl Image {
    fn new() -> Self {
        Image(ptr::null_mut())
    }

    pub fn width(&self) -> u32 {
        unsafe { (*self.0).x1 - (*self.0).x0 }
    }

    pub fn height(&self) -> u32 {
        unsafe { (*self.0).y1 - (*self.0).y0 }
    }

    pub fn num_components(&self) -> u32 {
        unsafe { (*self.0).numcomps }
    }

    pub fn components(&self) -> &[ffi::opj_image_comp_t] {
        let comps_len = self.num_components();
        unsafe { std::slice::from_raw_parts((*self.0).comps, comps_len as usize) }
    }

    pub fn factor(&self) -> u32 {
        unsafe { (*(*self.0).comps).factor }
    }

    pub fn color_space(&self) -> COLOR_SPACE {
        unsafe { (*self.0).color_space }
    }
}

pub struct Component(*mut ffi::opj_image_comp_t);

#[derive(Debug)]
pub struct ImageBuffer {
    pub buffer: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub num_bands: usize,
    pub precision: u32,
}
