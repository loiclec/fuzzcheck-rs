//! A small, naive implementation of signal handlers in order to detect and
//! recover from crashes.

use libc::{c_int, c_void, siginfo_t};
use std::mem;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

lazy_static! {
    static ref PAIR: Arc<(Mutex<i32>, Condvar)> = Arc::new((Mutex::new(0), Condvar::new()));
    static ref PAIR2: Arc<(Mutex<i32>, Condvar)> = PAIR.clone();
}

pub fn handle_signals<F>(signals: Vec<i32>, handler: F)
where
    F: Fn(i32) -> ! + Send,
{
    fn signal_handler(sig: c_int, _info: *mut siginfo_t, _data: *mut c_void) {
        let &(ref lock, ref cvar) = &**PAIR2;
        let mut signal = lock.lock().unwrap();
        *signal = sig as i32;
        // We notify the condvar that the value has changed.
        cvar.notify_one();
    }

    let mut new: libc::sigaction = unsafe { mem::zeroed() };
    new.sa_sigaction = signal_handler as usize;

    for signal in &signals {
        // C data structure, expected to be zeroed out.
        let mut old: libc::sigaction = unsafe { mem::zeroed() };
        // FFI ‒ pointers are valid, it doesn't take ownership.
        if unsafe { libc::sigaction(*signal, &new, &mut old) } != 0 {
            panic!("Could not install signal handler")
        }
    }

    let builder = thread::Builder::new();

    let _ = unsafe {
        builder
            .spawn_unchecked(move || {
                let &(ref lock, ref cvar) = &**PAIR;
                let mut signal = lock.lock().unwrap();
                while *signal == 0 {
                    signal = cvar.wait(signal).unwrap();
                }
                handler(*signal);
            })
            .unwrap()
    };
}
