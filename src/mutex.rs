use parking_lot::lock_api::{RawMutex, GuardSend};
use parking_lot::RawMutex as RMutex;
use chrono::{Utc, DateTime, TimeZone};
use std::time::Duration;
use std::sync::Mutex;

/// RMutexMax is a mutex with maximum lifetime

pub struct RMutexMax {
    mutex: RMutex,
    time: Mutex<DateTime::<Utc>>,
}

pub unsafe trait RawMutexMax: RawMutex {
    fn try_lock_max(&self, timeout: Duration) -> bool;
}

unsafe impl RawMutexMax for RMutexMax {
    fn try_lock_max(&self, timeout: Duration) -> bool {
        if self.mutex.try_lock() {
            *self.time.lock().unwrap() = Utc::now() + chrono::Duration::from_std(timeout).unwrap();

            true
        } else {
            let now = Utc::now();
            if now > *self.time.lock().unwrap() {
                unsafe {
                    self.mutex.unlock();
                    *self.time.lock().unwrap() = now + chrono::Duration::from_std(timeout).unwrap();
                    
                    return self.mutex.try_lock()
                }
            }

            false
        }
    }
}

unsafe impl RawMutex for RMutexMax {
    const INIT: Self = RMutexMax {
        mutex: RMutex::INIT,
        time: Mutex::new(DateTime::<Utc>::MIN_UTC),
    };

    type GuardMarker = GuardSend;

    fn lock(&self) {
        *self.time.lock().unwrap() = DateTime::<Utc>::MAX_UTC;

        self.mutex.lock()
    }

    fn try_lock(&self) -> bool {
        *self.time.lock().unwrap() = DateTime::<Utc>::MAX_UTC;

        self.mutex.try_lock()
    }

    unsafe fn unlock(&self) {
        self.mutex.unlock()
    }
}