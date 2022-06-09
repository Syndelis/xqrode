use std::io::{self, Write};

use gazo;
use open;
use regex::Regex;
use rqrr;
use tempfile;
use url;

fn main()
{
	let mut buffer = String::new();
	let stdin = io::stdin();

	stdin
		.read_line(&mut buffer)
		.expect("Failed to read from stdin.");

	let re = Regex::new(r"(-?\d+),(-?\d+) (\d+)x(\d+)").unwrap();

	let captures = re.captures(&buffer).expect("Failed to parse input.");

	let position = (
		captures.get(1).unwrap().as_str().parse::<i32>().unwrap(),
		captures.get(2).unwrap().as_str().parse::<i32>().unwrap(),
	);
	let size = (
		captures.get(3).unwrap().as_str().parse::<i32>().unwrap(),
		captures.get(4).unwrap().as_str().parse::<i32>().unwrap(),
	);

	let capture = gazo::capture_region(position, size);

	let capture_size = capture.get_size_pixels();

	let mut prepared_image = rqrr::PreparedImage::prepare_from_greyscale(
		capture_size.width as usize,
		capture_size.height as usize,
		move |x, y| {
			// average the rgb value
			capture.get_pixel(x, y).unwrap()[0..3]
				.iter()
				.cloned()
				.fold(0, |accumulator, item| accumulator + (item / 3))
		},
	);

	let grids = prepared_image.detect_grids();

	if grids.len() < 1
	{
		println!("No QR codes detected");

		return ();
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
