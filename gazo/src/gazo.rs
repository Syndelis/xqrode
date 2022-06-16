use std::{cmp, ffi, fs, os::unix::io::FromRawFd};

use nix::sys::memfd;
use wayland_client::{
	self,
	protocol::{wl_buffer, wl_callback, wl_output, wl_registry, wl_shm, wl_shm_pool},
	Connection, Dispatch, QueueHandle,
};
use wayland_protocols::xdg::xdg_output::zv1::client::{zxdg_output_manager_v1, zxdg_output_v1};
use wayland_protocols_wlr::screencopy::v1::client::{
	zwlr_screencopy_frame_v1, zwlr_screencopy_manager_v1,
};

use crate::rectangle;

// TODO: look into delegate dispatch to avoid storing OutputInfos as a Vec in
// State and having to pass an index to access them when needed.

// all coordinates in this crate are absolute in the compositor coordinate space
// unless otherwise specified as local

#[derive(Debug)]
struct OutputInfo
{
	wl_output: wl_output::WlOutput,
	name: Option<String>,
	logical_position: Option<rectangle::Position>,
	logical_size: Option<rectangle::Size>,
	transform: Option<wl_output::Transform>,
	image_file: Option<fs::File>, // file is backed by RAM
	image_mmap: Option<memmap2::Mmap>,
	image_position: Option<rectangle::Position>,
	image_size: Option<rectangle::Size>,
	image_ready: bool,
}

impl OutputInfo
{
	fn get_pixel(&self, position: rectangle::Position) -> [u8; 4]
	{
		// TODO: implement other transforms
		let index = match self.transform.as_ref().unwrap()
		{
			wl_output::Transform::Normal =>
			{
				((position.x - self.image_position.unwrap().x)
					+ ((position.y - self.image_position.unwrap().y)
						* self.image_size.unwrap().width))
					* 4
			}
			wl_output::Transform::_270 =>
			{
				(((position.x - self.image_position.unwrap().x) * self.image_size.unwrap().height)
					+ (self.image_size.unwrap().height
						- (position.y - self.image_position.unwrap().y)
						- 1)) * 4
			}
			_ => panic!("AHHHH"),
		};

		[
			self.image_mmap.as_ref().unwrap()[index as usize], // R
			self.image_mmap.as_ref().unwrap()[(index + 1) as usize], // G
			self.image_mmap.as_ref().unwrap()[(index + 2) as usize], // B
			self.image_mmap.as_ref().unwrap()[(index + 3) as usize], // A
		]
	}
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
				"wl_shm" =>
				{
					let wl_shm = registry
						.bind::<wl_shm::WlShm, _, _>(name, 1, queue_handle, ())
						.unwrap();

					self.wl_shm = Some(wl_shm);
				}
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
						image_file: None,
						image_mmap: None,
						image_position: None,
						image_size: None,
						image_ready: false,
					});
				}
				_ =>
				{}
			}
		}
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
		match event
		{
			wl_callback::Event::Done { callback_data: _ } => self.done = true,
			_ => println!("Unimplemented callback received, please open an issue."),
		}
	}
}

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
			_ =>
			{}
		}
	}
}

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
			zxdg_output_v1::Event::LogicalPosition { x, y } =>
			{
				self.output_infos[*index].logical_position = Some(rectangle::Position { x, y });
			}
			zxdg_output_v1::Event::LogicalSize { width, height } =>
			{
				self.output_infos[*index].logical_size = Some(rectangle::Size { width, height });
			}
			_ =>
			{}
		}
	}
}

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
			// TODO: handle other buffer events
			zwlr_screencopy_frame_v1::Event::Buffer {
				format,
				width,
				height,
				stride,
			} =>
			{
				let raw_fd = memfd::memfd_create(
					ffi::CStr::from_bytes_with_nul(b"gato\0").unwrap(),
					memfd::MemFdCreateFlag::MFD_CLOEXEC | memfd::MemFdCreateFlag::MFD_ALLOW_SEALING,
				)
				.unwrap();

				self.output_infos[*index].image_file =
					Some(unsafe { fs::File::from_raw_fd(raw_fd) });

				self.output_infos[*index]
					.image_file
					.as_mut()
					.unwrap()
					.set_len((width * height * 4) as u64)
					.expect("Failed to allocate memory for screencopy.");

				let wl_shm_pool = self
					.wl_shm
					.as_ref()
					.unwrap()
					.create_pool(raw_fd, (width * height * 4) as i32, queue_handle, ())
					.expect("Failed to create pool from wl_shm.");

				let wl_buffer = wl_shm_pool
					.create_buffer(
						0,
						width as i32,
						height as i32,
						stride as i32,
						format.into_result().unwrap(),
						queue_handle,
						(),
					)
					.expect("Failed to create buffer from wl_shm_pool.");

				wlr_screencopy_frame.copy(&wl_buffer);
			}
			zwlr_screencopy_frame_v1::Event::Ready {
				tv_sec_hi: _,
				tv_sec_lo: _,
				tv_nsec: _,
			} =>
			{
				self.output_infos[*index].image_mmap = Some(unsafe {
					memmap2::Mmap::map(self.output_infos[*index].image_file.as_ref().unwrap())
						.expect("Failed to create memory mapping")
				});

				self.output_infos[*index].image_ready = true;
			}
			_ =>
			{}
		}
	}
}

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

