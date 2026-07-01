use crate::sbi::sbi_call;
use core::fmt::Display;

const EXTN_BASE: usize = 0x10;
const FID_GET_SBI_SPEC_VER: usize = 0;
const FID_GET_IMPL_ID: usize = 1;
const FID_GET_IMPL_VER: usize = 2;
const FID_PROBE_SBI_EXT: usize = 3;
const FID_GET_VENDOR_ID: usize = 4;
const FID_GET_MARCH_ID: usize = 5;
const FID_GET_MIMPID: usize = 6;

pub fn get_spec_version() -> (isize, isize) {
    let v = sbi_call(EXTN_BASE, FID_GET_SBI_SPEC_VER, 0, 0, 0).value;
    let minor = v & 0x00ff_ffff;
    let major = (v >> 24) & 0x7f;
    (major, minor)
}

pub fn get_impl_id() -> ImplementationID {
    sbi_call(EXTN_BASE, FID_GET_IMPL_ID, 0, 0, 0).value.into()
}

pub fn get_impl_ver() -> isize {
    sbi_call(EXTN_BASE, FID_GET_IMPL_VER, 0, 0, 0).value
}

pub fn probe_sbi_ext() -> isize {
    sbi_call(EXTN_BASE, FID_PROBE_SBI_EXT, 0, 0, 0).value
}

pub fn get_vendor_id() -> isize {
    sbi_call(EXTN_BASE, FID_GET_VENDOR_ID, 0, 0, 0).value
}

pub fn get_march_id() -> isize {
    sbi_call(EXTN_BASE, FID_GET_MARCH_ID, 0, 0, 0).value
}

pub fn get_mimpid() -> isize {
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
