//! Supervisor Binary Interface calls.
//!
//! See the [SBI specification](https://github.com/riscv-non-isa/riscv-sbi-doc).

use core::{arch::asm, error, fmt};

const EXT_BASE: usize = 0x10;
const FID_SPEC_VERSION: usize = 0;
const FID_IMPL_ID: usize = 1;
const FID_IMPL_VERSION: usize = 2;
const FID_PROBE_EXT: usize = 3;
const FID_MVENDORID: usize = 4;
const FID_MARCHID: usize = 5;
const FID_MIMPID: usize = 6;

const EXT_TIME: usize = 0x5449_4D45;
const FID_SET_TIMER: usize = 0;

const EXT_SRST: usize = 0x5352_5354;
const FID_SYSTEM_RESET: usize = 0;

const PLATFORM_SPECIFIC_START: u32 = 0xF000_0000;

/// Calls function `fid` of extension `eid`.
///
/// # Safety
/// The arguments must satisfy the requirements of the called function. Pointer arguments are
/// physical addresses.
pub(crate) unsafe fn call(
    eid: usize,
    fid: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
) -> Result<usize, Error> {
    let error: usize;
    let value: usize;

    // SAFETY: The SBI preserves all registers except a0 and a1. The caller upholds the
    // requirements of the called function.
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => error,
            inlateout("a1") arg1 => value,
            in("a2") arg2,
            in("a6") fid,
            in("a7") eid,
        );
    }

    match error as isize {
        0 => Ok(value),
        code => Err(Error::from_code(code)),
    }
}

/// Error returned by an SBI call.
///
/// The meaning of each error depends on the function that returned it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Failed,
    NotSupported,
    InvalidParam,
    Denied,
    InvalidAddress,
    AlreadyAvailable,
    AlreadyStarted,
    AlreadyStopped,
    NoShmem,
    InvalidState,
    BadRange,
    Timeout,
    Io,
    DeniedLocked,
    /// Error code not defined by the SBI specification as of version 3.0.
    Unknown(isize),
}

impl Error {
    fn from_code(code: isize) -> Self {
        match code {
            -1 => Self::Failed,
            -2 => Self::NotSupported,
            -3 => Self::InvalidParam,
            -4 => Self::Denied,
            -5 => Self::InvalidAddress,
            -6 => Self::AlreadyAvailable,
            -7 => Self::AlreadyStarted,
            -8 => Self::AlreadyStopped,
            -9 => Self::NoShmem,
            -10 => Self::InvalidState,
            -11 => Self::BadRange,
            -12 => Self::Timeout,
            -13 => Self::Io,
            -14 => Self::DeniedLocked,
            code => Self::Unknown(code),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed => f.write_str("failed"),
            Self::NotSupported => f.write_str("not supported"),
            Self::InvalidParam => f.write_str("invalid parameter"),
            Self::Denied => f.write_str("denied"),
            Self::InvalidAddress => f.write_str("invalid address"),
            Self::AlreadyAvailable => f.write_str("already available"),
            Self::AlreadyStarted => f.write_str("already started"),
            Self::AlreadyStopped => f.write_str("already stopped"),
            Self::NoShmem => f.write_str("shared memory not available"),
            Self::InvalidState => f.write_str("invalid state"),
            Self::BadRange => f.write_str("bad range"),
            Self::Timeout => f.write_str("timed out"),
            Self::Io => f.write_str("I/O error"),
            Self::DeniedLocked => f.write_str("denied, locked"),
            Self::Unknown(code) => write!(f, "unknown error {code}"),
        }
    }
}

impl error::Error for Error {}

/// Calls a base extension function.
fn base(fid: usize, arg0: usize) -> usize {
    // SAFETY: Base extension functions only report firmware information.
    unsafe { call(EXT_BASE, fid, arg0, 0, 0) }.expect("base extension functions always succeed")
}

/// SBI specification version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u8,
    pub minor: u32,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Returns the SBI specification version implemented by the firmware.
pub fn spec_version() -> Version {
    let version = base(FID_SPEC_VERSION, 0);
    Version {
        major: ((version >> 24) & 0x7f) as u8,
        minor: (version & 0xff_ffff) as u32,
    }
}

/// SBI implementation, as reported by [`impl_id`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplId {
    BerkeleyBootLoader,
    OpenSbi,
    Xvisor,
    Kvm,
    RustSbi,
    Diosix,
    Coffer,
    XenProject,
    PolarFireHartSoftwareServices,
    Coreboot,
    Oreboot,
    Bhyve,
    /// Unknown implementation ID.
    Other(usize),
}

