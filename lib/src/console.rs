//! Debug console I/O through the SBI Debug Console extension.

use crate::sbi::{self, Error};
use core::fmt::{self, Write};

const EXT_DBCN: usize = 0x4442_434E;
const FID_WRITE: usize = 0;
const FID_READ: usize = 1;

// TODO: DBCN takes physical addresses. Passing pointers directly only works while kernel memory is
// identity mapped.

/// Writes bytes to the console, returning how many were written.
fn write(buf: &[u8]) -> Result<usize, Error> {
    // SAFETY: `buf` is valid for reads of `buf.len()` bytes.
    unsafe { sbi::call(EXT_DBCN, FID_WRITE, buf.len(), buf.as_ptr() as usize, 0) }
}

/// Reads pending console input into `buf`, returning how many bytes were read.
///
/// Nonblocking, returns `Ok(0)` if no input is pending.
pub fn read(buf: &mut [u8]) -> Result<usize, Error> {
    if buf.is_empty() {
        return Ok(0);
    }

    // SAFETY: `buf` is valid for writes of `buf.len()` bytes.
    unsafe { sbi::call(EXT_DBCN, FID_READ, buf.len(), buf.as_mut_ptr() as usize, 0) }
}

struct Console;

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut buf = s.as_bytes();
        // The SBI may write fewer bytes than requested.
        while !buf.is_empty() {
            let written = write(buf).map_err(|_| fmt::Error)?;
            buf = buf.get(written..).ok_or(fmt::Error)?;
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    // A failed console has nowhere to report errors, and panicking would recurse through the panic handler, so if this doesn't work, too bad for you!
    let _ = Console.write_fmt(args);
}

/// Prints to the debug console.
///
/// Console errors are ignored.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

/// Prints to the debug console, with a newline.
///
/// Console errors are ignored.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
