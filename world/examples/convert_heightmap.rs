use std::fs::File;
use std::io::{BufReader, Read, Write};
use image::{ImageBuffer, Rgb, codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use image::codecs::png::{CompressionType, FilterType};
use veloren_world::sim::WorldFile;
use bincode;

/// Loads the `.bin` file from the given path and extracts the alt array.
/// This example expects the file to be in the Veloren 0.7.0 format.
fn load_alt_array(file_path: &str) -> Vec<f32> {
    let file = File::open(file_path).expect("Failed to open file");
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).expect("Failed to read file");

    // Deserialize the buffer to get the world file.
    let world_file: WorldFile = bincode::deserialize(&buffer)
        .expect("Failed to deserialize world file");

    // Match based on world file version.
    if let WorldFile::Veloren0_7_0(map) = world_file {
        // Convert Vec<f64> to Vec<f32>
        return map.alt.iter().map(|&x| x as f32).collect();
    }

    panic!("Unsupported world file version");
}

/// Computes the minimum and maximum values in the alt array.
fn compute_min_max(alt_array: &[f32]) -> (f32, f32) {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for &val in alt_array {
        if val < min {
            min = val;
        }
        if val > max {
            max = val;
        }
    }
    (min, max)
}

/// Generates a heightmap PNG image from the alt array.
/// The alt values are scaled to the 0–255 range using the provided minimum and maximum.
fn generate_heightmap(alt_array: Vec<f32>, width: u32, height: u32, output_path: &str, min: f32, max: f32) {
    let mut heightmap: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);
    let range = max - min;
    // Avoid division by zero in case of a flat map:
    let range = if range == 0.0 { 1.0 } else { range };

    for (x, y, pixel) in heightmap.enumerate_pixels_mut() {
        let alt = alt_array[(y * width + x) as usize];
        // Scale the altitude value to [0, 255].
        let pixel_value = (((alt - min) / range) * 255.0).round() as u8;
        *pixel = Rgb([pixel_value, pixel_value, pixel_value]);
    }

    let mut heightmap_png = Vec::new();
    let mut encoder = PngEncoder::new_with_quality(
        &mut heightmap_png,
        CompressionType::Best,
        FilterType::Paeth,
    );
    encoder.write_image(
        heightmap.as_raw(),
        heightmap.width(),
        heightmap.height(),
        ExtendedColorType::Rgb8,
    ).expect("Failed to write PNG image");

    let mut f = File::create(output_path).expect("Failed to create output file");
    f.write_all(&heightmap_png).expect("Failed to write PNG data to file");
}

fn main() {
    // Set the path to your .bin file. Adjust as necessary.
    let file_path = "maps/map.bin";
    let alt_array = load_alt_array(file_path);

    // Compute the minimum and maximum altitude values.
    let (min_alt, max_alt) = compute_min_max(&alt_array);
    println!("Original alt range: min = {}, max = {}", min_alt, max_alt);

    // Set the dimensions. They must match your map's expected dimensions.
    let width = 1024;
    let height = 1024;
    let output_path = "heightmap.png";
    generate_heightmap(alt_array, width, height, output_path, min_alt, max_alt);
}