impl fmt::Display for ImplId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BerkeleyBootLoader => f.write_str("Berkeley Boot Loader"),
            Self::OpenSbi => f.write_str("OpenSBI"),
            Self::Xvisor => f.write_str("Xvisor"),
            Self::Kvm => f.write_str("KVM"),
            Self::RustSbi => f.write_str("RustSBI"),
            Self::Diosix => f.write_str("Diosix"),
            Self::Coffer => f.write_str("Coffer"),
            Self::XenProject => f.write_str("Xen Project"),
            Self::PolarFireHartSoftwareServices => f.write_str("PolarFire Hart Software Services"),
            Self::Coreboot => f.write_str("coreboot"),
            Self::Oreboot => f.write_str("oreboot"),
            Self::Bhyve => f.write_str("bhyve"),
            Self::Other(id) => write!(f, "unknown ({id})"),
        }
    }
}

/// Returns the ID of the SBI implementation.
pub fn impl_id() -> ImplId {
    match base(FID_IMPL_ID, 0) {
        0 => ImplId::BerkeleyBootLoader,
        1 => ImplId::OpenSbi,
        2 => ImplId::Xvisor,
        3 => ImplId::Kvm,
        4 => ImplId::RustSbi,
        5 => ImplId::Diosix,
        6 => ImplId::Coffer,
        7 => ImplId::XenProject,
        8 => ImplId::PolarFireHartSoftwareServices,
        9 => ImplId::Coreboot,
        10 => ImplId::Oreboot,
        11 => ImplId::Bhyve,
        id => ImplId::Other(id),
    }
}

/// Returns the version of the SBI implementation.
///
/// The encoding is specific to the implementation.
pub fn impl_version() -> usize {
    base(FID_IMPL_VERSION, 0)
}

/// Returns `true` if the firmware implements extension `eid`.
pub fn probe_extension(eid: usize) -> bool {
    base(FID_PROBE_EXT, eid) != 0
}

/// Returns the `mvendorid` CSR value.
pub fn mvendorid() -> usize {
    base(FID_MVENDORID, 0)
}

/// Returns the `marchid` CSR value.
pub fn marchid() -> usize {
    base(FID_MARCHID, 0)
}

/// Returns the `mimpid` CSR value.
pub fn mimpid() -> usize {
    base(FID_MIMPID, 0)
}

/// Schedules the next timer interrupt at `time`, in ticks of the `time` CSR.
///
/// Also clears the pending timer interrupt. Pass `u64::MAX` to clear it without scheduling a new
/// one.
pub fn set_timer(time: u64) -> Result<(), Error> {
    // SAFETY: Programming the timer has no memory effects. `usize` is 64 bits on this target, so
    // `time` fits in one argument.
    unsafe { call(EXT_TIME, FID_SET_TIMER, time as usize, 0, 0) }.map(drop)
}

/// Kind of system reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetType(u32);

impl ResetType {
    pub const SHUTDOWN: Self = Self(0);
    pub const COLD_REBOOT: Self = Self(1);
    pub const WARM_REBOOT: Self = Self(2);

    /// Returns a platform-specific reset type, or `None` if `value` is below `0xF000_0000`.
    pub const fn platform_specific(value: u32) -> Option<Self> {
        if value >= PLATFORM_SPECIFIC_START {
            Some(Self(value))
        } else {
            None
        }
    }
}

/// Reason for a system reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetReason(u32);

impl ResetReason {
    pub const NO_REASON: Self = Self(0);
    pub const SYSTEM_FAILURE: Self = Self(1);

    /// Returns a platform-specific reset reason, or `None` if `value` is below `0xF000_0000`.
    pub const fn platform_specific(value: u32) -> Option<Self> {
        if value >= PLATFORM_SPECIFIC_START {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns an SBI implementation specific reset reason, or `None` if `value` is outside
    /// `0xE000_0000..=0xEFFF_FFFF`.
    pub const fn sbi_specific(value: u32) -> Option<Self> {
        if value >= 0xE000_0000 && value <= 0xEFFF_FFFF {
            Some(Self(value))
        } else {
            None
        }
    }
}

/// Resets the system.
///
/// Returns only if the reset failed.
pub fn system_reset(kind: ResetType, reason: ResetReason) -> Error {
    // SAFETY: Resetting ends execution, so obviously nothing observes memory afterwards.
    match unsafe {
        call(
            EXT_SRST,
            FID_SYSTEM_RESET,
            kind.0 as usize,
            reason.0 as usize,
            0,
        )
    } {
        Ok(_) => unreachable!("SBI system reset returned success"),
        Err(err) => err,
    }
}
