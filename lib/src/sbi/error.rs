use core::{error::Error, fmt::Display};

const SBI_ERR_FAILED: isize = -1;
const SBI_ERR_NOT_SUPPORTED: isize = -2;
const SBI_ERR_INVALID_PARAM: isize = -3;
const SBI_ERR_DENIED: isize = -4;
const SBI_ERR_INVALID_ADDRESS: isize = -5;
const SBI_ERR_ALREADY_AVAILABLE: isize = -6;
const SBI_ERR_ALREADY_STARTED: isize = -7;
const SBI_ERR_ALREADY_STOPPED: isize = -8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(isize)]
pub enum SbiError {
    Failed = SBI_ERR_FAILED,
    NotSupported = SBI_ERR_NOT_SUPPORTED,
    InvalidParam = SBI_ERR_INVALID_PARAM,
    Denied = SBI_ERR_DENIED,
    InvalidAddress = SBI_ERR_INVALID_ADDRESS,
    AlreadyAvailable = SBI_ERR_ALREADY_AVAILABLE,
    AlreadyStarted = SBI_ERR_ALREADY_STARTED,
    AlreadyStopped = SBI_ERR_ALREADY_STOPPED,
}

impl Display for SbiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SbiError::Failed => write!(f, "operation failed due to I/O errors"),
            SbiError::NotSupported => write!(f, "operation unsupported"),
            SbiError::InvalidParam => write!(f, "operation failed due to an parameter"),
            SbiError::Denied => write!(f, "the requested operation is forbidden"),
            SbiError::InvalidAddress => {
                write!(f, "operation failed due to an invalid memory address")
            }
            SbiError::AlreadyAvailable => write!(f, "the provided hartid is already started"),
            SbiError::AlreadyStarted => write!(
                f,
                "some of the counters specified in parameters are already started"
            ),
            SbiError::AlreadyStopped => write!(
                f,
                "some of the counters specified in parameters are already stopped"
            ),
        }
    }
}

impl Error for SbiError {}

impl From<isize> for SbiError {
    fn from(value: isize) -> Self {
        match value {
            SBI_ERR_FAILED => SbiError::Failed,
            SBI_ERR_NOT_SUPPORTED => SbiError::NotSupported,
            SBI_ERR_INVALID_PARAM => SbiError::InvalidParam,
            SBI_ERR_DENIED => SbiError::Denied,
            SBI_ERR_INVALID_ADDRESS => SbiError::InvalidAddress,
            SBI_ERR_ALREADY_AVAILABLE => SbiError::AlreadyAvailable,
            SBI_ERR_ALREADY_STARTED => SbiError::AlreadyStarted,
            SBI_ERR_ALREADY_STOPPED => SbiError::AlreadyStopped,
            _ => unreachable!(),
        }
    }
}
