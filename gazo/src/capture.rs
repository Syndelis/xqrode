use wayland_client::protocol::wl_output;

pub(crate) struct OutputCapture
{
	pub transform: wl_output::Transform,
	pub image_mmap: memmap2::MmapMut,
	pub image_mmap_size: crate::rectangle::Size,
	pub image_logical_position: crate::rectangle::Position,
	pub image_logical_size: crate::rectangle::Size,
	pub image_pixel_format: crate::wayland::PixelFormat,
}

impl OutputCapture
{
	pub fn get_transform_corrected_buffer(self) -> (usize, usize, memmap2::MmapMut)
	{
		let mut mmap = memmap2::MmapMut::map_anon(
			self.image_mmap_size.width as usize * self.image_mmap_size.height as usize * 4,
		)
		.unwrap();

		let transform = self.transform;

		// width and height are switched odd numbered rotations (1 90 degree, or 3 90
		// degree)
		let (transformed_width, transformed_height) = match transform
		{
			wl_output::Transform::_90
			| wl_output::Transform::_270
			| wl_output::Transform::Flipped90
			| wl_output::Transform::Flipped270 => (self.image_mmap_size.height, self.image_mmap_size.width),
			_ => (self.image_mmap_size.width, self.image_mmap_size.height),
		};

		// loop over buffer in local coordinate space
		for y in 0..self.image_mmap_size.height
		{
			for x in 0..self.image_mmap_size.width
			{
				// the index in the new mmap
				let destination_index = {
					// apply clock wise rotation transformation and calculate index
					let (x, y) = match transform
					{
						wl_output::Transform::Normal | wl_output::Transform::Flipped => (x, y),
						wl_output::Transform::_90 | wl_output::Transform::Flipped90 =>
						{
							(self.image_mmap_size.height - y - 1, x)
						}
						wl_output::Transform::_180 | wl_output::Transform::Flipped180 =>
						{
							(
								self.image_mmap_size.width - x - 1,
								self.image_mmap_size.height - y - 1,
							)
						}
						wl_output::Transform::_270 | wl_output::Transform::Flipped270 =>
						{
							(y, self.image_mmap_size.width - x - 1)
						}
						_ =>
						{
							panic!(
								"Unimplemented transform found, please report this to the Gazo \
								 crate."
							);
						}
					};

					// conditionally calculate index for flipped variants
					(match transform
					{
						wl_output::Transform::Flipped
						| wl_output::Transform::Flipped90
						| wl_output::Transform::Flipped180
						| wl_output::Transform::Flipped270 =>
						{
							((y * transformed_width) + (transformed_width - x - 1)) * 4
						}
						_ => ((y * transformed_width) + x) * 4,
					}) as usize
				};

				// the index of the mmap in self
				let source_index = (((y * self.image_mmap_size.width) + x) * 4) as usize;

				// transform the pixel to Rgba8888
				let transformed_pixel = Self::transform_pixel(self.image_pixel_format, unsafe {
					self.image_mmap
						.get_unchecked(source_index..(source_index + 4))
				});

				// put the pixel in the new mmap at the correct index
				mmap[destination_index] = transformed_pixel[0];
				mmap[destination_index + 1] = transformed_pixel[1];
				mmap[destination_index + 2] = transformed_pixel[2];
				mmap[destination_index + 3] = transformed_pixel[3];
			}
		}

		// return the width, height, and mmap as the width and height will be corrected
		// for some transforms
		(
			transformed_width as usize,
			transformed_height as usize,
			mmap,
		)
	}

	// turn image pixel format into Rgba8888
	fn transform_pixel(image_pixel_format: crate::wayland::PixelFormat, pixel: &[u8]) -> [u8; 4]
	{
		match image_pixel_format
		{
			crate::wayland::PixelFormat::Argb8888 => [pixel[2], pixel[1], pixel[0], pixel[3]],
			crate::wayland::PixelFormat::Xbgr8888 => [pixel[0], pixel[1], pixel[2], 255],
			crate::wayland::PixelFormat::Xrgb8888 => [pixel[2], pixel[1], pixel[0], 255],
		}
	}
}

// TODO test pixel format adjustments
#[cfg(test)]
mod tests
{
	use super::*;

