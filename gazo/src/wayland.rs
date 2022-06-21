use std::{cmp, ffi, fs, os::unix::io::FromRawFd};

use nix::sys::memfd;
use rgb::FromSlice;
use wayland_client::{
	self,
	protocol::{wl_buffer, wl_callback, wl_output, wl_registry, wl_shm, wl_shm_pool},
	Connection, Dispatch, QueueHandle,
};
use wayland_protocols::xdg::xdg_output::zv1::client::{zxdg_output_manager_v1, zxdg_output_v1};
use wayland_protocols_wlr::screencopy::v1::client::{
	zwlr_screencopy_frame_v1, zwlr_screencopy_manager_v1,
};

use crate::{capture, rectangle};

// all coordinates in this crate are absolute in the compositor coordinate space
// unless otherwise specified as local

#[derive(Clone, Copy)]
pub(crate) enum PixelFormat
{
	Argb8888,
	Xrgb8888,
	Xbgr8888,
}

struct OutputInfo
{
	wl_output: wl_output::WlOutput,
	name: Option<String>,
	logical_position: Option<rectangle::Position>,
	logical_size: Option<rectangle::Size>,
	transform: Option<wl_output::Transform>,
	scale_factor: i32,
	image_file: Option<fs::File>,
	image_mmap: Option<memmap2::MmapMut>,
	image_mmap_size: Option<rectangle::Size>,
	image_logical_position: Option<rectangle::Position>,
	image_logical_size: Option<rectangle::Size>,
	image_pixel_format: Option<PixelFormat>,
	image_ready: bool,
}

#[derive(Debug)]
struct OutputCapture
{
	image_logical_position: rectangle::Position,
	image_logical_size: rectangle::Size,
	image_mmap: memmap2::MmapMut,
	image_mmap_size: rectangle::Size,
}

