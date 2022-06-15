use std::path::PathBuf;

use clap::{ArgEnum, Parser};
use regex::Regex;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum ImageType
{
	Png,
	Jpeg,
}

impl From<ImageType> for image::ImageFormat
{
	fn from(item: ImageType) -> Self
	{
		match item
		{
			ImageType::Png => Self::Png,
			ImageType::Jpeg => Self::Jpeg,
		}
	}
}

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
	#[clap(
		short('t'),
		arg_enum,
		value_parser,
		help("Set the output filetype. Defaults to png."),
		default_value("png")
	)]
	image_type: ImageType,
	#[clap(value_parser)]
	output_file: PathBuf,
}

fn main()
{
	let cli = Cli::parse();

	if cli.geometry.is_some()
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

		let capture = gazo::capture_region(position, size, cli.cursor).unwrap();

		let capture_size = capture.get_size_in_pixels();

		let mut image =
			image::RgbaImage::new(capture_size.width as u32, capture_size.height as u32);

		for x in 0..capture_size.width
		{
			for y in 0..capture_size.height
			{
				image.put_pixel(
					x as u32,
					y as u32,
					image::Rgba(capture.get_pixel(x as usize, y as usize)),
				);
			}
		}

		image
			.save_with_format(cli.output_file, cli.image_type.into())
			.expect("Error saving image.");
	}
	else if cli.output.is_some()
	{
		//
	}
	else
	{
		// TODO
	}
}
