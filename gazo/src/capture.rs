use std::cmp;

use crate::rectangle;

pub trait SingleCapture
{
	fn get_position(&self) -> rectangle::Position;

	fn get_size(&self) -> rectangle::Size;

	fn get_pixel(&self, position: rectangle::Position) -> [u8; 4];
}

#[derive(Debug)]
pub struct FullCapture<T: SingleCapture>
{
	captures: Vec<T>,
	capture_rectangle_absolute: rectangle::Rectangle,
}

/// This struct represents the entire contents of a capture.
impl<T: SingleCapture> FullCapture<T>
{
	pub(crate) fn new(captures: Vec<T>) -> Result<Self, crate::Error>
	{
		if captures.is_empty()
		{
			return Err(crate::Error::NoCaptures);
		}

		let mut upper_left = captures[0].get_position();
		let mut bottom_right = upper_left + captures[0].get_size();

		for capture in &captures[1..]
		{
			upper_left.x = cmp::min(capture.get_position().x, upper_left.x);

			upper_left.y = cmp::min(capture.get_position().y, upper_left.y);

			bottom_right.x = cmp::max(
				capture.get_position().x + capture.get_size().width,
				bottom_right.x,
			);

			bottom_right.y = cmp::max(
				capture.get_position().y + capture.get_size().height,
				bottom_right.y,
			);
		}

		Ok(FullCapture {
			captures,
			capture_rectangle_absolute: rectangle::Rectangle {
				position: upper_left,
				size: rectangle::Size {
					width: bottom_right.x - upper_left.x,
					height: bottom_right.y - upper_left.y,
				},
			},
		})
	}

	/// Returns the actual size of the capture in pixels. Use this to loop
	/// through the capture coordinates without going out of bounds.
	pub fn get_size_in_pixels(&self) -> rectangle::Size
	{
		self.capture_rectangle_absolute.size
	}

	/// Returns a pixel in the RGBA big endian format. This function will cause
	/// a panic if the provided coordinate falls outside the capture. The memory
	/// location must be calulated each call, so when multile accesses are
	/// needed it is recommended to read the pixels into contiguous memory
	/// first.
	pub fn get_pixel(&self, x: usize, y: usize) -> [u8; 4]
	{
		let position_absolute = rectangle::Position::new((
			self.capture_rectangle_absolute.position.x + x as i32,
			self.capture_rectangle_absolute.position.y + y as i32,
		));

		for capture in &self.captures
		{
			let rectangle = rectangle::Rectangle {
				position: capture.get_position(),
				size: capture.get_size(),
			};

			if rectangle.position_falls_within(position_absolute)
			{
				return capture.get_pixel(position_absolute);
			}
		}

		// outside screencopies, but within capture region
		// so should be transparent
		if self
			.capture_rectangle_absolute
			.position_falls_within(position_absolute)
		{
			[0, 0, 0, 0]
		}
		else
		{
			panic!("Coordinate fell outside of capture");
		}
	}
}