struct State
{
	done: bool,
	wlr_screencopy_manager: Option<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1>,
	xdg_output_manager: Option<zxdg_output_manager_v1::ZxdgOutputManagerV1>,
	wl_shm: Option<wl_shm::WlShm>,
	output_infos: Vec<OutputInfo>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State
{
	fn event(
		&mut self,
		registry: &wl_registry::WlRegistry,
		event: wl_registry::Event,
		_: &(),
		_connection: &Connection,
		queue_handle: &QueueHandle<Self>,
	)
	{
		if let wl_registry::Event::Global {
			name,
			interface,
			version: _,
		} = event
		{
			match &interface[..]
			{
				// get the screencopy manager (used to request capture of an output)
				"zwlr_screencopy_manager_v1" =>
				{
					let wlr_screencopy_manager = registry
						.bind::<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1, _, _>(
							name,
							3,
							queue_handle,
							(),
						)
						.unwrap();

					self.wlr_screencopy_manager = Some(wlr_screencopy_manager);
				}
				// get the xdg output manager (used to obtain information of outputs)
				"zxdg_output_manager_v1" =>
				{
					let xdg_output_manager = registry
						.bind::<zxdg_output_manager_v1::ZxdgOutputManagerV1, _, _>(
							name,
							3,
							queue_handle,
							(),
						)
						.unwrap();

					self.xdg_output_manager = Some(xdg_output_manager);
				}
				// get the shared memeory object (used to create shared memory pools)
				"wl_shm" =>
				{
					let wl_shm = registry
						.bind::<wl_shm::WlShm, _, _>(name, 1, queue_handle, ())
						.unwrap();

					self.wl_shm = Some(wl_shm);
				}
				// get the outputs for capture
				"wl_output" =>
				{
					let wl_output = registry
						.bind::<wl_output::WlOutput, _, _>(
							name,
							4,
							queue_handle,
							self.output_infos.len(),
						)
						.unwrap();

					self.output_infos.push(OutputInfo {
						wl_output,
						name: None,
						logical_position: None,
						logical_size: None,
						transform: None,
						scale_factor: 1,
						image_file: None,
						image_mmap: None,
						image_mmap_size: None,
						image_logical_position: None,
						image_logical_size: None,
						image_pixel_format: None,
						image_ready: false,
					});
				}
				_ =>
				{}
			}
		}
	}
}

// sent after all globals have been enumerated
impl Dispatch<wl_callback::WlCallback, ()> for State
{
	fn event(
		&mut self,
		_wl_callback: &wl_callback::WlCallback,
		event: wl_callback::Event,
		_: &(),
		_connection: &Connection,
		_queue_handle: &QueueHandle<Self>,
	)
	{
		if let wl_callback::Event::Done { callback_data: _ } = event
		{
			self.done = true;
		}
	}
}

// dispatch for output events like geometry and name information
impl Dispatch<wl_output::WlOutput, usize> for State
{
	fn event(
		&mut self,
		_wl_output: &wl_output::WlOutput,
		event: wl_output::Event,
		index: &usize,
		_connection: &Connection,
		_queue_handle: &QueueHandle<Self>,
	)
	{
		match event
		{
			wl_output::Event::Geometry { transform, .. } =>
			{
				self.output_infos[*index].transform = transform.into_result().ok();
			}
			wl_output::Event::Name { name } =>
			{
				self.output_infos[*index].name = Some(name);
			}
			wl_output::Event::Scale { factor } =>
			{
				self.output_infos[*index].scale_factor = factor;
				println!("Scale factor: {}", factor);
			}
			_ =>
			{}
		}
	}
}

// xdg protocol handler for additional output information like position and size
impl Dispatch<zxdg_output_v1::ZxdgOutputV1, usize> for State
{
	fn event(
		&mut self,
		_xdg_output: &zxdg_output_v1::ZxdgOutputV1,
		event: zxdg_output_v1::Event,
		index: &usize,
		_connection: &Connection,
		_queue_handle: &QueueHandle<Self>,
	)
	{
		match event
		{
			// logical position is the position in the compositor space accounting for transforms
			zxdg_output_v1::Event::LogicalPosition { x, y } =>
			{
				self.output_infos[*index].logical_position = Some(rectangle::Position { x, y });
			}
			// like logical position but for size
			zxdg_output_v1::Event::LogicalSize { width, height } =>
			{
				self.output_infos[*index].logical_size = Some(rectangle::Size { width, height });
			}
			_ =>
			{}
		}
	}
}

// handle screencopy events
impl Dispatch<zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1, usize> for State
{
	fn event(
		&mut self,
		wlr_screencopy_frame: &zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
		event: zwlr_screencopy_frame_v1::Event,
		index: &usize,
		_connection: &Connection,
		queue_handle: &QueueHandle<Self>,
	)
	{
		match event
		{
			// compositor is asking gazo to create a buffer for it to put the screencopy in
			// TODO: handle other buffer events
			zwlr_screencopy_frame_v1::Event::Buffer {
				format,
				width,
				height,
				stride,
			} =>
			{
				let format = format.into_result().unwrap();

				// check for valid format
				self.output_infos[*index].image_pixel_format = match format
				{
					wl_shm::Format::Argb8888 => Some(PixelFormat::Argb8888),
					wl_shm::Format::Xrgb8888 => Some(PixelFormat::Xrgb8888),
					wl_shm::Format::Xbgr8888 => Some(PixelFormat::Xbgr8888),
					_ => return,
				};

				self.output_infos[*index].image_mmap_size = Some(rectangle::Size {
					width: width as i32,
					height: height as i32,
				});

				println!("buffer size: {}x{}", width, height);

				// allocate memory with a file descriptor
				let raw_fd = memfd::memfd_create(
					ffi::CStr::from_bytes_with_nul(b"gato\0").unwrap(),
					memfd::MemFdCreateFlag::MFD_CLOEXEC | memfd::MemFdCreateFlag::MFD_ALLOW_SEALING,
				)
				.unwrap();

				self.output_infos[*index].image_file =
					Some(unsafe { fs::File::from_raw_fd(raw_fd) });

				// set file size
				self.output_infos[*index]
					.image_file
					.as_mut()
					.unwrap()
					.set_len((width * height * 4) as u64)
					.expect("Failed to allocate memory for screencopy.");

				// create pool from memory
				let wl_shm_pool = self
					.wl_shm
					.as_ref()
					.unwrap()
					.create_pool(raw_fd, (width * height * 4) as i32, queue_handle, ())
					.expect("Failed to create pool from wl_shm.");

				// create buffer from pool
				let wl_buffer = wl_shm_pool
					.create_buffer(
						0,
						width as i32,
						height as i32,
						stride as i32,
						format,
						queue_handle,
						(),
					)
					.expect("Failed to create buffer from wl_shm_pool.");

				// request copy of screen into buffer
				wlr_screencopy_frame.copy(&wl_buffer);
			}
			// buffer has been filled with screen data
			zwlr_screencopy_frame_v1::Event::Ready {
				tv_sec_hi: _,
				tv_sec_lo: _,
				tv_nsec: _,
			} =>
			{
				// create an mmap
				self.output_infos[*index].image_mmap = Some(unsafe {
					memmap2::MmapMut::map_mut(
						self.output_infos[*index].image_file.as_ref().unwrap(),
					)
					.expect("Failed to create memory mapping")
				});

				// mark image as ready
				self.output_infos[*index].image_ready = true;
			}
			_ =>
			{}
		}
	}
}

// unused dispatches //
impl Dispatch<wl_shm::WlShm, ()> for State
{
	fn event(
		&mut self,
		_wl_shm: &wl_shm::WlShm,
		_event: wl_shm::Event,
		_: &(),
		_connection: &Connection,
		_queue_handle: &QueueHandle<Self>,
	)
	{
	}
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for State
{
	fn event(
		&mut self,
		_wl_shm_pool: &wl_shm_pool::WlShmPool,
		_event: wl_shm_pool::Event,
		_: &(),
		_connection: &Connection,
		_queue_handle: &QueueHandle<Self>,
	)
	{
	}
}

impl Dispatch<wl_buffer::WlBuffer, ()> for State
{
	fn event(
		&mut self,
		_wl_buffer: &wl_buffer::WlBuffer,
		_event: wl_buffer::Event,
		_: &(),
		_connection: &Connection,
		_queue_handle: &QueueHandle<Self>,
	)
	{
	}
}

impl Dispatch<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1, ()> for State
{
	fn event(
		&mut self,
		_wlr_screencopy_manager: &zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
		_event: zwlr_screencopy_manager_v1::Event,
		_: &(),
		_connection: &Connection,
		_queue_handle: &QueueHandle<Self>,
	)
	{
	}
}

impl Dispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, ()> for State
{
	fn event(
		&mut self,
		_xdg_output_manager: &zxdg_output_manager_v1::ZxdgOutputManagerV1,
		_event: zxdg_output_manager_v1::Event,
		_: &(),
		_connection: &Connection,
		_queue_handle: &QueueHandle<Self>,
	)
	{
	}
}
// end unused dispatches //

fn connect_and_get_output_info() -> Result<(State, wayland_client::EventQueue<State>), crate::Error>
{
	let connection = Connection::connect_to_env()?;

	let mut event_queue = connection.new_event_queue();

	// wayland global object
	let wl_display = connection.display();

	// create a registry object to list and bind globals
	wl_display.get_registry(&event_queue.handle(), ()).unwrap();

	// ask the compositor to emit a 'done' event once globals have been enumerated
	wl_display.sync(&event_queue.handle(), ()).unwrap();

	let mut state = State {
		done: false,
		wlr_screencopy_manager: None,
		xdg_output_manager: None,
		wl_shm: None,
		output_infos: Vec::new(),
	};

	// run until done event has been sent for enumerating globals
	while !state.done
	{
		event_queue.blocking_dispatch(&mut state)?;
	}

	// get xdg output for logical position and size
	for (i, output_info) in state.output_infos.iter_mut().enumerate()
	{
		state
			.xdg_output_manager
			.as_ref()
			.unwrap()
			.get_xdg_output(&output_info.wl_output, &event_queue.handle(), i)
			.unwrap();
	}

	// run until all information about the outputs has been sent
	while state.output_infos.iter().any(|output_info| {
		output_info.logical_position.is_none()
			|| output_info.logical_size.is_none()
			|| output_info.transform.is_none()
	})
	{
		event_queue.blocking_dispatch(&mut state)?;
	}

	Ok((state, event_queue))
}

// shared return type for capture functions
type CaptureReturn = Result<(u32, u32, Vec<u8>), crate::Error>;

/// This function will capture the entirety of all outputs and composite them
/// into a single Vec<u8>
pub fn capture_all_outputs(include_cursor: bool) -> CaptureReturn
{
	let (mut state, mut event_queue) = connect_and_get_output_info()?;

	for (i, output_info) in state.output_infos.iter_mut().enumerate()
	{
		output_info.image_logical_position = output_info.logical_position;
		output_info.image_logical_size = output_info.logical_size;

		println!("logical size: {:?}", output_info.logical_size);

		// TODO: do not unwrap
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
/// arguments.
pub fn capture_output(name: &str, include_cursor: bool) -> CaptureReturn
{
	let (mut state, mut event_queue) = connect_and_get_output_info()?;

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
	state.output_infos[0].image_logical_size = state.output_infos[0].logical_size;

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
/// there. This will be the same as the default output provided by
/// <a href = "https://github.com/emersion/slurp" target = "_blank">slurp</a>.
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

	let (mut state, mut event_queue) = connect_and_get_output_info()?;

	state.output_infos.retain_mut(|output_info| {
		// determine the region of the output that is selected
		match rectangle::Rectangle::new(
			output_info.logical_position.unwrap(),
			output_info.logical_size.unwrap(),
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
		let mut image_position_local = image_position - output_info.logical_position.unwrap();

		match output_info.transform.as_ref().unwrap()
		{
			wl_output::Transform::Normal =>
			{
				// no adjustment needed
			}
			// TODO: handle other transforms (check docs for the flipped variants)
			wl_output::Transform::_90 =>
			{
				// TODO
			}
			wl_output::Transform::_180 =>
			{
				// TODO
			}
			wl_output::Transform::_270 =>
			{
				// transforms position so it starts at the logical top left
				// this transform causes (0, 0) to be at the bottom right of the monitor
				image_position_local = rectangle::Position {
					x: -image_position_local.x,
					y: -image_position_local.y,
				} + (output_info.logical_size.unwrap() - image_size);
			}
			_ =>
			{
				return Err(crate::Error::Unimplemented(format!(
					"Output transform not implemented: {:?}",
					output_info.transform.as_ref().unwrap()
				)));
			}
		}

		// TODO: do not unwrap
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

fn captures_to_buffer(output_infos: Vec<OutputInfo>) -> CaptureReturn
{
	if output_infos.is_empty()
	{
		return Err(crate::Error::NoCaptures);
	}

	let output_captures: Vec<OutputCapture> = output_infos
		.into_iter()
		.map(|output_info| {
			let (mmap_width, mmap_height, image_mmap) = capture::create_transform_corrected_buffer(
				output_info.transform.unwrap(),
				output_info.image_mmap.unwrap(),
				output_info.image_mmap_size.unwrap(),
				output_info.image_pixel_format.unwrap(),
			);

			OutputCapture {
				image_logical_position: output_info.logical_position.unwrap(),
				image_logical_size: output_info.image_logical_size.unwrap(),
				image_mmap,
				image_mmap_size: rectangle::Size::new(mmap_width as i32, mmap_height as i32),
			}
		})
		.collect();

	let mut upper_left = output_captures[0].image_logical_position;
	let mut bottom_right = upper_left + output_captures[0].image_logical_size;

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

	let mut buffer: Vec<u8> = vec![0; size.width as usize * size.height as usize * 4];

	let time = std::time::Instant::now();

	for output_capture in output_captures.into_iter()
	{
		println!("{:?}", output_capture);

		let mut destination = vec![
			rgb::RGBA::<u8>::new(0, 0, 0, 0);
			output_capture.image_logical_size.width as usize
				* output_capture.image_logical_size.height as usize
		];

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
			&output_capture.image_mmap.as_rgba()[..]
		};

		let position_offset = output_capture.image_logical_position - upper_left;

		for y in 0..output_capture.image_logical_size.height
		{
			for x in 0..output_capture.image_logical_size.width
			{
				// convert to absolute coordinates
				// let position = rectangle::Position {
				// 	x: x + output_capture.image_logical_position.x,
				// 	y: y + output_capture.image_logical_position.y,
				// };

				let output_capture_index = ((y * output_capture.image_logical_size.width) + x);

				let output_capture_index = output_capture_index as usize;

				let index =
					((position_offset.y + y) * (size.width * 4)) + ((position_offset.x + x) * 4);

				let index = index as usize;

				buffer[index] = image_buffer[output_capture_index].r;
				buffer[index + 1] = image_buffer[output_capture_index].g;
				buffer[index + 2] = image_buffer[output_capture_index].b;
				buffer[index + 3] = image_buffer[output_capture_index].a;
			}
		}
	}

	println!("{:?}", time.elapsed());

	Ok((size.width as u32, size.height as u32, buffer))
}
