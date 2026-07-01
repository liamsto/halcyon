use crate::sbi::{error::SbiError, sbi_call};

const EXTN_SRST: usize = 0x5352_5354;
const FID_SYSTEM_RESET: usize = 0;
const PLATFORM_SPECIFIC_START: u32 = 0xF000_0000;
// const PLATFORM_SPECIFIC_END: u32 = 0xFFFF_FFFF; // unused in comparison as of now since self.0 <= PLATFORM_SPECIFIC_END is always true
const RESERVED_START: u32 = 0x00000003;
const RESERVED_END: u32 = 0xEFFFFFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ResetType(u32);

impl ResetType {
    pub const SHUTDOWN: Self = Self(0x0000_0000);
    pub const COLD_REBOOT: Self = Self(0x0000_0001);
    pub const WARM_REBOOT: Self = Self(0x0000_0002);

    pub const fn platform_specific(value: u32) -> Option<Self> {
        if value >= PLATFORM_SPECIFIC_START {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn value(self) -> usize {
        self.0 as usize
    }

    pub const fn is_platform_specific(self) -> bool {
        self.0 >= PLATFORM_SPECIFIC_START
    }

    pub const fn is_reserved(self) -> bool {
        self.0 >= RESERVED_START && self.0 <= RESERVED_END
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ResetReason(u32);

impl ResetReason {
    pub const NO_REASON: Self = Self(0x0000_0000);
    pub const SYSTEM_FAILURE: Self = Self(0x0000_0001);

    pub const fn platform_specific(value: u32) -> Option<Self> {
        if value >= PLATFORM_SPECIFIC_START {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn sbi_specific(value: u32) -> Option<Self> {
        const SBI_SPECIFIC_START: u32 = 0xE0000000;
        const SBI_SPECIFIC_END: u32 = 0xEFFFFFFF;

        if value >= SBI_SPECIFIC_START && value <= SBI_SPECIFIC_END {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn value(self) -> usize {
        self.0 as usize
    }

    pub const fn is_platform_specific(self) -> bool {
        self.0 >= PLATFORM_SPECIFIC_START
    }

    pub const fn is_reserved(self) -> bool {
        const RESERVED_START: u32 = 0x00000002;
        const RESERVED_END: u32 = 0xDFFFFFFF;
        self.0 >= RESERVED_START && self.0 <= RESERVED_END
    }
}

// Until https://github.com/rust-lang/rust/pull/155499 is (hopefully) merged,
// this will still return Result<(), SbiError>, even though technically a
// successful system reset should never return, i.e it should be updated to
// Result<!, SbiError> when ! is stable.
pub fn system_reset(
    reset_type: ResetType,
    reset_reason: Option<ResetReason>,
) -> Result<(), SbiError> {
    let reason = reset_reason.unwrap_or(ResetReason::NO_REASON);

    let ret = sbi_call(
        EXTN_SRST,
        FID_SYSTEM_RESET,
        reset_type.value(),
        reason.value(),
        0,
    );

    if ret.error < 0 {
        Err(SbiError::from(ret.error))
    } else {
        Ok(())
    }
}