	// test the validity of the transformations
	#[test]
	fn test_transformation_odd_row_column_square()
	{
		// the test case input has the opposite transformation applied to this data, so
		// this should be the result when the transformation is applied
		let expected_result = vec![
			vec![
				[0, 0, 0, 255],
				[1, 1, 1, 255],
				[2, 2, 2, 255],
				[3, 3, 3, 255],
				[4, 4, 4, 255],
			],
			vec![
				[5, 5, 5, 255],
				[6, 6, 6, 255],
				[7, 7, 7, 255],
				[8, 8, 8, 255],
				[9, 9, 9, 255],
			],
			vec![
				[10, 10, 10, 255],
				[11, 11, 11, 255],
				[12, 12, 12, 255],
				[13, 13, 13, 255],
				[14, 14, 14, 255],
			],
			vec![
				[15, 15, 15, 255],
				[16, 16, 16, 255],
				[17, 17, 17, 255],
				[18, 18, 18, 255],
				[19, 19, 19, 255],
			],
			vec![
				[20, 20, 20, 255],
				[21, 21, 21, 255],
				[22, 22, 22, 255],
				[23, 23, 23, 255],
				[24, 24, 24, 255],
			],
		];

		run_transformation_test(
			wl_output::Transform::Normal,
			vec![
				vec![
					[0, 0, 0, 255],
					[1, 1, 1, 255],
					[2, 2, 2, 255],
					[3, 3, 3, 255],
					[4, 4, 4, 255],
				],
				vec![
					[5, 5, 5, 255],
					[6, 6, 6, 255],
					[7, 7, 7, 255],
					[8, 8, 8, 255],
					[9, 9, 9, 255],
				],
				vec![
					[10, 10, 10, 255],
					[11, 11, 11, 255],
					[12, 12, 12, 255],
					[13, 13, 13, 255],
					[14, 14, 14, 255],
				],
				vec![
					[15, 15, 15, 255],
					[16, 16, 16, 255],
					[17, 17, 17, 255],
					[18, 18, 18, 255],
					[19, 19, 19, 255],
				],
				vec![
					[20, 20, 20, 255],
					[21, 21, 21, 255],
					[22, 22, 22, 255],
					[23, 23, 23, 255],
					[24, 24, 24, 255],
				],
			],
			expected_result.clone(),
		);

		run_transformation_test(
			wl_output::Transform::_90,
			vec![
				vec![
					[4, 4, 4, 255],
					[9, 9, 9, 255],
					[14, 14, 14, 255],
					[19, 19, 19, 255],
					[24, 24, 24, 255],
				],
				vec![
					[3, 3, 3, 255],
					[8, 8, 8, 255],
					[13, 13, 13, 255],
					[18, 18, 18, 255],
					[23, 23, 23, 255],
				],
				vec![
					[2, 2, 2, 255],
					[7, 7, 7, 255],
					[12, 12, 12, 255],
					[17, 17, 17, 255],
					[22, 22, 22, 255],
				],
				vec![
					[1, 1, 1, 255],
					[6, 6, 6, 255],
					[11, 11, 11, 255],
					[16, 16, 16, 255],
					[21, 21, 21, 255],
				],
				vec![
					[0, 0, 0, 255],
					[5, 5, 5, 255],
					[10, 10, 10, 255],
					[15, 15, 15, 255],
					[20, 20, 20, 255],
				],
			],
			expected_result.clone(),
		);

		run_transformation_test(
			wl_output::Transform::_180,
			vec![
				vec![
					[24, 24, 24, 255],
					[23, 23, 23, 255],
					[22, 22, 22, 255],
					[21, 21, 21, 255],
					[20, 20, 20, 255],
				],
				vec![
					[19, 19, 19, 255],
					[18, 18, 18, 255],
					[17, 17, 17, 255],
					[16, 16, 16, 255],
					[15, 15, 15, 255],
				],
				vec![
					[14, 14, 14, 255],
					[13, 13, 13, 255],
					[12, 12, 12, 255],
					[11, 11, 11, 255],
					[10, 10, 10, 255],
				],
				vec![
					[9, 9, 9, 255],
					[8, 8, 8, 255],
					[7, 7, 7, 255],
					[6, 6, 6, 255],
					[5, 5, 5, 255],
				],
				vec![
					[4, 4, 4, 255],
					[3, 3, 3, 255],
					[2, 2, 2, 255],
					[1, 1, 1, 255],
					[0, 0, 0, 255],
				],
			],
			expected_result.clone(),
		);

		run_transformation_test(
			wl_output::Transform::_270,
			vec![
				vec![
					[20, 20, 20, 255],
					[15, 15, 15, 255],
					[10, 10, 10, 255],
					[5, 5, 5, 255],
					[0, 0, 0, 255],
				],
				vec![
					[21, 21, 21, 255],
					[16, 16, 16, 255],
					[11, 11, 11, 255],
					[6, 6, 6, 255],
					[1, 1, 1, 255],
				],
				vec![
					[22, 22, 22, 255],
					[17, 17, 17, 255],
					[12, 12, 12, 255],
					[7, 7, 7, 255],
					[2, 2, 2, 255],
				],
				vec![
					[23, 23, 23, 255],
					[18, 18, 18, 255],
					[13, 13, 13, 255],
					[8, 8, 8, 255],
					[3, 3, 3, 255],
				],
				vec![
					[24, 24, 24, 255],
					[19, 19, 19, 255],
					[14, 14, 14, 255],
					[9, 9, 9, 255],
					[4, 4, 4, 255],
				],
			],
			expected_result.clone(),
		);

		run_transformation_test(
			wl_output::Transform::Flipped,
			vec![
				vec![
					[4, 4, 4, 255],
					[3, 3, 3, 255],
					[2, 2, 2, 255],
					[1, 1, 1, 255],
					[0, 0, 0, 255],
				],
				vec![
					[9, 9, 9, 255],
					[8, 8, 8, 255],
					[7, 7, 7, 255],
					[6, 6, 6, 255],
					[5, 5, 5, 255],
				],
				vec![
					[14, 14, 14, 255],
					[13, 13, 13, 255],
					[12, 12, 12, 255],
					[11, 11, 11, 255],
					[10, 10, 10, 255],
				],
				vec![
					[19, 19, 19, 255],
					[18, 18, 18, 255],
					[17, 17, 17, 255],
					[16, 16, 16, 255],
					[15, 15, 15, 255],
				],
				vec![
					[24, 24, 24, 255],
					[23, 23, 23, 255],
					[22, 22, 22, 255],
					[21, 21, 21, 255],
					[20, 20, 20, 255],
				],
			],
			expected_result.clone(),
		);

		run_transformation_test(
			wl_output::Transform::Flipped90,
			vec![
				vec![
					[0, 0, 0, 255],
					[5, 5, 5, 255],
					[10, 10, 10, 255],
					[15, 15, 15, 255],
					[20, 20, 20, 255],
				],
				vec![
					[1, 1, 1, 255],
					[6, 6, 6, 255],
					[11, 11, 11, 255],
					[16, 16, 16, 255],
					[21, 21, 21, 255],
				],
				vec![
					[2, 2, 2, 255],
					[7, 7, 7, 255],
					[12, 12, 12, 255],
					[17, 17, 17, 255],
					[22, 22, 22, 255],
				],
				vec![
					[3, 3, 3, 255],
					[8, 8, 8, 255],
					[13, 13, 13, 255],
					[18, 18, 18, 255],
					[23, 23, 23, 255],
				],
				vec![
					[4, 4, 4, 255],
					[9, 9, 9, 255],
					[14, 14, 14, 255],
					[19, 19, 19, 255],
					[24, 24, 24, 255],
				],
			],
			expected_result.clone(),
		);
	}

