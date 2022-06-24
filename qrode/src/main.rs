use std::io::Write;

use clap::Parser;

#[derive(Parser)]
#[clap(name = "qrode")]
#[clap(author = "redArch <redarch@protonmail.com>")]
#[clap(version = "0.1.0")]
#[clap(about = "QR code decoder tool for Wayland compositors. Works great with slurp.", long_about = None)]
struct Cli
{
	#[clap(
		short('g'),
		value_parser,
		value_names(&gazo::Region::get_parser_formats()),
		allow_hyphen_values(true),
		help("Set the region to capture")
	)]
	geometry: gazo::Region,
}

fn main()
{
	let cli = Cli::parse();

	let capture = gazo::capture_region(cli.geometry.position, cli.geometry.size, false).unwrap();

	let mut prepared_image = rqrr::PreparedImage::prepare_from_greyscale(
		capture.width as usize,
		capture.height as usize,
		move |x, y| {
			let index = (y * capture.width) + x;

			// average the rgb values for grayscale, value must be divided individually as
			// total can exceed the size of a u8
			(capture.pixel_data[index].r / 3)
				+ (capture.pixel_data[index].g / 3)
				+ (capture.pixel_data[index].b / 3)
		},
	);

	let grids = prepared_image.detect_grids();

	if grids.is_empty()
	{
		println!("No QR codes detected");
		std::process::exit(1);
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
}
