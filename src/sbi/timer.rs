use crate::sbi::sbi_call;

const EXTN_TIME: usize = 0x54494D45;
const FID_SET_TIMER: usize = 0;

pub fn set_timer(time: u64) -> () {
    sbi_call(EXTN_TIME, FID_SET_TIMER, time as usize, 0, 0);
}
