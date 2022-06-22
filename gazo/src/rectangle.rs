use std::ops::{Add, Sub};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Position
{
	pub x: i32,
	pub y: i32,
}

impl Position
{
	pub fn new(x: i32, y: i32) -> Self
	{
		Position { x, y }
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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Size
{
	pub width: i32,
	pub height: i32,
}

impl Size
{
	pub fn new(width: i32, height: i32) -> Self
	{
		Size { width, height }
	}
}

impl Add for Size
{
	type Output = Self;

	fn add(self, rhs: Size) -> Self::Output
	{
		Size {
			width: self.width + rhs.width,
			height: self.height + rhs.height,
		}
	}
}

impl Sub for Size
{
	type Output = Self;

	fn sub(self, rhs: Size) -> Self::Output
	{
		Size {
			width: self.width - rhs.width,
			height: self.height - rhs.height,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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

	pub fn get_intersection(self, rectangle: Rectangle) -> Option<Rectangle>
	{
		let mut intersecting_rectangle = Rectangle {
			position: Position { x: 0, y: 0 },
			size: Size {
				width: 0,
				height: 0,
			},
		};

		if (self.position.x..self.position.x + self.size.width).contains(&rectangle.position.x)
		{
			intersecting_rectangle.position.x = rectangle.position.x;
		}
		else if (rectangle.position.x..rectangle.position.x + rectangle.size.width)
			.contains(&self.position.x)
		{
			intersecting_rectangle.position.x = self.position.x;
		}
		else
		{
			return None;
		}

		if (self.position.y..self.position.y + self.size.height).contains(&rectangle.position.y)
		{
			intersecting_rectangle.position.y = rectangle.position.y;
		}
		else if (rectangle.position.y..rectangle.position.y + rectangle.size.height)
			.contains(&self.position.y)
		{
			intersecting_rectangle.position.y = self.position.y;
		}
		else
		{
			return None;
		}

		if (self.position.x..=self.position.x + self.size.width)
			.contains(&(rectangle.position.x + rectangle.size.width))
		{
			intersecting_rectangle.size.width =
				rectangle.position.x + rectangle.size.width - intersecting_rectangle.position.x;
		}
		else if (rectangle.position.x..=rectangle.position.x + rectangle.size.width)
			.contains(&(self.position.x + self.size.width))
		{
			intersecting_rectangle.size.width =
				self.position.x + self.size.width - intersecting_rectangle.position.x;
		}
		else
		{
			return None;
		}

		if (self.position.y..=self.position.y + self.size.height)
			.contains(&(rectangle.position.y + rectangle.size.height))
		{
			intersecting_rectangle.size.height =
				rectangle.position.y + rectangle.size.height - intersecting_rectangle.position.y;
		}
		else if (rectangle.position.y..=rectangle.position.y + rectangle.size.height)
			.contains(&(self.position.y + self.size.height))
		{
			intersecting_rectangle.size.height =
				self.position.y + self.size.height - intersecting_rectangle.position.y;
		}
		else
		{
			return None;
		}

		Some(intersecting_rectangle)
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
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(-1000, 0),
					size: Size::new(500, 500),
				},
				None,
			),
			// ______________
			// |    |  |    |
			// |   2|  |1   |
			// |____|__|____|
			(
				Rectangle {
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(-250, 0),
					size: Size::new(500, 500),
				},
				Some(Rectangle {
					position: Position::new(0, 0),
					size: Size::new(250, 500),
				}),
			),
			// ______________
			// |    |  |    |
			// |   1|  |2   |
			// |____|__|____|
			(
				Rectangle {
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(250, 0),
					size: Size::new(500, 500),
				},
				Some(Rectangle {
					position: Position::new(250, 0),
					size: Size::new(250, 500),
				}),
			),
			// _________   _________
			// |       |   |       |
			// |   1   |   |   2   |
			// |_______|   |_______|
			(
				Rectangle {
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(1000, 0),
					size: Size::new(500, 500),
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
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(0, -1000),
					size: Size::new(500, 500),
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
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(0, -250),
					size: Size::new(500, 500),
				},
				Some(Rectangle {
					position: Position::new(0, 0),
					size: Size::new(500, 250),
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
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(0, 250),
					size: Size::new(500, 500),
				},
				Some(Rectangle {
					position: Position::new(0, 250),
					size: Size::new(500, 250),
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
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(0, 1000),
					size: Size::new(500, 500),
				},
				None,
			),
			// edge cases
		];

		for test_case in test_cases
		{
			assert_eq!(test_case.0.get_intersection(test_case.1), test_case.2);
		}
	}

	#[test]
	fn test_rectangle_get_intersection_edge_cases()
	{
		let test_cases = [
			// top edge
			(
				Rectangle {
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(0, -500),
					size: Size::new(500, 500),
				},
				None,
			),
			// left edge
			(
				Rectangle {
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(500, 0),
					size: Size::new(500, 500),
				},
				None,
			),
			// bottom edge
			(
				Rectangle {
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(0, 500),
					size: Size::new(500, 500),
				},
				None,
			),
			// left edge
			(
				Rectangle {
					position: Position::new(0, 0),
					size: Size::new(500, 500),
				},
				Rectangle {
					position: Position::new(-500, 0),
					size: Size::new(500, 500),
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
