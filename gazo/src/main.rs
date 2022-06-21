use std::{fs, path::PathBuf};

use clap::Parser;
use regex::Regex;

#[derive(clap::Parser)]
#[clap(name = "gazo")]
#[clap(author = "redArch <redarch@protonmail.com>")]
#[clap(version = "0.0.1")]
#[clap(about = "Screenshot tool for Wayland compositors", long_about = None)]
struct Cli
{
	#[clap(
		short('g'),
		value_parser,
		help("Set the region to capture"),
		conflicts_with("output")
	)]
	geometry: Option<String>,
	#[clap(short('o'), value_parser, help("Set the output name to capture."))]
	output: Option<String>,
	#[clap(short('c'), action, help("Include cursors in the screenshot."))]
	cursor: bool,
	#[clap(value_parser, help("Location to save the image. Image type is PNG."))]
	output_file: PathBuf,
}

fn main()
{
	let cli = Cli::parse();

	let capture = match if cli.geometry.is_some()
	{
		// TODO parse geometry using clap
		let re = Regex::new(r"(-?\d+),(-?\d+) (\d+)x(\d+)").unwrap();

		let captures = re
			.captures(cli.geometry.as_ref().unwrap())
			.expect("Failed to parse geometry.");

		let position = (
			captures.get(1).unwrap().as_str().parse::<i32>().unwrap(),
			captures.get(2).unwrap().as_str().parse::<i32>().unwrap(),
		);
		let size = (
			captures.get(3).unwrap().as_str().parse::<i32>().unwrap(),
			captures.get(4).unwrap().as_str().parse::<i32>().unwrap(),
		);

		gazo::capture_region(position, size, cli.cursor)
	}
	else if cli.output.is_some()
	{
		gazo::capture_output(cli.output.as_ref().unwrap(), cli.cursor)
	}
	else
	{
		gazo::capture_all_outputs(cli.cursor)
	}
	{
		Ok(value) => value,
		Err(error) =>
		{
			eprintln!("There was a problem capturing the screen: {}.", error);
			std::process::exit(1);
		}
	};

	let file = fs::File::create(cli.output_file).unwrap();

	let mut encoder = mtpng::encoder::Encoder::new(file, &mtpng::encoder::Options::new());

	let mut header = mtpng::Header::new();

	header
		.set_size(capture.width as u32, capture.height as u32)
		.unwrap();
	header
		.set_color(mtpng::ColorType::TruecolorAlpha, 8)
		.unwrap();

	encoder.write_header(&header).unwrap();

	encoder.write_image_rows(&capture.pixel_data).unwrap();

	encoder.flush().unwrap();
}
