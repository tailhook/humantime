//! Human-friendly time parser
//!
//! Currently this only currently implements parsing of duration. Relative and
//! absolute times may be added in future.
//!
//! The format of values accpted is described in docstring of `parse_duration`.
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

#[macro_use] extern crate quick_error;

mod duration;
mod wrapper;
mod date;

pub use duration::{parse_duration, Error as DurationError};
pub use wrapper::{Duration, Timestamp};
pub use date::{parse_rfc3339, parse_rfc3339_weak, Error as TimestampError};
pub use date::{format_rfc3339, format_rfc3339_seconds, format_rfc3339_nanos};