	#[test]
	fn test_transformation_even_square()
	{
	}

	// instantiates struct and uses given arguments to test
	fn run_transformation_test(
		transform: wl_output::Transform,
		input: Vec<Vec<[u8; 4]>>,
		expected_result: Vec<Vec<[u8; 4]>>,
	)
	{
		let width = input.len();
		let height = input[0].len();

		let input_mmap = pixel_array_to_mmap(input);

		println!("\nTransform to apply: {:?}", transform);
		println!("\nThe input data:");

		print_mmap(width, height, &input_mmap);

		let output_capture = OutputCapture {
			transform,
			image_mmap: input_mmap,
			image_mmap_size: crate::rectangle::Size::new(width as i32, height as i32),
			// logical position and size should not affect this test
			image_logical_position: crate::rectangle::Position::new(0, 0),
			image_logical_size: crate::rectangle::Size::new(0, 0),
			image_pixel_format: crate::wayland::PixelFormat::Xbgr8888,
		};

		// apply the transformation
		let (width, height, result_mmap) = output_capture.get_transform_corrected_buffer();

		// the width and height should now match the expected result's
		assert_eq!(width, expected_result.len());
		assert_eq!(height, expected_result[0].len());

		// turn the pixel array into an mmap for comparison
		let expected_result_mmap = pixel_array_to_mmap(expected_result);

		// print the result
		println!("\nThe result:");
		print_mmap(width, height, &result_mmap);

		// print the expected result
		println!("\nThe expected result:");
		print_mmap(width, height, &expected_result_mmap);

		// the mmap lengths should match
		assert_eq!(result_mmap.len(), expected_result_mmap.len());

		// check each value in the mmap
		for i in 0..result_mmap.len()
		{
			assert_eq!(result_mmap[i], expected_result_mmap[i]);
		}
	}

	// helper function to flatten 3d array (Vec) into mmap
	fn pixel_array_to_mmap(pixel_array: Vec<Vec<[u8; 4]>>) -> memmap2::MmapMut
	{
		let mut mmap = memmap2::MmapMut::map_anon(5 * 5 * 4).unwrap();

		for (i, row) in pixel_array.into_iter().enumerate()
		{
			for (j, pixel) in row.into_iter().enumerate()
			{
				for (k, channel) in pixel.into_iter().enumerate()
				{
					mmap[(i * 5 * 4) + (j * 4) + k] = channel;
				}
			}
		}

		mmap
	}

	// helper funtion to print the contents of an mmap nicely
	fn print_mmap(width: usize, height: usize, mmap: &memmap2::MmapMut)
	{
		for i in 0..width
		{
			print!("[");
			for j in 0..height
			{
				print!("[");
				for k in 0..4
				{
					if k < 3
					{
						print!("{:03}, ", mmap[(i * 5 * 4) + (j * 4) + k]);
					}
					else
					{
						print!("{:03}", mmap[(i * 5 * 4) + (j * 4) + k]);
					}
				}

				if j < height - 1
				{
					print!("], ");
				}
				else
				{
					print!("]");
				}
			}
			println!("],");
		}
	}
}