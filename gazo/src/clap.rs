// this module contains the Region type, which implements parsing for clap
use clap::builder::{self, TypedValueParser};
use regex::Regex;

/// A type that implements argument parsing for the [`clap`] crate for a region
/// in the form `{x},{y} {width}x{height}` (this is the default format of
/// <a href = "https://github.com/emersion/slurp" target = "_blank">slurp</a>). Enable the `clap-region-parsing` feature to use this struct, then
/// give it to clap as an argument type and it will parse it.
#[derive(Clone)]
pub struct Region
{
	/// The position of the top left corner of the region. This can be used
	/// directly as the position argument for
	/// [capture_region](crate::capture_region).
	pub position: (i32, i32),
	/// The size of the region expanding down and left from the position. This
	/// can be used directly as the size argument for
	/// [capture_region](crate::capture_region)
	pub size: (i32, i32),
}

impl Region
{
	/// Returns a slice of formats that can be parsed into a [`Region`].
	pub const fn get_parser_formats() -> [&'static str; 1]
	{
		["{x},{y} {width}x{height}"]
	}
}

impl builder::ValueParserFactory for Region
{
	type Parser = RegionValueParser;

	fn value_parser() -> Self::Parser
	{
		RegionValueParser
	}
}

#[derive(Clone)]
pub struct RegionValueParser;

impl TypedValueParser for RegionValueParser
{
	type Value = Region;

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

		let re = Regex::new(r"(-?\d+),(-?\d+) (\d+)x(\d+)").unwrap();

		let captures = re.captures(value).ok_or_else(|| {
			clap::Error::raw(
				clap::ErrorKind::ValueValidation,
				"The argument was malformed. Please use the format: '{x},{y} {width}x{height}'.",
			)
		})?;

		let position = (
			captures.get(1).unwrap().as_str().parse::<i32>().unwrap(),
			captures.get(2).unwrap().as_str().parse::<i32>().unwrap(),
		);
		let size = (
			captures.get(3).unwrap().as_str().parse::<i32>().unwrap(),
			captures.get(4).unwrap().as_str().parse::<i32>().unwrap(),
		);

		Ok(Region { position, size })
	}
}
