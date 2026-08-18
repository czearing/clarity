//! Writing to a reader who may stop reading.
//!
//! When the reader closes the pipe the next write fails. The print macros treat a failed write as
//! a fatal error, so a broken pipe turns correct output into a crash report. Observed on a real
//! run of the naming pass piped to `head`.
//!
//! A reader who has stopped reading is not an error. It is the ordinary end of a report, so it
//! ends the run quietly and successfully, which is what every other tool on the command line does.

use std::io::Write;

/// Write to standard output, and stop quietly if nothing is reading.
///
/// The exit is taken here rather than handed back to the caller because there is nothing a caller
/// could do with it: every line still to come would go the same way.
pub fn put(args: core::fmt::Arguments) {
    let mut out = std::io::stdout().lock();
    if out.write_fmt(args).is_err() {
        std::process::exit(0);
    }
}

/// Write a line to standard output, and stop quietly if nothing is reading.
#[macro_export]
macro_rules! say {
    () => { $crate::out::put(core::format_args!("\n")) };
    ($($arg:tt)*) => {
        $crate::out::put(core::format_args!("{}\n", core::format_args!($($arg)*)))
    };
}

/// Write to standard output without ending the line, and stop quietly if nothing is reading.
#[macro_export]
macro_rules! put {
    ($($arg:tt)*) => { $crate::out::put(core::format_args!($($arg)*)) };
}
