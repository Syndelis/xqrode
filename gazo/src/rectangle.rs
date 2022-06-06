#[derive(Debug, PartialEq)]
pub struct Rectangle
{
	pub position: (i32, i32),
	pub size: (i32, i32),
}

impl Rectangle
{
	pub fn new(position: (i32, i32), size: (i32, i32)) -> Self
	{
		Rectangle { position, size }
	}

	pub fn get_intersection(self, rectangle: Rectangle) -> Option<Rectangle>
	{
		if self.size.0 <= 0 || self.size.1 <= 0 || rectangle.size.0 <= 0 || rectangle.size.1 <= 0
		{
			return None;
		}

		let mut position: (Option<i32>, Option<i32>) = (None, None);
		let mut size: (Option<i32>, Option<i32>) = (None, None);

		if (self.position.0..=self.position.0 + self.size.0).contains(&rectangle.position.0)
		{
			position.0 = Some(rectangle.position.0);
		}
		else if (rectangle.position.0..=rectangle.position.0 + rectangle.size.0)
			.contains(&self.position.0)
		{
			position.0 = Some(self.position.0);
		}
		else
		{
			return None;
		}

		if (self.position.1..=self.position.1 + self.size.1).contains(&rectangle.position.1)
		{
			position.1 = Some(rectangle.position.1);
		}
		else if (rectangle.position.1..=rectangle.position.1 + rectangle.size.1)
			.contains(&self.position.1)
		{
			position.1 = Some(self.position.1);
		}
		else
		{
			return None;
		}

		if (self.position.0..=self.position.0 + self.size.0)
			.contains(&(rectangle.position.0 + rectangle.size.0))
		{
			size.0 = Some(rectangle.position.0 + rectangle.size.0 - position.0.unwrap());
		}
		else if (rectangle.position.0..=rectangle.position.0 + rectangle.size.0)
			.contains(&(self.position.0 + self.size.0))
		{
			size.0 = Some(self.position.0 + self.size.0 - position.0.unwrap());
		}
		else
		{
			return None;
		}

		if (self.position.1..=self.position.1 + self.size.1)
			.contains(&(rectangle.position.1 + rectangle.size.1))
		{
			size.1 = Some(rectangle.position.1 + rectangle.size.1 - position.1.unwrap());
		}
		else if (rectangle.position.1..=rectangle.position.1 + rectangle.size.1)
			.contains(&(self.position.1 + self.size.1))
		{
			size.1 = Some(self.position.1 + self.size.1 - position.1.unwrap());
		}
		else
		{
			return None;
		}

		Some(Rectangle {
			position: (position.0.unwrap(), position.1.unwrap()),
			size: (size.0.unwrap(), size.1.unwrap()),
		})
	}
}

#[cfg(test)]
mod tests
{
	use super::*;

	// test the differenct execution paths
	#[test]
	fn test_rectangle_get_intersection()
	{
		let test_cases = [
			// horizontal
			// _________   _________
			// |       |   |       |
			// |   2   |   |   1   |
			// |_______|   |_______|
			(
				Rectangle {
					position: (0, 0),
					size: (500, 500),
				},
				Rectangle {
					position: (-1000, 0),
					size: (500, 500),
				},
				None,
			),
			// ______________
			// |    |  |    |
			// |   2|  |1   |
			// |____|__|____|
			(
				Rectangle {
					position: (0, 0),
					size: (500, 500),
				},
				Rectangle {
					position: (-250, 0),
					size: (500, 500),
				},
				Some(Rectangle {
					position: (0, 0),
					size: (250, 500),
				}),
			),
			// ______________
			// |    |  |    |
			// |   1|  |2   |
			// |____|__|____|
			(
				Rectangle {
					position: (0, 0),
					size: (500, 500),
				},
				Rectangle {
					position: (250, 0),
					size: (500, 500),
				},
				Some(Rectangle {
					position: (250, 0),
					size: (250, 500),
				}),
			),
			// _________   _________
			// |       |   |       |
			// |   1   |   |   2   |
			// |_______|   |_______|
			(
				Rectangle {
					position: (0, 0),
					size: (500, 500),
				},
				Rectangle {
					position: (1000, 0),
					size: (500, 500),
				},
				None,
			),
			// vertical
			// _________
			// |       |
			// |   2   |
			// |_______|
			// _________
			// |       |
			// |   1   |
			// |_______|
			(
				Rectangle {
					position: (0, 0),
					size: (500, 500),
				},
				Rectangle {
					position: (0, -1000),
					size: (500, 500),
				},
				None,
			),
			// _________
			// |       |
			// |___2___|
			// |_______|
			// |   1   |
			// |_______|
			(
				Rectangle {
					position: (0, 0),
					size: (500, 500),
				},
				Rectangle {
					position: (0, -250),
					size: (500, 500),
				},
				Some(Rectangle {
					position: (0, 0),
					size: (500, 250),
				}),
			),
			// _________
			// |       |
			// |___1___|
			// |_______|
			// |   2   |
			// |_______|
			(
				Rectangle {
					position: (0, 0),
					size: (500, 500),
				},
				Rectangle {
					position: (0, 250),
					size: (500, 500),
				},
				Some(Rectangle {
					position: (0, 250),
					size: (500, 250),
				}),
			),
			// _________
			// |       |
			// |   1   |
			// |_______|
			// _________
			// |       |
			// |   2   |
			// |_______|
			(
				Rectangle {
					position: (0, 0),
					size: (500, 500),
				},
				Rectangle {
					position: (0, 1000),
					size: (500, 500),
				},
				None,
			),
		];

		for test_case in test_cases
		{
			assert_eq!(test_case.0.get_intersection(test_case.1), test_case.2);
		}
	}
}
