//! Gazo is a crate to capture screen pixel data on Wayland compositors
//! implementing the wlr_screencopy protocol. All coordinates in this crate are
//! absolute in the logical compositor coordinate space.

#![deny(missing_docs)]
#![feature(type_alias_impl_trait)]

mod capture;
mod rectangle;
mod wayland;

pub use crate::wayland::{capture_all_outputs, capture_output, capture_region};

/// Enum representing potential errors.
#[derive(thiserror::Error, Debug)]
pub enum Error
{
	/// This error will only be returned by [`capture_output`] when the given
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
	/// Error thrown in the event of an unimplemented handler; hopefully this
	/// will be removed soon.
	#[error("{0}")]
	Unimplemented(String),
}
