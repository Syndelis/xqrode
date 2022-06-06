use std::{
	ffi, fs,
	io::{Read, Write},
	os::unix::io::FromRawFd,
};

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

// use image::
mod rectangle;

struct OutputInfo
{
	wl_output: wl_output::WlOutput,
	logical_position: Option<(i32, i32)>,
	logical_size: Option<(i32, i32)>,
	transform: Option<wl_output::Transform>,
	image_file: Option<fs::File>, // file is backed by RAM
	image_size: Option<(i32, i32)>,
	image_ready: bool,
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
						logical_position: None,
						logical_size: None,
						transform: None,
						image_file: None,
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
		// only here to satusfy a trait bound
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
		// only here to satusfy a trait bound
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
		if let wl_output::Event::Geometry { transform, .. } = event
		{
			self.output_infos[*index].transform = transform.into_result().ok();
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
				self.output_infos[*index].logical_position = Some((x, y));
			}
			zxdg_output_v1::Event::LogicalSize { width, height } =>
			{
				self.output_infos[*index].logical_size = Some((width, height));
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
				self.output_infos[*index].image_size = Some((width as i32, height as i32));

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
				let output_info = &mut self.output_infos[*index];

				let image_file = output_info.image_file.as_mut().unwrap();

				let mut tmp_file =
					fs::File::create(format!("/tmp/qrode/output{}.ppm", *index)).unwrap();

				let image_size = output_info.image_size.as_ref().unwrap();

				let mut buffer = vec![0_u8; (image_size.0 * image_size.1 * 4) as usize];

				image_file.read_exact(&mut buffer).unwrap();

				writeln!(tmp_file, "P3\n{} {}\n255", image_size.0, image_size.1).unwrap();

				for i in (0..(image_size.0 * image_size.1 * 4)).step_by(4)
				{
					writeln!(
						tmp_file,
						"{} {} {}",
						buffer[i as usize],
						buffer[i as usize + 1],
						buffer[i as usize + 2]
					)
					.unwrap();
				}

				tmp_file.flush().expect("Failed to flush.");

				output_info.image_ready = true;
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

pub fn capture_desktop(position: (i32, i32), size: (i32, i32))
{
	let connection =
		Connection::connect_to_env().expect("Failed to get connection to wayland server.");

	let mut event_queue = connection.new_event_queue();

	let wl_display = connection.display();

	wl_display
		.get_registry(&event_queue.handle(), ())
		.expect("Failed to create wayland registry object.");

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
		event_queue.blocking_dispatch(&mut state).unwrap();
	}

	// get xdg output for logical position and size
	for (i, output_info) in state.output_infos.iter_mut().enumerate()
	{
		state
			.xdg_output_manager
			.as_ref()
			.unwrap()
			.get_xdg_output(&output_info.wl_output, &event_queue.handle(), i)
			.ok();
	}

	// run until all information about the outputs has been sent
	while state.output_infos.iter().any(|output_info| {
		output_info.logical_position.is_none()
			|| output_info.logical_size.is_none()
			|| output_info.transform.is_none()
	})
	{
		event_queue.blocking_dispatch(&mut state).unwrap();
	}

	// request capture of screen
	for (i, output_info) in state.output_infos.iter().enumerate()
	{
		let rectangle::Rectangle { position, size } = match rectangle::Rectangle::new(
			output_info.logical_position.unwrap(),
			output_info.logical_size.unwrap(),
		)
		.get_intersection(rectangle::Rectangle { position, size })
		{
			Some(rectangle) => rectangle,
			None => continue,
		};

		let position = (
			position.0 - output_info.logical_position.unwrap().0,
			position.1 - output_info.logical_position.unwrap().1,
		);

		match output_info.transform.as_ref().unwrap()
		{
			wl_output::Transform::Normal =>
			{
				state
					.wlr_screencopy_manager
					.as_ref()
					.unwrap()
					.capture_output_region(
						0,
						&output_info.wl_output,
						position.0,
						position.1,
						size.0,
						size.1,
						&event_queue.handle(),
						i,
					)
					.unwrap();
			}
			// TODO: handle other transforms
			wl_output::Transform::_90 =>
			{}
			wl_output::Transform::_180 =>
			{}
			wl_output::Transform::_270 =>
			{
				// transforms position so it starts at top left
				let position = (
					output_info.logical_size.as_ref().unwrap().0 - size.0 - position.0,
					output_info.logical_size.as_ref().unwrap().1 - size.1 - position.1,
				);

				state
					.wlr_screencopy_manager
					.as_ref()
					.unwrap()
					.capture_output_region(
						0,
						&output_info.wl_output,
						position.0,
						position.1,
						size.0,
						size.1,
						&event_queue.handle(),
						i,
					)
					.unwrap();
			}
			_ =>
			{
				println!("Display transform not implemented, please open an issue.");
			}
		}
	}

	while state
		.output_infos
		.iter()
		.any(|output_info| !output_info.image_ready)
	{
		event_queue.blocking_dispatch(&mut state).unwrap();
	}
}
