use crate::io::puts;
use crate::sbi::dbg_console;
use core::fmt::Write;

struct Console;

impl Write for Console {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match dbg_console::write(s) {
            Ok(_) => Ok(()),
            Err(_) => {
                puts("an error occured");
                Ok(())
            }
        }
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        match dbg_console::write_byte(c as u8) {
            Ok(_) => Ok(()),
            Err(e) => {
                puts("an error occured");
                Ok(())
            }
        }
    }
}

pub fn print(args: core::fmt::Arguments) {
    Console.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::io::console::print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
