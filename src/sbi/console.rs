use crate::sbi::{error::SbiError, sbi_call};

const EXT_DBCN: usize = 0x4442434E;
const FID_CONSOLE_WRITE: usize = 0;
const FID_CONSOLE_READ: usize = 1;
const FID_CONSOLE_WRITE_BYTE: usize = 2;

/// # Function: Console Write (FID #0)
/// Write bytes to the debug console from input memory.
///
/// See the [SBI Documentation](https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/src/ext-debug-console.adoc#function-console-write-fid-0) for more information.
/// ## Errors
/// - Returns `SbiError::InvalidParam` if the `num_bytes`, `base_addr_lo`, and `base_addr_hi` parameters from the call to `sbi_call` are invalid.
/// - Returns `SbiError::Denied` if writing to the debug console is not allowed.
/// - Returns `SbiError::Failed` if the write fails because of I/O errors.
pub fn write(msg: &str) -> Result<usize, SbiError> {
    let addr = msg.as_ptr() as usize;

    let ret = sbi_call(EXT_DBCN, FID_CONSOLE_WRITE, msg.len(), addr, 0);

    if ret.error < 0 {
        Err(SbiError::from(ret.error))
    } else {
        Ok(ret.value as usize)
    }
}

pub fn read(buf: &mut [u8]) -> Result<usize, SbiError> {
    if buf.is_empty() {
        return Ok(0);
    }

    let addr = buf.as_mut_ptr() as usize;

    let ret = sbi_call(
        EXT_DBCN,
        FID_CONSOLE_READ,
        buf.len(),
        addr,
        0, // unless >XLEN phys addrs?
    );
    if ret.error < 0 {
        Err(SbiError::from(ret.error))
    } else {
        Ok(ret.value as usize)
    }
}

pub fn write_byte(byte: u8) -> Result<usize, SbiError> {
    let ret = sbi_call(EXT_DBCN, FID_CONSOLE_WRITE_BYTE, byte as usize, 0, 0);
    if ret.error < 0 {
        Err(SbiError::from(ret.error))
    } else {
        Ok(ret.value as usize)
    }
}
