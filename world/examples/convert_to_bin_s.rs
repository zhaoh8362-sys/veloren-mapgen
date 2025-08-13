/// This example reads a grayscale PNG heightmap, applies a simple smoothing
/// algorithm to smooth the terrain, and converts it into a .bin world file
/// (Veloren0_7_0 variant). The program takes three parameters:
///   1. Input PNG file path
///   2. Vertical scale factor (for converting 0–255 grayscale to altitude)
///   3. Height offset (an additive bias for all altitude values)
///
/// The algorithm works by converting each pixel's red channel value using:
///     altitude = (pixel / 255.0) * scale_factor + height_offset
/// Then one iteration of a simple box filter is applied to smooth the map.
/// The map_size_lg is computed from the image size (as exponent: 2^n).
///
/// Usage:
///   cargo run --example convert_to_bin --release -- path/to/heightmap.png 1000.0 -200.0
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use image::ImageReader;
use image::GenericImageView;
use bincode;
use veloren_world::sim::{WorldFile, WorldMap_0_7_0};
use vek::Vec2;

/// Applies a single iteration of a simple box filter to the altitude array.
/// The altitudes vector is interpreted as a grid with given width and height.
fn smooth_altitudes(alt: &[f64], width: u32, height: u32) -> Vec<f64> {
    let w = width as usize;
    let h = height as usize;
    let mut out = alt.to_vec();
    // Iterate over every pixel.
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            let mut count = 0.0;
            // Process 3x3 kernel (include self and neighbors).
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx >= 0 && ny >= 0 && nx < w as isize && ny < h as isize {
                        let idx = (ny as usize) * w + (nx as usize);
                        sum += alt[idx];
                        count += 1.0;
                    }
                }
            }
            out[y * w + x] = sum / count;
        }
    }
    out
}

fn main() {
    // Ensure proper usage: three parameters required.
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <input_png> <scale_factor> <height_offset>", args[0]);
        std::process::exit(1);
    }
    let input_path = PathBuf::from(&args[1]);
    let scale_factor: f64 = args[2].parse().expect("Invalid scale factor");
    let height_offset: f64 = args[3].parse().expect("Invalid height offset");

    // Open and decode the PNG image.
    let img = ImageReader::open(&input_path)
        .expect("Failed to open image")
        .decode()
        .expect("Failed to decode image");

    // Get image dimensions.
    let (width, height) = img.dimensions();
    println!("Image dimensions: {}x{}", width, height);

    // Validate image is square and dimensions are power-of-two.
    if width != height {
        eprintln!("Image width and height must be equal.");
        std::process::exit(1);
    }
    if !width.is_power_of_two() {
        eprintln!("Image width (and height) must be a power of two.");
        std::process::exit(1);
    }
    // Compute exponent n such that resolution = 2^n.
    let exponent = width.trailing_zeros();
    let expected_pixels = (1 << exponent) * (1 << exponent);
    if width * height != expected_pixels {
        eprintln!(
            "Pixel count mismatch: found {} pixels, expected {} pixels.",
            width * height,
            expected_pixels
        );
        std::process::exit(1);
    }

    // Create altitude vector from the red channel of the image.
    // Formula: altitude = (pixel / 255.0) * scale_factor + height_offset.
    let mut alt_vec: Vec<f64> = Vec::with_capacity((width * height) as usize);
    for (_x, _y, pixel) in img.pixels() {
        let r = pixel[0] as f64;
        let alt = (r / 255.0) * scale_factor + height_offset;
        alt_vec.push(alt);
    }

    // Apply smoothing algorithm.
    let alt_vec_smoothed = smooth_altitudes(&alt_vec, width, height);

    // For basement, duplicate the smoothed altitudes.
    let basement_vec = alt_vec_smoothed.clone();
    let continent_scale = 1.5;
    // Create a world map struct.
    // The map_size_lg field stores the exponents, so if exponent = 10, resolution = 2^10 = 1024.
    let world_map = WorldMap_0_7_0 {
        map_size_lg: Vec2::new(exponent, exponent),
        continent_scale_hack: continent_scale,
        alt: alt_vec_smoothed.into_boxed_slice(),
        basement: basement_vec.into_boxed_slice(),
    };

    // Wrap the world map into the WorldFile enum.
    let world_file = WorldFile::Veloren0_7_0(world_map);

    // Serialize the world file using bincode.
    let serialized = bincode::serialize(&world_file).expect("Failed to serialize world file");

    // Determine the output file path (same base as input, but with a .bin extension).
    let mut output_path = input_path.clone();
    output_path.set_extension("bin");

    let mut file = File::create(&output_path).expect("Failed to create output file");
    file.write_all(&serialized)
        .expect("Failed to write output file");

    println!(
        "Converted {} -> {}",
        input_path.display(),
        output_path.display()
    );
    println!(
        "Map size: {}x{} (exponent: {}), scale factor: {}, height offset: {}",
        width,
        height,
        exponent,
        scale_factor,
        height_offset
    );
}