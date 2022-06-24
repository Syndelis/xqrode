use std::{fs, path::PathBuf};

use clap::Parser;
use gazo::ComponentBytes;

#[derive(clap::Parser)]
#[clap(name = "gazo")]
#[clap(author = "redArch <redarch@protonmail.com>")]
#[clap(version = "0.0.2")]
#[clap(about = "Screenshot tool for Wayland compositors", long_about = None)]
struct Cli
{
	#[clap(
		short('g'),
		value_parser,
		value_names(&gazo::Region::get_parser_formats()),
		allow_hyphen_values(true),
		help("Set the region to capture"),
		conflicts_with("output")
	)]
	geometry: Option<gazo::Region>,
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

	// get a capture based on the arguments provided
	let capture = match if cli.geometry.is_some()
	{
		let gazo::Region { position, size } = cli.geometry.unwrap();

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

	// create file to store image
	let file = fs::File::create(cli.output_file).unwrap_or_else(|_| {
		eprintln!("Failed to create the output file.");
		std::process::exit(1);
	});

	// encode to PNG in the file
	let mut encoder = mtpng::encoder::Encoder::new(file, &mtpng::encoder::Options::new());

	let mut header = mtpng::Header::new();

	header
		.set_size(capture.width as u32, capture.height as u32)
		.unwrap();
	header
		.set_color(mtpng::ColorType::TruecolorAlpha, 8)
		.unwrap();

	encoder.write_header(&header).unwrap();

	encoder
		.write_image_rows(capture.pixel_data.as_bytes())
		.unwrap();

	encoder.flush().unwrap();
}
