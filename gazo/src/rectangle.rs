pub struct Rectangle
{
	pub position: (i32, i32),
	pub size: (i32, i32),
}

pub struct OutputBox(pub Rectangle);
pub struct SelectionBox(pub Rectangle);

impl OutputBox
{
	pub fn selection_intersection_local_coordinates(
		self,
		selection_box: SelectionBox,
	) -> Option<Rectangle>
	{
		let SelectionBox(selection_box_rectangle) = selection_box;
		let OutputBox(output_box_rectangle) = self;

		let mut position: (Option<i32>, Option<i32>) = (None, None);
		let mut size: (Option<i32>, Option<i32>) = (None, None);

		if (output_box_rectangle.position.0
			..=output_box_rectangle.position.0 + output_box_rectangle.size.0)
			.contains(&selection_box_rectangle.position.0)
		{
			position.0 = Some(selection_box_rectangle.position.0);
		}
		else if (selection_box_rectangle.position.0
			..=selection_box_rectangle.position.0 + selection_box_rectangle.size.0)
			.contains(&output_box_rectangle.position.0)
		{
			position.0 = Some(output_box_rectangle.position.0);
		}
		else
		{
			return None;
		}

		if (output_box_rectangle.position.1
			..=output_box_rectangle.position.1 + output_box_rectangle.size.1)
			.contains(&selection_box_rectangle.position.1)
		{
			position.1 = Some(selection_box_rectangle.position.1);
		}
		else if (selection_box_rectangle.position.1
			..=selection_box_rectangle.position.1 + selection_box_rectangle.size.1)
			.contains(&output_box_rectangle.position.1)
		{
			position.1 = Some(output_box_rectangle.position.1);
		}
		else
		{
			return None;
		}

		if (output_box_rectangle.position.0
			..=output_box_rectangle.position.0 + output_box_rectangle.size.0)
			.contains(&(selection_box_rectangle.position.0 + selection_box_rectangle.size.0))
		{
			size.0 = Some(
				selection_box_rectangle.position.0 + selection_box_rectangle.size.0
					- position.0.unwrap(),
			);
		}
		else if (selection_box_rectangle.position.0
			..=selection_box_rectangle.position.0 + selection_box_rectangle.size.0)
			.contains(&(output_box_rectangle.position.0 + output_box_rectangle.size.0))
		{
			size.0 = Some(
				output_box_rectangle.position.0 + output_box_rectangle.size.0 - position.0.unwrap(),
			);
		}
		else
		{
			return None;
		}

		if (output_box_rectangle.position.1
			..=output_box_rectangle.position.1 + output_box_rectangle.size.1)
			.contains(&(selection_box_rectangle.position.1 + selection_box_rectangle.size.1))
		{
			size.1 = Some(
				selection_box_rectangle.position.1 + selection_box_rectangle.size.1
					- position.1.unwrap(),
			);
		}
		else if (selection_box_rectangle.position.1
			..=selection_box_rectangle.position.1 + selection_box_rectangle.size.1)
			.contains(&(output_box_rectangle.position.1 + output_box_rectangle.size.1))
		{
			size.1 = Some(
				output_box_rectangle.position.1 + output_box_rectangle.size.1 - position.1.unwrap(),
			);
		}
		else
		{
			return None;
		}

		Some(Rectangle {
			position: (
				position.0.unwrap() - output_box_rectangle.position.0,
				position.1.unwrap() - output_box_rectangle.position.1,
			),
			size: (size.0.unwrap(), size.1.unwrap()),
		})
	}
}
