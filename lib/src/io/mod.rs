pub mod console;

pub fn putchar(ch: u8) {
    const UART0: *mut u8 = 0x1000_0000 as *mut u8;

    unsafe {
        UART0.write_volatile(ch);
    }
}

pub fn puts(s: &str) {
    for b in s.bytes() {
        putchar(b);
    }
}
