use std::{io::{self, Read, Seek, Write}, ffi, fs, os::unix::{self, io::FromRawFd}};
use regex::Regex;

use wayland_client::{
	protocol::{
        wl_callback,
		wl_registry,
        wl_output,
        wl_shm,
        wl_shm_pool,
        wl_buffer
	},
    Dispatch,
	Connection,
	QueueHandle
};

use wayland_protocols::{
	xdg::xdg_output::zv1::client::{
		zxdg_output_manager_v1,
		zxdg_output_v1
	}
};

use wayland_protocols_wlr::screencopy::v1::client::{zwlr_screencopy_manager_v1, zwlr_screencopy_frame_v1};

use nix::sys::memfd;

//use image::

struct QrodeOutput
{
    wl_output: wl_output::WlOutput,
    xdg_output: Option<zxdg_output_v1::ZxdgOutputV1>,
    logical_position: Option<(i32, i32)>,
    logical_size: Option<(i32, i32)>,
    transform: Option<wl_output::Transform>
}

struct QrodeScreencopyFrame
{
    wlr_screencopy_frame: zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    image_file: Option<fs::File>,
    size: Option<(u32, u32)>
}

struct State
{
    done: bool,
    output_done_events: i32,
    wlr_screencopy_manager: Option<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1>,
    xdg_output_manager: Option<zxdg_output_manager_v1::ZxdgOutputManagerV1>,
    qrode_outputs: Vec<QrodeOutput>,
    qrode_screencopy_frames: Vec<QrodeScreencopyFrame>,
    wl_shm: Option<wl_shm::WlShm>
}

