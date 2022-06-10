mod capture;
mod gazo;
mod rectangle;

pub use capture::FullCapture;
pub use gazo::capture_region;
pub use rectangle::{Position, Size};

// All coordinates in this crate are absolute in the compositor coordinate space
// unless otherwise specified.

#[derive(thiserror::Error, Debug)]
pub enum Error
{
	#[error("no captures when trying to construct the full capture")]
	NoCaptures,
	#[error("failed to connect to the wayland server")]
	Connect(#[from] wayland_client::ConnectError),
	#[error("failed to dispatch event from wayland server")]
	Dispatch(#[from] wayland_client::DispatchError),
	#[error("{0}")]
	Unimplemented(String),
}
