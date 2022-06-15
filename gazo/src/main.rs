use std::{fs, path::PathBuf, time};

use clap::{ArgEnum, Parser};
use image::ImageEncoder;
use regex::Regex;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum ImageType
{
	Png,
	Jpeg,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum Level
{
	Fast,
	Best,
	Huffman,
	Rle,
}

impl From<Level> for image::codecs::png::CompressionType
{
	fn from(item: Level) -> Self
	{
		match item
		{
			Level::Fast => Self::Fast,
			Level::Best => Self::Best,
			Level::Huffman => Self::Huffman,
			Level::Rle => Self::Rle,
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
		help("Set the output filetype."),
		default_value("png")
	)]
	image_type: ImageType,
	#[clap(value_parser)]
	output_file: PathBuf,
	#[clap(
		short('l'),
		value_parser,
		help("Set the PNG filetype compression level."),
		default_value("fast")
	)]
	level: Level,
}

fn main()
{
	let time = time::Instant::now();
	let cli = Cli::parse();

	let capture = if cli.geometry.is_some()
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

		gazo::capture_region(position, size, cli.cursor).unwrap()
	}
	else if cli.output.is_some()
	{
		// TODO
		panic!("UNIMPLEMENTED");
	}
	else
	{
		gazo::capture_all_outputs(cli.cursor).unwrap()
	};

	println!("Time to get capture: {:?}", time.elapsed());

	let capture_size = capture.get_size_in_pixels();

	let mut image_buffer: Vec<u8> =
		Vec::with_capacity((capture_size.width * capture_size.height * 4) as usize);

	for y in 0..capture_size.height
	{
		for x in 0..capture_size.width
		{
			for channel in capture.get_pixel(x as usize, y as usize)
			{
				image_buffer.push(channel);
			}
		}
	}

	println!("Time to read capture into buffer: {:?}", time.elapsed());

	let file = fs::File::create(cli.output_file).unwrap();

	match cli.image_type
	{
		ImageType::Png =>
		{
			image::codecs::png::PngEncoder::new_with_quality(
				file,
				cli.level.into(),
				image::codecs::png::FilterType::Adaptive,
			)
			.write_image(
				&image_buffer,
				capture_size.width as u32,
				capture_size.height as u32,
				image::ColorType::Rgba8,
			)
			.unwrap();
		}
		ImageType::Jpeg =>
		{
			image::codecs::jpeg::JpegEncoder::new(file)
				.write_image(
					&image_buffer,
					capture_size.width as u32,
					capture_size.height as u32,
					image::ColorType::Rgba8,
				)
				.unwrap();
		}
	}

	println!("Time to encode and write: {:?}", time.elapsed());
}