fn connect_and_get_output_info() -> Result<(State, wayland_client::EventQueue<State>), crate::Error>
{
	let connection = Connection::connect_to_env()?;

	let mut event_queue = connection.new_event_queue();

	let wl_display = connection.display();

	wl_display.get_registry(&event_queue.handle(), ()).unwrap();

	// TODO: try without this line, may be unecessary
	wl_display.sync(&event_queue.handle(), ()).unwrap();

	let mut state = State {
		done: false,
		wlr_screencopy_manager: None,
		xdg_output_manager: None,
		wl_shm: None,
		output_infos: Vec::new(),
	};

	// run until initial done event and done event has been sent for all wl_outputs
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

type CaptureReturn = Result<(u32, u32, Vec<u8>), crate::Error>;

pub fn capture_all_outputs(include_cursor: bool) -> CaptureReturn
{
	let (mut state, mut event_queue) = connect_and_get_output_info()?;

	for (i, output_info) in state.output_infos.iter_mut().enumerate()
	{
		output_info.image_position = output_info.logical_position;
		output_info.image_size = output_info.logical_size;

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

pub fn capture_output(output: &str, include_cursor: bool) -> CaptureReturn
{
	let (mut state, mut event_queue) = connect_and_get_output_info()?;

	state.output_infos = state
		.output_infos
		.into_iter()
		.filter(|output_info| {
			if output_info.name.as_ref().unwrap() == output
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
		return Err(crate::Error::NoOutput(output.to_owned()));
	}

	state.output_infos[0].image_position = Some(rectangle::Position { x: 0, y: 0 });
	state.output_infos[0].image_size = state.output_infos[0].logical_size;

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

pub fn capture_region(
	region_position: (i32, i32),
	region_size: (i32, i32),
	include_cursor: bool,
) -> CaptureReturn
{
	let region_rectangle = rectangle::Rectangle {
		position: rectangle::Position::new(region_position),
		size: rectangle::Size::new(region_size),
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
				output_info.image_position = Some(rectangle.position);
				output_info.image_size = Some(rectangle.size);

				true
			}
			None => false,
		}
	});

	// request capture of screen
	for (i, output_info) in state.output_infos.iter_mut().enumerate()
	{
		let image_position = output_info.image_position.unwrap();
		let image_size = output_info.image_size.unwrap();

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

fn captures_to_buffer(captures: Vec<OutputInfo>) -> CaptureReturn
{
	if captures.is_empty()
	{
		return Err(crate::Error::NoCaptures);
	}

	let mut upper_left = captures[0].image_position.unwrap();
	let mut bottom_right = upper_left + captures[0].image_size.unwrap();

	for capture in &captures[1..]
	{
		upper_left.x = cmp::min(capture.image_position.unwrap().x, upper_left.x);

		upper_left.y = cmp::min(capture.image_position.unwrap().y, upper_left.y);

		bottom_right.x = cmp::max(
			capture.image_position.unwrap().x + capture.image_size.unwrap().width,
			bottom_right.x,
		);

		bottom_right.y = cmp::max(
			capture.image_position.unwrap().y + capture.image_size.unwrap().height,
			bottom_right.y,
		);
	}

	let size = rectangle::Size {
		width: bottom_right.x - upper_left.x,
		height: bottom_right.y - upper_left.y,
	};

	let mut buffer: Vec<u8> = vec![0; size.width as usize * size.height as usize * 4];

	for capture in &captures
	{
		let position_offset = capture.image_position.unwrap() - upper_left;

		for y in 0..capture.image_size.unwrap().height
		{
			for x in 0..capture.image_size.unwrap().width
			{
				// TODO add offset to x and y from for loops and use to calculate buff index
				// convert to absolute coordinates
				let position = rectangle::Position {
					x: x + capture.image_position.unwrap().x,
					y: y + capture.image_position.unwrap().y,
				};

				// println!("capture position: {:?}", capture.image_position.unwrap());

				let pixel = capture.get_pixel(position);

				let index =
					((position_offset.y + y) * (size.width * 4)) + ((position_offset.x + x) * 4);

				buffer[index as usize] = pixel[0];
				buffer[(index as usize) + 1] = pixel[1];
				buffer[(index as usize) + 2] = pixel[2];
				buffer[(index as usize) + 3] = pixel[3];
			}
		}
	}

	Ok((size.width as u32, size.height as u32, buffer))
}
