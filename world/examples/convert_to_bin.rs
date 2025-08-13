use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use image::ImageReader;
use image::GenericImageView;
use bincode;
use veloren_world::sim::{WorldFile, WorldMap_0_7_0};
use vek::Vec2;

fn main() {
    // Ensure the program is called with two parameters: input PNG and scale factor.
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input_png> <scale_factor>", args[0]);
        std::process::exit(1);
    }
    let input_path = PathBuf::from(&args[1]);
    let scale_factor: f64 = args[2].parse().expect("Invalid scale factor");

    // Open and decode the PNG image.
    let img = ImageReader::open(&input_path)
        .expect("Failed to open image")
        .decode()
        .expect("Failed to decode image");

    // Get image dimensions.
    let (width, height) = img.dimensions();
    println!("Image dimensions: {}x{}", width, height);

    // Validate that the image is square.
    if width != height {
        eprintln!("Image width and height must be equal.");
        std::process::exit(1);
    }
    // Validate that width is a power-of-two.
    if !width.is_power_of_two() {
        eprintln!("Image width (and height) must be a power of two.");
        std::process::exit(1);
    }
    // Compute the exponent n such that resolution = 2^n.
    // For example, if width is 1024, then n = 10.
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

    // Set a bias for altitude.
    const ALTITUDE_BIAS: f64 = -600.0;

    // Create the altitude vector.
    // We assume the PNG is a grayscale image so we use the red channel.
    // The formula: altitude = (pixel / 255.0) * scale_factor + bias.
    let mut alt_vec: Vec<f64> = Vec::with_capacity((width * height) as usize);
    for (_x, _y, pixel) in img.pixels() {
        let r = pixel[0] as f64;
        let alt = (r / 255.0) * scale_factor + ALTITUDE_BIAS;
        alt_vec.push(alt);
    }

    // For the basement, as a simple approach, we duplicate the altitudes.
    let basement_vec = alt_vec.clone();
    let continent_scale = 1.6;
    // Create a world map struct.
    // Note that map_size_lg is stored as the exponent, so if exponent = 10, that means the actual resolution is 2^10=1024.
    let world_map = WorldMap_0_7_0 {
        map_size_lg: Vec2::new(exponent, exponent),
        // Use the scale factor here in the continent_scale_hack field.
        continent_scale_hack: continent_scale,
        alt: alt_vec.into_boxed_slice(),
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
        "Map size: {}x{} (exponent: {}), scale factor: {}",
        width,
        height,
        exponent,
        scale_factor
    );
}