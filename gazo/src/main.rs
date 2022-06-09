use std::{
	fs,
	io::{self, Write},
};

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

	let capture = gazo::capture_region(position, size);

	let capture_size = capture.get_size_pixels();

	let mut tmp_file = fs::File::create("/tmp/gazo.ppm").unwrap();

	writeln!(
		tmp_file,
		"P3\n{} {}\n255",
		capture_size.width, capture_size.height
	)
	.unwrap();

	for y in 0..capture_size.height
	{
		for x in 0..capture_size.width
		{
			let pixel = capture.get_pixel(x as usize, y as usize).unwrap();
			writeln!(tmp_file, "{} {} {}", pixel[0], pixel[1], pixel[2]).unwrap();
		}
	}

	tmp_file.flush().expect("Failed to flush.");
}
