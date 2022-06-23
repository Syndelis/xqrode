//! Gazo is a crate to capture screen pixel data on Wayland compositors
//! that implement the wlr_screencopy protocol, like <a href = "https://github.com/swaywm/sway" target = "_blank">sway</a>.

#![deny(missing_docs)]

use std::cmp;

pub use rgb::ComponentBytes;
use rgb::FromSlice;
use wayland_client::protocol::wl_output;

mod backend;
#[cfg(feature = "clap-region-parsing")]
mod clap;
mod rectangle;
mod transform;

#[cfg(feature = "clap-region-parsing")]
pub use crate::clap::Region;

/// Enum representing potential errors.
#[derive(thiserror::Error, Debug)]
pub enum Error
{
	/// This error will be returned by [`capture_output`] when the given
	/// output name does not match any outputs listed by the compositor.
	#[error("output \"{0}\" was not found")]
	NoOutput(String),
	/// This error may be returned by any screen capturing function. Should
	/// realistically only occur when using [`capture_region`] with a region
	/// outside of the compositor space.
	#[error("no screen captures when trying to composite the complete capture")]
	NoCaptures,
	/// Wrapper for a Wayland connection error. Should only happen in
	/// environments without a Wayland compositor running.
	#[error("failed to connect to the wayland server")]
	Connect(#[from] wayland_client::ConnectError),
	/// Wrapper for a Wayland dispatch error. Should not happen unless there is
	/// an error in the library or the compositor.
	#[error("failed to dispatch event from wayland server")]
	Dispatch(#[from] wayland_client::DispatchError),
	/// Error thrown in the event of an unimplemented handler. This can occur
	/// when the Wayland protocols used add another variant that is not handled
	/// in this library.
	#[error("{0}")]
	Unimplemented(String),
}

/// This is the return type for the Ok variant of the capture functions. It
/// contains the dimensions (width and height in pixels) of the capture and a
/// `Vec` with the captured pixel data as [`rgb::RGBA8`] (see [`rgb::RGBA`]).
/// This can be cast to a slice of `u8`s using the [`ComponentBytes`] trait.
pub struct Capture
{
	/// The width of the capture in pixels.
	pub width: usize,
	/// The height of the capture in pixels.
	pub height: usize,
	/// The `Vec` containing the pixel data.
	pub pixel_data: Vec<rgb::RGBA8>,
}

// shared return type for capture functions
type CaptureReturn = Result<Capture, crate::Error>;

/// This function will capture the entirety of all outputs and composite them
/// into the capture, accounting for transformations, offsets, and scaling.
pub fn capture_all_outputs(include_cursor: bool) -> CaptureReturn
{
	let (mut state, mut event_queue) = backend::connect_and_get_output_info()?;

	for (i, output_info) in state.output_infos.iter_mut().enumerate()
	{
		// the image logical position and size will be the same as the output logical
		// position and size
		output_info.image_logical_position = output_info.output_logical_position;
		output_info.image_logical_size = output_info.output_logical_size;

		// this only returns an error when the object ID is invalid, which it should not
		// be at this point
		state
			.wlr_screencopy_manager
			.as_ref()
			.unwrap()
			.capture_output(
				include_cursor as i32,
				&output_info.wl_output,
				&event_queue.handle(),
				i,
			)
			.unwrap();
	}

	// wait for images to be ready
	while state
		.output_infos
		.iter()
		.any(|output_info| !output_info.image_ready)
	{
		event_queue.blocking_dispatch(&mut state)?;
	}

	captures_to_buffer(state.output_infos)
}

/// This function will capture the output specified in the `name` field of the
/// arguments, returning an error if the name does not match an output.
pub fn capture_output(name: &str, include_cursor: bool) -> CaptureReturn
{
	let (mut state, mut event_queue) = backend::connect_and_get_output_info()?;

	state.output_infos = state
		.output_infos
		.into_iter()
		.filter(|output_info| {
			if output_info.name.as_ref().unwrap() == name
			{
				true
			}
			else
			{
				output_info.wl_output.release();
				false
			}
		})
		.collect();

	if state.output_infos.is_empty()
	{
		return Err(crate::Error::NoOutput(name.to_owned()));
	}

	state.output_infos[0].image_logical_position = Some(rectangle::Position { x: 0, y: 0 });
	state.output_infos[0].image_logical_size = state.output_infos[0].output_logical_size;

	state
		.wlr_screencopy_manager
		.as_ref()
		.unwrap()
		.capture_output(
			include_cursor as i32,
			&state.output_infos[0].wl_output,
			&event_queue.handle(),
			0,
		)
		.unwrap();

	while !state.output_infos[0].image_ready
	{
		event_queue.blocking_dispatch(&mut state)?;
	}

	captures_to_buffer(state.output_infos)
}

/// This function will capture the region of the compositor specified by the
/// `region_position` and `region_size` arguments. The `region_position` should
/// be the top left corner of the region with the `region_size` expanding from
/// there; these values should be based on the compositor logical output
/// positions and sizes. This will be the same as the default output provided by <a href = "https://github.com/emersion/slurp" target = "_blank">slurp</a>.
pub fn capture_region(
	region_position: (i32, i32),
	region_size: (i32, i32),
	include_cursor: bool,
) -> CaptureReturn
{
	let region_rectangle = rectangle::Rectangle {
		position: rectangle::Position::new(region_position.0, region_position.1),
		size: rectangle::Size::new(region_size.0, region_size.1),
	};

	let (mut state, mut event_queue) = backend::connect_and_get_output_info()?;

	state.output_infos.retain_mut(|output_info| {
		// determine the region of the output that is selected
		match rectangle::Rectangle::new(
			output_info.output_logical_position.unwrap(),
			output_info.output_logical_size.unwrap(),
		)
		.get_intersection(region_rectangle)
		{
			Some(rectangle) =>
			{
				output_info.image_logical_position = Some(rectangle.position);
				output_info.image_logical_size = Some(rectangle.size);

				true
			}
			None => false,
		}
	});

	// request capture of screen
	for (i, output_info) in state.output_infos.iter_mut().enumerate()
	{
		let image_position = output_info.image_logical_position.unwrap();
		let image_size = output_info.image_logical_size.unwrap();

		// adjust position to local output coordinates
		let image_position_local = {
			// this is what the image_position_local should be
			let image_position_local_normal =
				image_position - output_info.output_logical_position.unwrap();

			// 2 of the transforms seem to have their logical coordinates start in the
			// bottom right instead of the top left, so this adjusts for that. TODO:
			// determine if this is the expected behavior as it does not seem to be
			// specified in the Wayland protocol docs
			match output_info.transform.as_ref().unwrap()
			{
				wl_output::Transform::Normal
				| wl_output::Transform::_180
				| wl_output::Transform::Flipped
				| wl_output::Transform::Flipped270
				| wl_output::Transform::Flipped90
				| wl_output::Transform::Flipped180 => image_position_local_normal,
				wl_output::Transform::_270 | wl_output::Transform::_90 =>
				{
					// transforms position so it starts at the logical top left
					rectangle::Position::new(
						-image_position_local_normal.x
							+ output_info.output_logical_size.unwrap().width
							- image_size.width,
						-image_position_local_normal.y
							+ output_info.output_logical_size.unwrap().height
							- image_size.height,
					)
				}
				_ =>
				{
					return Err(Error::Unimplemented(format!(
						"output transform not implemented: {:?}",
						output_info.transform.as_ref().unwrap()
					)));
				}
			}
		};

		// should not fail
		state
			.wlr_screencopy_manager
			.as_ref()
			.unwrap()
			.capture_output_region(
				include_cursor as i32,
				&output_info.wl_output,
				image_position_local.x,
				image_position_local.y,
				image_size.width,
				image_size.height,
				&event_queue.handle(),
				i,
			)
			.unwrap();
	}

	// wait for images to be ready
	while state
		.output_infos
		.iter()
		.any(|output_info| !output_info.image_ready)
	{
		event_queue.blocking_dispatch(&mut state)?;
	}

	captures_to_buffer(state.output_infos)
}

fn captures_to_buffer(output_infos: Vec<backend::OutputInfo>) -> CaptureReturn
{
	if output_infos.is_empty()
	{
		return Err(crate::Error::NoCaptures);
	}

	let output_captures: Vec<backend::OutputCapture> = output_infos
		.into_iter()
		.map(|output_info| {
			let (mmap_width, mmap_height, image_mmap) =
				transform::create_transform_corrected_buffer(
					output_info.transform.unwrap(),
					output_info.image_mmap.unwrap(),
					output_info.image_mmap_size.unwrap(),
					output_info.image_pixel_format.unwrap(),
				);

			backend::OutputCapture {
				image_logical_position: output_info.image_logical_position.unwrap(),
				image_logical_size: output_info.image_logical_size.unwrap(),
				image_mmap,
				image_mmap_size: rectangle::Size::new(mmap_width as i32, mmap_height as i32),
			}
		})
		.collect();

	// init dimension values
	let mut upper_left = output_captures[0].image_logical_position;
	let mut bottom_right = rectangle::Position::new(
		upper_left.x + output_captures[0].image_logical_size.width,
		upper_left.y + output_captures[0].image_logical_size.height,
	);

	// loop over the other captures to determine the full dimensions
	for capture in &output_captures[1..]
	{
		upper_left.x = cmp::min(capture.image_logical_position.x, upper_left.x);

		upper_left.y = cmp::min(capture.image_logical_position.y, upper_left.y);

		bottom_right.x = cmp::max(
			capture.image_logical_position.x + capture.image_logical_size.width,
			bottom_right.x,
		);

		bottom_right.y = cmp::max(
			capture.image_logical_position.y + capture.image_logical_size.height,
			bottom_right.y,
		);
	}

	let size = rectangle::Size {
		width: bottom_right.x - upper_left.x,
		height: bottom_right.y - upper_left.y,
	};

	let mut buffer: Vec<rgb::RGBA8> = vec![
		rgb::RGBA8 {
			r: 0,
			g: 0,
			b: 0,
			a: 0
		};
		size.width as usize * size.height as usize
	];

	for output_capture in output_captures.into_iter()
	{
		let mut destination = vec![
			rgb::RGBA::<u8>::new(0, 0, 0, 0);
			output_capture.image_logical_size.width as usize
				* output_capture.image_logical_size.height as usize
		];

		// resize if needed, otherwise just use the current mmap
		let image_buffer = if output_capture.image_mmap_size.width
			!= output_capture.image_logical_size.width
			|| output_capture.image_mmap_size.height != output_capture.image_logical_size.height
		{
			let mut resizer = resize::Resizer::new(
				output_capture.image_mmap_size.width as usize,
				output_capture.image_mmap_size.height as usize,
				output_capture.image_logical_size.width as usize,
				output_capture.image_logical_size.height as usize,
				resize::Pixel::RGBA8,
				resize::Type::Lanczos3,
			)
			.unwrap();

			resizer
				.resize(output_capture.image_mmap.as_rgba(), &mut destination)
				.unwrap();

			destination.as_slice()
		}
		else
		{
			output_capture.image_mmap.as_rgba()
		};

		let position_offset = output_capture.image_logical_position - upper_left;

		for y in 0..output_capture.image_logical_size.height
		{
			for x in 0..output_capture.image_logical_size.width
			{
				let output_capture_index = (y * output_capture.image_logical_size.width) + x;

				let output_capture_index = output_capture_index as usize;

				let index = ((position_offset.y + y) * size.width) + (position_offset.x + x);

				let index = index as usize;

				buffer[index] = image_buffer[output_capture_index];
			}
		}
	}

	Ok(Capture {
		width: size.width as usize,
		height: size.height as usize,
		pixel_data: buffer,
	})
}
