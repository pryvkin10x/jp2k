fn main() {
    let bytes = include_bytes!("rust-logo-512x512-blk.jp2");

    let codec = jp2k::Codec::jp2();
    let stream = jp2k::Stream::from_bytes(bytes).unwrap();

    let jp2k::ImageBuffer {
        buffer,
        width,
        height,
        num_bands,
        precision,
    } = stream
        .decode(codec, jp2k::DecodeParams::default().with_reduce_factor(1))
        .unwrap();

    let color_type = match (num_bands, precision) {
        (1, 8) => image::ColorType::L8,
        (1, 16) => image::ColorType::L16,
        (2, 8) => image::ColorType::La8,
        (3, 8) => image::ColorType::Rgb8,
        (4, 8) => image::ColorType::Rgba8,
        _ => {
            panic!(
                "unsupported num_bands, precision: {}, {}",
                num_bands, precision
            )
        }
    };

    image::save_buffer(
        "examples/output/image.png",
        &buffer,
        width,
        height,
        color_type,
    )
    .unwrap();
}
