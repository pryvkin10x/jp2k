fn main() {
    let bytes = include_bytes!("rust-logo-512x512-blk.jp2");

    let codec = jp2k::Codec::jp2();
    let stream = jp2k::Stream::from_bytes(bytes).unwrap();
    // let stream = jp2k::Stream::from_file("/mnt/c/projects/iiif-server/cache/remote/322930.jp2").unwrap();

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
}
