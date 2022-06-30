pub mod xwrap;
pub mod util;

use image::{RgbaImage, Rgba, GenericImage, Pixel};
use regex::Regex;
pub use x11::xlib;
pub use image;
pub use util::Rect;

use clap::builder::{self, TypedValueParser};

impl Rect {

    pub const fn get_parser_formats() -> [&'static str; 1] {
		["{w}x{h}+{x}+{y}"]
	}

}

impl builder::ValueParserFactory for Rect {

	type Parser = RectValueParser;

	fn value_parser() -> Self::Parser {
		RectValueParser
	}

}

#[derive(Clone)]
pub struct RectValueParser;

impl TypedValueParser for RectValueParser {
	type Value = Rect;

	fn parse_ref(
		&self,
		_command: &clap::Command,
		_argument: Option<&clap::Arg>,
		value: &std::ffi::OsStr,
	) -> Result<Self::Value, clap::Error>
	{
		if value.is_empty()
		{
			return Err(clap::Error::raw(
				clap::ErrorKind::EmptyValue,
				"The region argument must not be empty.",
			));
		}

		let value = value.to_str().ok_or_else(|| {
			clap::Error::raw(
				clap::ErrorKind::InvalidUtf8,
				"The argument containted invalid UTF-8 characters.",
			)
		})?;

		let regex = Regex::new(r"(\d+)x(\d+)\+(\d+)\+(\d+)").unwrap();

		let captures = regex.captures(value).ok_or_else(|| {
			clap::Error::raw(
				clap::ErrorKind::ValueValidation,
				"The argument was malformed. Please use the format: '{w}x{h}+{x}+{y}'.",
			)
		})?;

		// should be safe to unwrap as the regex should only match on valid numbers
		let w = captures.get(1).unwrap().as_str().parse::<i32>().unwrap();
		let h = captures.get(2).unwrap().as_str().parse::<i32>().unwrap();
		let x = captures.get(3).unwrap().as_str().parse::<i32>().unwrap();
		let y = captures.get(4).unwrap().as_str().parse::<i32>().unwrap();

		Ok(Rect { x, y, w, h })
	}
}

pub fn capture_region(rect: Rect) -> image::DynamicImage {

	let display = match xwrap::Display::open(None) {
		Some(d) => d,
		None => {
			eprintln!("Failed to open display");
			std::process::exit(1);
		}
	};

	let root = display.get_default_root();
	let window_rect = display.get_window_rect(root);

	let sel = match rect.intersection(window_rect) {

		Some(sel) => Rect {
			x: sel.x - window_rect.x,
			y: sel.y - window_rect.y,
			w: sel.w,
			h: sel.h,
		},

		None => {
			eprintln!("Region is outside of the screen");
			std::process::exit(1);
		}

	};

	let image = match display.get_image(root, sel, xwrap::ALL_PLANES, xlib::ZPixmap) {
		Some(i) => i,
		None => {
			eprintln!("Failed to get image from X");
			std::process::exit(1);
		}
	};

	let mut image = match image.into_image_buffer() {
		Some(i) => image::DynamicImage::ImageRgba8(i),
		None => {
			eprintln!("Failed to convert captured framebuffer, only 24/32 \
						bit (A)RGB8 is supported");
			std::process::exit(1);
		}
	};

	match display.get_screen_rects(root) {
		Some(screens) => {
			let screens: Vec<Rect> =
				screens.filter_map(|s| s.intersection(sel)).collect();

			if screens.len() > 1 {
				let mut masked = RgbaImage::from_pixel(
					sel.w as u32, sel.h as u32,
					Rgba::from_channels(0, 0, 0, 0)
				);

				for screen in screens {
					let sub = Rect {
						x: screen.x - sel.x,
						y: screen.y - sel.y,
						w: screen.w,
						h: screen.h,
					};

					let mut sub_src = image.sub_image(
						sub.x as u32, sub.y as u32,
						sub.w as u32, sub.h as u32
					);

					masked.copy_from(&mut sub_src, sub.x as u32, sub.y as u32)
						.expect("Failed to copy sub-image");
				}

				image = image::DynamicImage::ImageRgba8(masked);

			}
		},
		None => eprintln!("Failed to enumerate screens, not masking")
	};

	image

}