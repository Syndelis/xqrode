use std::io::Write;

use clap::Parser;

#[derive(Parser)]
#[clap(name = "qrode")]
#[clap(author = "redArch <redarch@protonmail.com>")]
#[clap(version = "0.1.0")]
#[clap(about = "QR code decoder tool for Wayland compositors", long_about = None)]
struct Cli
{
	#[clap(short('g'), value_parser, help("Set the region to capture"))]
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

			// average the rgb values for grayscale value
			// must be divided individually
			(capture.pixel_data[index].r / 3)
				+ (capture.pixel_data[index].g / 3)
				+ (capture.pixel_data[index].b / 3)
		},
	);

	let grids = prepared_image.detect_grids();

	if grids.is_empty()
	{
		println!("No QR codes detected");

		return;
	}

	for grid in grids
	{
		let (_, data) = grid.decode().unwrap();

		match url::Url::parse(&data)
		{
			Ok(url) =>
			{
				open::that(url.as_str()).expect("Failed to open URL with default application.")
			}
			Err(_) =>
			{
				let tempfile = tempfile::NamedTempFile::new().unwrap();

				write!(tempfile.as_file(), "{}", data).unwrap();

				open::that(tempfile.path())
					.expect("Failed to open the QR code data with default application.");
			}
		}
	}
}