impl Dispatch<wl_registry::WlRegistry, ()> for State
{
   fn event(
		&mut self,
		registry: &wl_registry::WlRegistry,
		event: wl_registry::Event,
		_: &(),
		_connection: &Connection,
		queue_handle: &QueueHandle<Self>
	)
	{
		if let wl_registry::Event::Global {
			name,
			interface,
			version: _
		} = event
		{
			match &interface[..]
			{
				"zwlr_screencopy_manager_v1" =>
				{
					let wlr_screencopy_manager = registry
						.bind::<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1, _, _>(name, 3, queue_handle, ())
						.unwrap();

					self.wlr_screencopy_manager = Some(wlr_screencopy_manager);

                    println!("Got screencopy manager");
				},
                "zxdg_output_manager_v1" =>
                {
                    let xdg_output_manager = registry
                        .bind::<zxdg_output_manager_v1::ZxdgOutputManagerV1, _, _>(name, 3, queue_handle, ())
                        .unwrap();

                    self.xdg_output_manager = Some(xdg_output_manager);

                    println!("Got output manager");
                },
                "wl_shm" =>
                {
                    let wl_shm = registry
                        .bind::<wl_shm::WlShm, _, _>(name, 1, queue_handle, ())
                        .unwrap();
                    
                    self.wl_shm = Some(wl_shm);
                },
                "wl_output" =>
                {
                    let wl_output = registry
                        .bind::<wl_output::WlOutput, _, _>(name, 4, queue_handle, ())
                        .unwrap();

                    self.qrode_outputs
                        .push
                        (
                            QrodeOutput
                            {
                                wl_output,
                                xdg_output: None,
                                logical_position: None,
                                logical_size: None,
                                transform: None
                            }
                        );
                    
                    println!("Got wl_output");
                }
				_ =>
				{
                    //println!("{}", interface);
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
        _queue_handle: &QueueHandle<Self>
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
        _queue_handle: &QueueHandle<Self>
    )
    {
        // only here to satusfy a trait bound
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for State
{
    fn event(
        &mut self,
        _wl_callback: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>
    )
    {
        match event
        {
            wl_callback::Event::Done{callback_data: _} => self.done = true,
            _ => println!("Unimplemented callback received.")
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for State
{
    fn event(
        &mut self,
        _wl_output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>
    )
    {
        match event
        {
            wl_output::Event::Done =>
            {
                self.output_done_events += 1;
                println!("Done");
            },
            wl_output::Event::Geometry{transform, ..} =>
            {
                let transform = transform.into_result().unwrap();
                
                println!("TRANSFORM: {:?}", transform);
            },
            _ => {}
        }
    }
}

impl Dispatch<zxdg_output_v1::ZxdgOutputV1, ()> for State
{
    fn event(
        &mut self,
        xdg_output: &zxdg_output_v1::ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        _: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>
    )
    {
        println!("Received properties: {:?}", event);

        let qrode_output = self.qrode_outputs
            .iter_mut()
            .find(|qrode_output| *qrode_output.xdg_output.as_ref().unwrap() == *xdg_output)
            .unwrap();
        
        match event
        {
            zxdg_output_v1::Event::LogicalPosition{x, y} =>
            {
                qrode_output.logical_position = Some((x, y));
            },
            zxdg_output_v1::Event::LogicalSize{width, height} =>
            {
                qrode_output.logical_size = Some((width, height));
            },
            _ => {}
        }
    }
}

impl Dispatch<zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1, ()> for State
{
    fn event(
        &mut self,
        wlr_screencopy_frame: &zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _: &(),
        _connection: &Connection,
        queue_handle: &QueueHandle<Self>
    )
    {
        let qrode_screencopy_frame = self.qrode_screencopy_frames
            .iter_mut()
            .find(|qrode_screencopy_frame| &qrode_screencopy_frame.wlr_screencopy_frame == wlr_screencopy_frame)
            .unwrap();
        
        match event
        {
            // TODO: handle other buffer events
            zwlr_screencopy_frame_v1::Event::Buffer{format, width, height, stride} =>
            {
                let raw_fd = 
                    memfd::memfd_create
                        (
                            ffi::CStr::from_bytes_with_nul(b"qrode\0").unwrap(),
                            // try with empy
                            memfd::MemFdCreateFlag::MFD_CLOEXEC | memfd::MemFdCreateFlag::MFD_ALLOW_SEALING
                            //memfd::MemFdCreateFlag::empty()
                        ).unwrap();
                
                //qrode_screencopy_frame.raw_fd = Some(raw_fd);
                qrode_screencopy_frame.image_file = Some(unsafe{fs::File::from_raw_fd(raw_fd)});
                qrode_screencopy_frame.size = Some((width, height));

                // TODO: handle other pixel formats
                qrode_screencopy_frame.image_file.as_mut().unwrap().set_len((width * height * 4) as u64);
                
                let wl_shm_pool = self.wl_shm.as_ref().unwrap().create_pool(raw_fd, (width * height * 4) as i32, queue_handle, ()).unwrap();

                println!("format: {:?}", format);
                let wl_buffer = wl_shm_pool.create_buffer(0, width as i32, height as i32, stride as i32, format.into_result().unwrap(), queue_handle, ()).unwrap();
                
                wlr_screencopy_frame.copy(&wl_buffer);
            },
            zwlr_screencopy_frame_v1::Event::BufferDone =>
            {
                self.output_done_events += 1;
            },
            zwlr_screencopy_frame_v1::Event::Ready{tv_sec_hi: _, tv_sec_lo: _, tv_nsec: _} =>
            {
                //let mut image_file = unsafe{fs::File::from_raw_fd(qrode_screencopy_frame.raw_fd.unwrap())};
                let mut image_file = qrode_screencopy_frame.image_file.as_mut().unwrap();
                //image_file.rewind();

                let mut tmp_file = fs::File::create(format!("/tmp/output{}.ppm", self.output_done_events)).unwrap();
                
                //io::copy(image_file, &mut tmp_file);
                let size = qrode_screencopy_frame.size.unwrap();

                // TODO: handle alternate buffer formats
                let mut buffer = vec![0 as u8; (size.0 * size.1 * 4) as usize];

                image_file.read_exact(&mut buffer).unwrap();
                
                write!(tmp_file, "P3\n{} {}\n255\n", size.0, size.1).unwrap();
                
                for i in (0..(size.0 * size.1 * 4)).step_by(4)
                {
                    // xbgr8888le to rgb888be
                    write!(tmp_file, "{} {} {}\n", buffer[i as usize], buffer[i as usize + 1], buffer[i as usize + 2]).unwrap();
                }
                
                tmp_file.flush().expect("Failed to flush.");
                
                //println!("buffer: {:?}", buffer);
                
                self.output_done_events += 1;
                println!("Ready");
            },
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
        event: wl_shm::Event,
        _: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>
    )
    {
        println!("WlShm event: {:?}", event);
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
        _queue_handle: &QueueHandle<Self>
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
        _queue_handle: &QueueHandle<Self>
    )
    {

    }
}

fn main()
{
    let mut buffer = String::new();
    let stdin = io::stdin();

    stdin.read_line(&mut buffer).expect("Failed to read from stdin.");

    let re = Regex::new(r"(\d+),(\d+) (\d+)x(\d+)").unwrap();

    let captures = re.captures(&buffer).expect("Failed to parse input.");

    let position = (captures.get(1).unwrap().as_str().parse::<i32>().unwrap(), captures.get(2).unwrap().as_str().parse::<i32>().unwrap());
    let size = (captures.get(3).unwrap().as_str().parse::<i32>().unwrap(), captures.get(4).unwrap().as_str().parse::<i32>().unwrap());

    println!("Position: ({}, {}), Size: ({}, {})", position.0, position.1, size.0, size.1);

    let connection = Connection::connect_to_env().expect("Failed to get connection to wayland server.");

    let mut event_queue = connection.new_event_queue();

    let queue_handle = event_queue.handle();

    let wl_display = connection.display();

    wl_display.get_registry(&queue_handle, ()).expect("Failed to create wayland registry object.");

    wl_display.sync(&queue_handle, ()).unwrap();

    let mut state = State
    {
        done: false,
        output_done_events: 0,
        wlr_screencopy_manager: None,
        xdg_output_manager: None,
        qrode_outputs: Vec::new(),
        qrode_screencopy_frames: Vec::new(),
        wl_shm: None
    };

    // run until initial done event and done event has been sent for all wl_outputs
    while !state.done || state.output_done_events < state.qrode_outputs.len() as i32
	{
		event_queue.blocking_dispatch(&mut state).unwrap();
	}

    // reset dones for xdg_output dones
    state.output_done_events = 0;
    
    for qrode_output in &mut state.qrode_outputs
    {
        qrode_output.xdg_output = state.xdg_output_manager.as_ref().unwrap().get_xdg_output(&qrode_output.wl_output, &queue_handle, ()).ok();
    }
    
    // run until all done event has been sent for all xdg_outputs
    while state.output_done_events < state.qrode_outputs.len() as i32
    {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
    
    state.output_done_events = 0;
    
    for qrode_output in &state.qrode_outputs
    {
        state.qrode_screencopy_frames
            .push
            (
                QrodeScreencopyFrame
                {
                    wlr_screencopy_frame: state.wlr_screencopy_manager.as_ref().unwrap().capture_output_region(0, &qrode_output.wl_output, position.0, position.1, size.0, size.1, &queue_handle, ()).unwrap(),
                    image_file: None,
                    size: None
                }
            );
    }
    
    while state.output_done_events < state.qrode_outputs.len() as i32
    {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
    
    state.output_done_events = 0;
    
    while state.output_done_events < state.qrode_outputs.len() as i32
    {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
}
