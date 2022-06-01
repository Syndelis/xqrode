use std::{
	ffi, fs,
	io::{self, Read, Write},
	os::unix::io::FromRawFd,
};

use nix::sys::memfd;
use regex::Regex;
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

struct QrodeOutputInfo
{
	wl_output: wl_output::WlOutput,
	logical_position: Option<(i32, i32)>,
	logical_size: Option<(i32, i32)>,
	transform: Option<wl_output::Transform>,
	image_file: Option<fs::File>, // file is backed by RAM
	image_size: Option<(i32, i32)>,
}

struct State
{
	done: bool,
	wlr_screencopy_manager: Option<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1>,
	xdg_output_manager: Option<zxdg_output_manager_v1::ZxdgOutputManagerV1>,
	wl_shm: Option<wl_shm::WlShm>,
	qrode_output_infos: Vec<QrodeOutputInfo>,
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

					println!("Got screencopy manager");
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

					println!("Got output manager");
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
							self.qrode_output_infos.len(),
						)
						.unwrap();

					self.qrode_output_infos.push(QrodeOutputInfo {
						wl_output,
						logical_position: None,
						logical_size: None,
						transform: None,
						image_file: None,
						image_size: None,
					});

					println!("Got wl_output");
				}
				_ =>
				{
					// println!("{}", interface);
				}
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
			_ => println!("Unimplemented callback received."),
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
			println!("Transform: {:?}", transform);
			self.qrode_output_infos[*index].transform = transform.into_result().ok();
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
		println!("Received properties: {:?}", event);

		match event
		{
			zxdg_output_v1::Event::LogicalPosition { x, y } =>
			{
				self.qrode_output_infos[*index].logical_position = Some((x, y));
			}
			zxdg_output_v1::Event::LogicalSize { width, height } =>
			{
				self.qrode_output_infos[*index].logical_size = Some((width, height));
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
				println!("stride: {}", stride);
				let raw_fd = memfd::memfd_create(
					ffi::CStr::from_bytes_with_nul(b"qrode\0").unwrap(),
					memfd::MemFdCreateFlag::MFD_CLOEXEC | memfd::MemFdCreateFlag::MFD_ALLOW_SEALING,
				)
				.unwrap();

				// qrode_screencopy_frame.raw_fd = Some(raw_fd);
				self.qrode_output_infos[*index].image_file =
					Some(unsafe { fs::File::from_raw_fd(raw_fd) });
				self.qrode_output_infos[*index].image_size = Some((width as i32, height as i32));

				println!("258: width: {}, height: {}", width, height);
				// TODO: handle other pixel formats
				self.qrode_output_infos[*index]
					.image_file
					.as_mut()
					.unwrap()
					.set_len((width * height * 4) as u64)
					.expect("Failed to allocate memory for screencopy");

				let wl_shm_pool = self
					.wl_shm
					.as_ref()
					.unwrap()
					.create_pool(raw_fd, (width * height * 4) as i32, queue_handle, ())
					.unwrap();

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
					.unwrap();

				wlr_screencopy_frame.copy(&wl_buffer);
			}
			zwlr_screencopy_frame_v1::Event::Ready {
				tv_sec_hi: _,
				tv_sec_lo: _,
				tv_nsec: _,
			} =>
			{
				let qrode_output_info = &mut self.qrode_output_infos[*index];
				// let mut image_file =
				// unsafe{fs::File::from_raw_fd(qrode_screencopy_frame.raw_fd.unwrap())};
				let image_file = qrode_output_info.image_file.as_mut().unwrap();
				// image_file.rewind();

				let mut tmp_file =
					fs::File::create(format!("/tmp/qrode/output{}.ppm", *index)).unwrap();

				// io::copy(image_file, &mut tmp_file);
				let image_size = qrode_output_info.image_size.as_ref().unwrap();

				// TODO: handle alternate buffer formats
				let mut buffer = vec![0_u8; (image_size.0 * image_size.1 * 4) as usize];

				image_file.read_exact(&mut buffer).unwrap();

				writeln!(tmp_file, "P3\n{} {}\n255", image_size.0, image_size.1).unwrap();

				for i in (0..(image_size.0 * image_size.1 * 4)).step_by(4)
				{
					// xbgr8888le to rgb888be
					writeln!(
						tmp_file,
						"{} {} {}",
						buffer[i as usize],
						buffer[i as usize + 1],
						buffer[i as usize + 2]
					)
					.unwrap();

					// if i > 6038300 || i < 100
					// {
					//     //println!("rgbx BE: {} {} {} {}", buffer[i as usize], buffer[i as usize
					// + 1], buffer[i as usize + 2], buffer[i as usize + 3]); }
				}

				tmp_file.flush().expect("Failed to flush.");

				// println!("buffer: {:?}", buffer);

				// self.output_done_events += 1;
				println!("Ready");
			}
			zwlr_screencopy_frame_v1::Event::BufferDone =>
			{
				// let raw_fd =
				//     memfd::memfd_create
				//         (
				//             ffi::CStr::from_bytes_with_nul(b"qrode\0").unwrap(),
				//             memfd::MemFdCreateFlag::MFD_CLOEXEC |
				// memfd::MemFdCreateFlag::MFD_ALLOW_SEALING         ).unwrap();

				// //qrode_screencopy_frame.raw_fd = Some(raw_fd);
				// self.qrode_output_infos[*index].image_file =
				// Some(unsafe{fs::File::from_raw_fd(raw_fd)}); self.qrode_output_infos[*index].
				// image_size = Some((width as i32, height as i32));

				// // TODO: handle other pixel formats
				// self.qrode_output_infos[*index].image_file.as_mut().unwrap().set_len((width *
				// height * 4) as u64);

				// let wl_shm_pool = self.wl_shm.as_ref().unwrap().create_pool(raw_fd, (width *
				// height * 4) as i32, queue_handle, ()).unwrap();

				// let wl_buffer = wl_shm_pool.create_buffer(0, width as i32, height as i32, stride
				// as i32, wl_shm::Format::Argb8888, queue_handle, ()).unwrap();

				// wlr_screencopy_frame.copy(&wl_buffer);
			}
			_ =>
			{
				println!("Event: {:?}", event);
			}
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
		// println!("WlShm event: {:?}", event);
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

fn main()
{
	let mut buffer = String::new();
	let stdin = io::stdin();

	stdin
		.read_line(&mut buffer)
		.expect("Failed to read from stdin.");

	let re = Regex::new(r"(-?\d+),(-?\d+) (\d+)x(\d+)").unwrap();

	let captures = re.captures(&buffer).expect("Failed to parse input.");

	let position = (
		captures.get(1).unwrap().as_str().parse::<i32>().unwrap(),
		captures.get(2).unwrap().as_str().parse::<i32>().unwrap(),
	);
	let size = (
		captures.get(3).unwrap().as_str().parse::<i32>().unwrap(),
		captures.get(4).unwrap().as_str().parse::<i32>().unwrap(),
	);

	println!(
		"Position: ({}, {}), Size: ({}, {})",
		position.0, position.1, size.0, size.1
	);

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
		qrode_output_infos: Vec::new(),
	};

	// run until initial done event and done event has been sent for all wl_outputs
	while !state.done
	{
		event_queue.blocking_dispatch(&mut state).unwrap();
	}

	// get xdg output for logical position and size
	for (i, qrode_output_info) in state.qrode_output_infos.iter_mut().enumerate()
	{
		state
			.xdg_output_manager
			.as_ref()
			.unwrap()
			.get_xdg_output(&qrode_output_info.wl_output, &event_queue.handle(), i)
			.ok();
		println!("GET XDG OUTPUT");
	}

	// run until all information about the outputs has been sent
	while state.qrode_output_infos.iter().any(|qrode_output_info| {
		qrode_output_info.logical_position.is_none()
			|| qrode_output_info.logical_size.is_none()
			|| qrode_output_info.transform.is_none()
	})
	{
		event_queue.blocking_dispatch(&mut state).unwrap();
	}

	// request capture of screen
	for (i, qrode_output_info) in state.qrode_output_infos.iter().enumerate()
	{
		let Rectangle { position, size } = match box_output_intersection_local_coordinates(
			SelectionBox(Rectangle { position, size }),
			OutputBox(Rectangle {
				position: qrode_output_info.logical_position.unwrap(),
				size: qrode_output_info.logical_size.unwrap(),
			}),
		)
		{
			Some(rectangle) => rectangle,
			None => continue,
		};
		// let relative_position = (cmp::max(position.0 -
		// qrode_output_info.logical_position.as_ref().unwrap().0, 0), cmp::max(position.1 -
		// qrode_output_info.logical_position.as_ref().unwrap().1, 0)); let relative_size =
		// (cmp::min(qrode_output_info.logical_size.as_ref().unwrap().0 - relative_position.0,
		// size.0 + position.0 - qrode_output_info.logical_position.as_ref().unwrap().0),
		// cmp::min(qrode_output_info.logical_size.as_ref().unwrap().1 - relative_position.1, size.1
		// + position.1 - qrode_output_info.logical_position.as_ref().unwrap().1));

		println!("position: {:?}, size: {:?}", position, size);
		// TODO: account for logical position and size
		match qrode_output_info.transform.as_ref().unwrap()
		{
			wl_output::Transform::Normal =>
			{
				state
					.wlr_screencopy_manager
					.as_ref()
					.unwrap()
					.capture_output_region(
						0,
						&qrode_output_info.wl_output,
						position.0,
						position.1,
						size.0,
						size.1,
						&event_queue.handle(),
						i,
					)
					.unwrap();
			}
			wl_output::Transform::_90 =>
			{}
			wl_output::Transform::_180 =>
			{}
			wl_output::Transform::_270 =>
			{
				// transforms position so it starts at top left
				let position = (
					qrode_output_info.logical_size.as_ref().unwrap().0 - size.0 - position.0,
					qrode_output_info.logical_size.as_ref().unwrap().1 - size.1 - position.1,
				);
				// let position = (qrode_output_info.logical_size.as_ref().unwrap().0 - position.0 -
				// size.0, qrode_output_info.logical_size.as_ref().unwrap().1 - position.1);
				// let size = (-size.0, -size.1);

				state
					.wlr_screencopy_manager
					.as_ref()
					.unwrap()
					.capture_output_region(
						0,
						&qrode_output_info.wl_output,
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
				println!("Not implemented, please report!");
			}
		}
	}

	loop
	{
		event_queue.blocking_dispatch(&mut state);
	}
}

struct Rectangle
{
	position: (i32, i32),
	size: (i32, i32),
}

struct SelectionBox(Rectangle);
struct OutputBox(Rectangle);

fn box_output_intersection_local_coordinates(
	selection_box: SelectionBox,
	output_box: OutputBox,
) -> Option<Rectangle>
{
	let SelectionBox(selection_box_rectangle) = selection_box;
	let OutputBox(output_box_rectangle) = output_box;

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
