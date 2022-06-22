use std::{ffi, fs, os::unix::io::FromRawFd};

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

// all coordinates in this crate are absolute in the compositor coordinate space
// unless otherwise specified as local

#[derive(Debug)]
pub(crate) struct OutputInfo
{
	pub wl_output: wl_output::WlOutput,
	pub name: Option<String>,
	pub output_logical_position: Option<rectangle::Position>,
	pub output_logical_size: Option<rectangle::Size>,
	pub transform: Option<wl_output::Transform>,
	pub scale_factor: i32,
	pub image_file: Option<fs::File>,
	pub image_mmap: Option<memmap2::MmapMut>,
	pub image_mmap_size: Option<rectangle::Size>,
	pub image_logical_position: Option<rectangle::Position>,
	pub image_logical_size: Option<rectangle::Size>,
	pub image_pixel_format: Option<wl_shm::Format>,
	pub image_ready: bool,
}

#[derive(Debug)]
pub(crate) struct OutputCapture
{
	pub image_logical_position: rectangle::Position,
	pub image_logical_size: rectangle::Size,
	pub image_mmap: memmap2::MmapMut,
	pub image_mmap_size: rectangle::Size,
}

pub(crate) struct State
{
	pub done: bool,
	pub wlr_screencopy_manager: Option<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1>,
	pub xdg_output_manager: Option<zxdg_output_manager_v1::ZxdgOutputManagerV1>,
	pub wl_shm: Option<wl_shm::WlShm>,
	pub output_infos: Vec<OutputInfo>,
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
						output_logical_position: None,
						output_logical_size: None,
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
				self.output_infos[*index].output_logical_position =
					Some(rectangle::Position { x, y });
			}
			// like logical position but for size
			zxdg_output_v1::Event::LogicalSize { width, height } =>
			{
				self.output_infos[*index].output_logical_size =
					Some(rectangle::Size { width, height });
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
					wl_shm::Format::Argb8888
					| wl_shm::Format::Xrgb8888
					| wl_shm::Format::Xbgr8888 => Some(format),
					_ => return,
				};

				self.output_infos[*index].image_mmap_size = Some(rectangle::Size {
					width: width as i32,
					height: height as i32,
				});

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

pub(crate) fn connect_and_get_output_info(
) -> Result<(State, wayland_client::EventQueue<State>), crate::Error>
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
		output_info.output_logical_position.is_none()
			|| output_info.output_logical_size.is_none()
			|| output_info.transform.is_none()
	})
	{
		event_queue.blocking_dispatch(&mut state)?;
	}

	Ok((state, event_queue))
}
