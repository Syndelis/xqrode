#![feature(type_alias_impl_trait)]

mod rectangle;
mod wayland;

pub use crate::{
	rectangle::{Position, Size},
	wayland::{capture_all_outputs, capture_output, capture_region},
};

// All coordinates in this crate are absolute in the compositor coordinate space
// unless otherwise specified.

#[derive(thiserror::Error, Debug)]
pub enum Error
{
	#[error("output \"{0}\" was not found")]
	NoOutput(String),
	#[error("no captures when trying to construct the full capture")]
	NoCaptures,
	#[error("failed to connect to the wayland server")]
	Connect(#[from] wayland_client::ConnectError),
	#[error("failed to dispatch event from wayland server")]
	Dispatch(#[from] wayland_client::DispatchError),
	#[error("{0}")]
	Unimplemented(String),
}
