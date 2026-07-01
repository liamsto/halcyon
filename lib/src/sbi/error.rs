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
            Self::Failed => write!(f, "operation failed due to I/O errors"),
            Self::NotSupported => write!(f, "operation unsupported"),
            Self::InvalidParam => write!(f, "operation failed due to an parameter"),
            Self::Denied => write!(f, "the requested operation is forbidden"),
            Self::InvalidAddress => {
                write!(f, "operation failed due to an invalid memory address")
            }
            Self::AlreadyAvailable => write!(f, "the provided hartid is already started"),
            Self::AlreadyStarted => write!(
                f,
                "some of the counters specified in parameters are already started"
            ),
            Self::AlreadyStopped => write!(
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
            SBI_ERR_FAILED => Self::Failed,
            SBI_ERR_NOT_SUPPORTED => Self::NotSupported,
            SBI_ERR_INVALID_PARAM => Self::InvalidParam,
            SBI_ERR_DENIED => Self::Denied,
            SBI_ERR_INVALID_ADDRESS => Self::InvalidAddress,
            SBI_ERR_ALREADY_AVAILABLE => Self::AlreadyAvailable,
            SBI_ERR_ALREADY_STARTED => Self::AlreadyStarted,
            SBI_ERR_ALREADY_STOPPED => Self::AlreadyStopped,
            _ => unreachable!(),
        }
    }
}
