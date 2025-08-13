/// This example traverses all .bin files in a given folder (specified as the first command-line argument),
/// extracts the `alt` array from each world file (formatted as Veloren 0.7.0),
/// computes its minimum and maximum values,
/// re-maps the alt values to the range 0\u2013255 for a grayscale height map,
/// prints the original value range for each file,
/// and saves the height map as a PNG file with the same base name (but with a .png extension).
///
/// To run this example:
///   cargo run --example convert_all_heightmaps --release -- /path/to/folder
use std::env;
use std::fs::{File, read_dir};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use image::{ImageBuffer, Rgb, codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use image::codecs::png::{CompressionType, FilterType};
use veloren_world::sim::WorldFile;
use bincode;

/// Loads the .bin file from the given path and extracts the alt array.
/// This example expects the world file to be in the Veloren 0.7.0 format.
fn load_alt_array(file_path: &Path) -> Vec<f32> {
    let file = File::open(file_path).expect("Failed to open file");
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).expect("Failed to read file");

    // Deserialize the buffer to get the world file.
    let world_file: WorldFile = bincode::deserialize(&buffer)
        .expect("Failed to deserialize world file");
    
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
/// The alt values are scaled to [0, 255] using the provided min and max values.
fn generate_heightmap(alt_array: Vec<f32>, width: u32, height: u32, output_path: &Path, min: f32, max: f32) {
    let mut heightmap: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);
    let range = max - min;
    let range = if range == 0.0 { 1.0 } else { range };

    for (x, y, pixel) in heightmap.enumerate_pixels_mut() {
        let alt = alt_array[(y * width + x) as usize];
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

/// Processes a single .bin file:
/// - Loads the alt array, computes the min/max,
/// - Generates a PNG heightmap with the same base filename,
/// - Prints the original range.
fn process_bin_file(bin_path: &Path, width: u32, height: u32) {
    println!("Processing file: {}", bin_path.display());
    let alt_array = load_alt_array(bin_path);
    let (min_alt, max_alt) = compute_min_max(&alt_array);
    println!("  alt range: min = {}, max = {}", min_alt, max_alt);

    // Create the output path with the same base name but .png extension.
    let mut output_path = bin_path.to_path_buf();
    output_path.set_extension("png");

    generate_heightmap(alt_array, width, height, &output_path, min_alt, max_alt);
    println!("  Heightmap saved to: {}", output_path.display());
}

fn main() {
    // Get the folder path from the command-line arguments.
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <folder_path>", args[0]);
        std::process::exit(1);
    }
    let folder_path = PathBuf::from(&args[1]);
    if !folder_path.is_dir() {
        eprintln!("The provided path is not a directory: {}", folder_path.display());
        std::process::exit(1);
    }

    // Set the dimensions. These should match your world dimensions.
    let width = 1024;
    let height = 1024;

    // Iterate through all entries in the folder.
    for entry in read_dir(folder_path).expect("Failed to read directory") {
        if let Ok(entry) = entry {
            let path = entry.path();
            // Process only files with the .bin extension.
            if let Some(ext) = path.extension() {
                if ext == "bin" {
                    process_bin_file(&path, width, height);
                }
            }
        }
    }
}