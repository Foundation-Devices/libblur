use image::io::Reader as ImageReader;
use image::{EncodableLayout, GenericImageView};
use libblur::{FastBlurChannels, ThreadingPolicy};
use std::time::Instant;

#[allow(dead_code)]
fn f32_to_f16(bytes: Vec<f32>) -> Vec<u16> {
    return bytes
        .iter()
        .map(|&x| half::f16::from_f32(x).to_bits())
        .collect();
}

#[allow(dead_code)]
fn f16_to_f32(bytes: Vec<u16>) -> Vec<f32> {
    return bytes
        .iter()
        .map(|&x| half::f16::from_bits(x).to_f32())
        .collect();
}

fn main() {
    let img = ImageReader::open("assets/test_image_2.png")
        .unwrap()
        .decode()
        .unwrap();
    let dimensions = img.dimensions();
    println!("dimensions {:?}", img.dimensions());

    println!("{:?}", img.color());
    let src_bytes = img.as_bytes();
    let channels = 4;
    let stride = dimensions.0 as usize * channels;
    let mut bytes: Vec<u8> = Vec::with_capacity(dimensions.1 as usize * stride);
    for i in 0..dimensions.1 as usize * stride {
        bytes.push(src_bytes[i]);
    }
    let mut dst_bytes: Vec<u8> = Vec::with_capacity(dimensions.1 as usize * stride);
    dst_bytes.resize(dimensions.1 as usize * stride, 0);
    let start_time = Instant::now();

    // libblur::fast_gaussian(
    //     &mut bytes,
    //     stride as u32,
    //     dimensions.0,
    //     dimensions.1,
    //     32,
    //     FastBlurChannels::Channels3,
    //     ThreadingPolicy::Adaptive,
    // );

    // libblur::gaussian_box_blur(
    //     &bytes,
    //     stride as u32,
    //     &mut dst_bytes,
    //     stride as u32,
    //     dimensions.0,
    //     dimensions.1,
    //     35,
    //     FastBlurChannels::Channels4,
    //     ThreadingPolicy::Single,
    // );
    // bytes = dst_bytes;
    libblur::gaussian_blur(
        &bytes,
        stride as u32,
        &mut dst_bytes,
        stride as u32,
        dimensions.0,
        dimensions.1,
        150 * 2 + 1,
        (150f32 * 2f32 + 1f32) / 6f32,
        FastBlurChannels::Channels4,
        ThreadingPolicy::Single,
    );
    bytes = dst_bytes;
    // libblur::median_blur(
    //     &bytes,
    //     stride as u32,
    //     &mut dst_bytes,
    //     stride as u32,
    //     dimensions.0,
    //     dimensions.1,
    //     125,
    //     FastBlurChannels::Channels3,
    //     ThreadingPolicy::Adaptive,
    // );
    // bytes = dst_bytes;
    // libblur::gaussian_box_blur(&bytes, stride as u32, &mut dst_bytes, stride as u32, dimensions.0, dimensions.1, 77,
    //                            FastBlurChannels::Channels3, ThreadingPolicy::Single);
    // bytes = dst_bytes;
    let elapsed_time = start_time.elapsed();
    // Print the elapsed time in milliseconds
    println!("Elapsed time: {:.2?}", elapsed_time);

    image::save_buffer(
        "blurred.png",
        bytes.as_bytes(),
        dimensions.0,
        dimensions.1,
        if channels == 4 {
            image::ExtendedColorType::Rgba8
        } else {
            image::ExtendedColorType::Rgb8
        },
    )
    .unwrap();
}
