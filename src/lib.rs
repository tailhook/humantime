//! Human-friendly time parser
//!
//! Currently this only currently implements parsing of duration. Relative and
//! absolute times may be added in future.

#[macro_use] extern crate quick_error;

mod duration;
mod wrapper;

pub use duration::{parse_duration, Error as DurationError};
pub use wrapper::Duration;

