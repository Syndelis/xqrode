use std::io::Write;

use clap::Parser;
use libshotgun::{self, image, Rect};

#[derive(Parser)]
#[clap(name = "x11qrode")]
#[clap(author = "redArch <redarch@protonmail.com>")]
#[clap(version = "0.1.0")]
#[clap(about = "QR code decoder tool for Wayland compositors. Works great with slurp.", long_about = None)]
struct Cli
{
	#[clap(
		short('g'),
		value_parser,
		value_names(&libshotgun::Rect::get_parser_formats()),
		allow_hyphen_values(true),
		help("Set the region to capture")
	)]
	geometry: libshotgun::Rect,
}

fn main() {
	let cli = Cli::parse();

	let rect = cli.geometry;
	let mut image = libshotgun::capture_region(rect);

	if !get_qr(&image, rect) {
		image.invert();
		if !get_qr(&image, rect) {
			println!("No QR code found.");
			return;
		}
	}

}

fn get_qr(image: &image::DynamicImage, rect: Rect) -> bool {

	let image_buffer = image.to_rgb8();

	let mut prepared_image = rqrr::PreparedImage::prepare_from_greyscale(
		rect.w as usize,
		rect.h as usize,
		move |x, y| {

			// average the rgb values for grayscale, value must be divided individually as
			// total can exceed the size of a u8

			let rgb = image_buffer.get_pixel(x as u32, y as u32).0;
			(rgb[0] / 3)
			+ (rgb[1] / 3)
			+ (rgb[2] / 3)

		},
	);

	let grids = prepared_image.detect_grids();

	if grids.is_empty() {
		return false;
	}

	for grid in grids
	{
		let (_, data) = grid.decode().unwrap_or_else(|error| {
			eprintln!("There was a problem decoding the QR code: {}", error);
			std::process::exit(1);
		});

		match url::Url::parse(&data)
		{
			Ok(url) =>
			{
				open::that(url.as_str()).unwrap_or_else(|error| {
					eprintln!("Failed to open URL with default application: {}", error);
					std::process::exit(1);
				});
			}
			Err(_) =>
			{
				let tempfile = tempfile::NamedTempFile::new().unwrap();

				write!(tempfile.as_file(), "{}", data).unwrap();

				open::that(tempfile.path()).unwrap_or_else(|error| {
					eprintln!(
						"Failed to open the QR code text with default application: {}",
						error
					);
					std::process::exit(1);
				});
			}
		}
	}

	true

}