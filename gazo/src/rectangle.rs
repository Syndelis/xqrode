use std::ops::{Add, Sub};

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Position
{
	pub x: i32,
	pub y: i32,
}

impl Position
{
	pub fn new(position: (i32, i32)) -> Self
	{
		Position {
			x: position.0,
			y: position.1,
		}
	}
}

impl Add for Position
{
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output
	{
		Position {
			x: self.x + rhs.x,
			y: self.y + rhs.y,
		}
	}
}

impl Sub for Position
{
	type Output = Self;

	fn sub(self, rhs: Self) -> Self::Output
	{
		Position {
			x: self.x - rhs.x,
			y: self.y - rhs.y,
		}
	}
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Size
{
	pub width: i32,
	pub height: i32,
}

impl Size
{
	pub fn new(size: (i32, i32)) -> Self
	{
		Size {
			width: size.0,
			height: size.1,
		}
	}
}

impl Add<Size> for Position
{
	type Output = Self;

	fn add(self, rhs: Size) -> Self::Output
	{
		Position {
			x: self.x + rhs.width,
			y: self.y + rhs.height,
		}
	}
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Rectangle
{
	pub position: Position,
	pub size: Size,
}

impl Rectangle
{
	pub fn new(position: Position, size: Size) -> Self
	{
		Rectangle { position, size }
	}

	// TODO: test this
	pub fn position_falls_within(&self, position: Position) -> bool
	{
		if (position.x >= self.position.x && position.x < self.position.x + self.size.width)
			&& (position.y >= self.position.y && position.y < self.position.y + self.size.height)
		{
			true
		}
		else
		{
			false
		}
	}

	pub fn get_intersection(self, rectangle: Rectangle) -> Option<Rectangle>
	{
		if self.size.width <= 0
			|| self.size.height <= 0
			|| rectangle.size.width <= 0
			|| rectangle.size.height <= 0
		{
			return None;
		}

		let mut position: (Option<i32>, Option<i32>) = (None, None);
		let mut size: (Option<i32>, Option<i32>) = (None, None);

		if (self.position.x..=self.position.x + self.size.width).contains(&rectangle.position.x)
		{
			position.0 = Some(rectangle.position.x);
		}
		else if (rectangle.position.x..=rectangle.position.x + rectangle.size.width)
			.contains(&self.position.x)
		{
			position.0 = Some(self.position.x);
		}
		else
		{
			return None;
		}

		if (self.position.y..=self.position.y + self.size.height).contains(&rectangle.position.y)
		{
			position.1 = Some(rectangle.position.y);
		}
		else if (rectangle.position.y..=rectangle.position.y + rectangle.size.height)
			.contains(&self.position.y)
		{
			position.1 = Some(self.position.y);
		}
		else
		{
			return None;
		}

		if (self.position.x..=self.position.x + self.size.width)
			.contains(&(rectangle.position.x + rectangle.size.width))
		{
			size.0 = Some(rectangle.position.x + rectangle.size.width - position.0.unwrap());
		}
		else if (rectangle.position.x..=rectangle.position.x + rectangle.size.width)
			.contains(&(self.position.x + self.size.width))
		{
			size.0 = Some(self.position.x + self.size.width - position.0.unwrap());
		}
		else
		{
			return None;
		}

		if (self.position.y..=self.position.y + self.size.height)
			.contains(&(rectangle.position.y + rectangle.size.height))
		{
			size.1 = Some(rectangle.position.y + rectangle.size.height - position.1.unwrap());
		}
		else if (rectangle.position.y..=rectangle.position.y + rectangle.size.height)
			.contains(&(self.position.y + self.size.height))
		{
			size.1 = Some(self.position.y + self.size.height - position.1.unwrap());
		}
		else
		{
			return None;
		}

		Some(Rectangle {
			position: Position {
				x: position.0.unwrap(),
				y: position.1.unwrap(),
			},
			size: Size {
				width: size.0.unwrap(),
				height: size.1.unwrap(),
			},
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
					position: Position::new((0, 0)),
					size: Size::new((500, 500)),
				},
				Rectangle {
					position: Position::new((-1000, 0)),
					size: Size::new((500, 500)),
				},
				None,
			),
			// ______________
			// |    |  |    |
			// |   2|  |1   |
			// |____|__|____|
			(
				Rectangle {
					position: Position::new((0, 0)),
					size: Size::new((500, 500)),
				},
				Rectangle {
					position: Position::new((-250, 0)),
					size: Size::new((500, 500)),
				},
				Some(Rectangle {
					position: Position::new((0, 0)),
					size: Size::new((250, 500)),
				}),
			),
			// ______________
			// |    |  |    |
			// |   1|  |2   |
			// |____|__|____|
			(
				Rectangle {
					position: Position::new((0, 0)),
					size: Size::new((500, 500)),
				},
				Rectangle {
					position: Position::new((250, 0)),
					size: Size::new((500, 500)),
				},
				Some(Rectangle {
					position: Position::new((250, 0)),
					size: Size::new((250, 500)),
				}),
			),
			// _________   _________
			// |       |   |       |
			// |   1   |   |   2   |
			// |_______|   |_______|
			(
				Rectangle {
					position: Position::new((0, 0)),
					size: Size::new((500, 500)),
				},
				Rectangle {
					position: Position::new((1000, 0)),
					size: Size::new((500, 500)),
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
					position: Position::new((0, 0)),
					size: Size::new((500, 500)),
				},
				Rectangle {
					position: Position::new((0, -1000)),
					size: Size::new((500, 500)),
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
					position: Position::new((0, 0)),
					size: Size::new((500, 500)),
				},
				Rectangle {
					position: Position::new((0, -250)),
					size: Size::new((500, 500)),
				},
				Some(Rectangle {
					position: Position::new((0, 0)),
					size: Size::new((500, 250)),
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
					position: Position::new((0, 0)),
					size: Size::new((500, 500)),
				},
				Rectangle {
					position: Position::new((0, 250)),
					size: Size::new((500, 500)),
				},
				Some(Rectangle {
					position: Position::new((0, 250)),
					size: Size::new((500, 250)),
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
					position: Position::new((0, 0)),
					size: Size::new((500, 500)),
				},
				Rectangle {
					position: Position::new((0, 1000)),
					size: Size::new((500, 500)),
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
