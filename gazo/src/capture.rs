use super::rectangle;

#[derive(Debug)]
pub struct Capture
{
	output_infos: Vec<super::OutputInfo>,
	capture_rectangle_absolute: rectangle::Rectangle,
	current_position_absolute: rectangle::Position,
}

impl Capture
{
	pub(crate) fn new(output_infos: Vec<super::OutputInfo>) -> Option<Self>
	{
		if output_infos.len() < 1
		{
			return None;
		}

		let mut upper_left = output_infos[0].image_position_absolute.unwrap();
		let mut bottom_right = upper_left + output_infos[0].image_size.unwrap();

		for output_info in &output_infos[1..]
		{
			let image_position = output_info.image_position_absolute.unwrap();
			let image_size = output_info.image_size.unwrap();

			if image_position.x < upper_left.x
			{
				upper_left.x = image_position.x;
			}

			if image_position.y < upper_left.y
			{
				upper_left.y = image_position.y;
			}

			if image_position.x + image_size.width > bottom_right.x
			{
				bottom_right.x = image_position.x + image_size.width;
			}

			if image_position.y + image_size.height > bottom_right.y
			{
				bottom_right.y = image_position.y + image_size.height;
			}
		}

		Some(Capture {
			output_infos,
			capture_rectangle_absolute: rectangle::Rectangle {
				position: upper_left,
				size: rectangle::Size {
					width: bottom_right.x - upper_left.x,
					height: bottom_right.y - upper_left.y,
				},
			},
			current_position_absolute: upper_left,
		})
	}

	pub fn get_pixel_size(&self) -> rectangle::Size
	{
		self.capture_rectangle_absolute.size
	}
}

impl Iterator for Capture
{
	type Item = [u8; 4];

	fn next(&mut self) -> Option<Self::Item>
	{
		// store current position for this call
		let current_position_absolute = self.current_position_absolute;

		// increment position for next call
		self.current_position_absolute.x += 1;

		if self.current_position_absolute.x
			>= self.capture_rectangle_absolute.position.x
				+ self.capture_rectangle_absolute.size.width
		{
			self.current_position_absolute.x = self.capture_rectangle_absolute.position.x;
			self.current_position_absolute.y += 1;
		}

		// find the output for this position
		for output_info in &self.output_infos
		{
			let rectangle = rectangle::Rectangle {
				position: output_info.image_position_absolute.unwrap(),
				size: output_info.image_size.unwrap(),
			};

			if rectangle.position_falls_within(current_position_absolute)
			{
				let index = ((current_position_absolute.x
					- output_info.image_position_absolute.unwrap().x)
					+ ((current_position_absolute.y
						- output_info.image_position_absolute.unwrap().y)
						* output_info.image_size.unwrap().width))
					* 4;

				return Some([
					output_info.image_mmap.as_ref().unwrap()[index as usize], // R
					output_info.image_mmap.as_ref().unwrap()[(index + 1) as usize], // G
					output_info.image_mmap.as_ref().unwrap()[(index + 2) as usize], // B
					255,                                                      // A
				]);
			}
		}

		// outside screencopies, but within capture region
		// so should be transparent
		if self
			.capture_rectangle_absolute
			.position_falls_within(current_position_absolute)
		{
			Some([0, 0, 0, 0])
		}
		else
		{
			println!("OUTSIDE");
			None
		}
	}
}
