use core::arch::asm;

pub mod console;
pub mod error;
pub mod reset;
pub mod timer;

#[derive(Debug, Clone, Copy)]
pub struct SbiRet {
    pub error: isize,
    pub value: isize,
}

fn sbi_call(extension: usize, function: usize, arg0: usize, arg1: usize, arg2: usize) -> SbiRet {
    let error: isize;
    let value: isize;

    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 as isize => error,
            inlateout("a1") arg1 as isize => value,
            in("a2") arg2,
            in("a6") function,
            in("a7") extension,
        );
    }

    SbiRet { error, value }
}

const EXTN_BASE: usize = 0x10;
const FID_GET_SBI_SPEC_VER: usize = 0;
const FID_GET_IMPL_ID: usize = 1;
const FID_GET_IMPL_VER: usize = 2;
const FID_PROBE_SBI_EXT: usize = 3;
const FID_GET_VENDOR_ID: usize = 4;
const FID_GET_MARCH_ID: usize = 5;
const FID_GET_MIMPID: usize = 6;

pub fn spec_version() -> (isize, isize) {
    let v = sbi_call(EXTN_BASE, FID_GET_SBI_SPEC_VER, 0, 0, 0).value;
    let minor = v & 0x00ff_ffff;
    let major = (v >> 24) & 0x7f;
    (major, minor)
}

pub fn impl_id() -> ImplementationID {
    sbi_call(EXTN_BASE, FID_GET_IMPL_ID, 0, 0, 0).value.into()
}

pub fn impl_ver() -> isize {
    sbi_call(EXTN_BASE, FID_GET_IMPL_VER, 0, 0, 0).value
}

pub fn sbi_ext() -> isize {
    sbi_call(EXTN_BASE, FID_PROBE_SBI_EXT, 0, 0, 0).value
}

pub fn vendor_id() -> isize {
    sbi_call(EXTN_BASE, FID_GET_VENDOR_ID, 0, 0, 0).value
}

pub fn march_id() -> isize {
    sbi_call(EXTN_BASE, FID_GET_MARCH_ID, 0, 0, 0).value
}

pub fn mimpid() -> isize {
    sbi_call(EXTN_BASE, FID_GET_MIMPID, 0, 0, 0).value
}

#[repr(isize)]
pub enum ImplementationID {
    BerkeleyBootLoader,
    OpenSBI,
    Xvisor,
    Kvm,
    RustSBI,
    Diosix,
    Coffer,
    XenProject,
    PolarFireHartSoftwareServices,
    Coreboot,
    Oreboot,
    Bhyve,
    Unknown,
}

use core::fmt::Display;

impl Display for ImplementationID {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BerkeleyBootLoader => write!(f, "Berkeley Boot Loader"),
            Self::OpenSBI => write!(f, "OpenSBI"),
            Self::Xvisor => write!(f, "Xvisor"),
            Self::Kvm => write!(f, "KVM"),
            Self::RustSBI => write!(f, "RustSBI"),
            Self::Diosix => write!(f, "Diosix"),
            Self::Coffer => write!(f, "Coffer"),
            Self::XenProject => write!(f, "Xen Project"),
            Self::PolarFireHartSoftwareServices => write!(f, "PolarFire Hart Software Services"),
            Self::Coreboot => write!(f, "coreboot"),
            Self::Oreboot => write!(f, "oreboot"),
            Self::Bhyve => write!(f, "bhyve"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl From<isize> for ImplementationID {
    fn from(value: isize) -> Self {
        match value {
            0 => Self::BerkeleyBootLoader,
            1 => Self::OpenSBI,
            2 => Self::Xvisor,
            3 => Self::Kvm,
            4 => Self::RustSBI,
            5 => Self::Diosix,
            6 => Self::Coffer,
            7 => Self::XenProject,
            8 => Self::PolarFireHartSoftwareServices,
            9 => Self::Coreboot,
            10 => Self::Oreboot,
            11 => Self::Bhyve,
            _ => Self::Unknown,
        }
    }
}
