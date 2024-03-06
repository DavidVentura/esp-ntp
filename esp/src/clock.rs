use chrono::{DateTime, Utc};
use libc;

pub fn now() -> DateTime<Utc> {
    let mut tp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let is_error =
        unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut tp as *mut libc::timespec) };
    assert_eq!(is_error, 0);

    DateTime::from_timestamp(tp.tv_sec, tp.tv_nsec.try_into().unwrap()).unwrap()
}

pub fn set_time(now: DateTime<Utc>) {
    let tv_sec = now.timestamp();
    let tv_nsec = now.timestamp_subsec_nanos().try_into().unwrap();
    let tp = libc::timespec { tv_sec, tv_nsec };

    let ret = unsafe { libc::clock_settime(libc::CLOCK_REALTIME, &tp as *const libc::timespec) };
}
