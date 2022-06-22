use std::io::{self, Write};

use regex::Regex;

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

	let capture = gazo::capture_region(position, size, false).unwrap();

	let mut prepared_image = rqrr::PreparedImage::prepare_from_greyscale(
		capture.width as usize,
		capture.height as usize,
		move |x, y| {
			let index = (y * capture.width * 4) + (x * 4);

			// average the rgb values for grayscale value
			// must be divided individually
			(capture.pixel_data[index] / 3)
				+ (capture.pixel_data[index + 1] / 3)
				+ (capture.pixel_data[index + 2] / 3)
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
