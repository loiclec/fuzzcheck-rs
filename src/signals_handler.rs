use crate::libc::*;

use std::mem;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::mem::MaybeUninit;

use std::sync::Once;
static START: Once = Once::new();


static mut PAIR: MaybeUninit<Arc<(Mutex<i32>, Condvar)>> = MaybeUninit::<_>::uninit();
static mut PAIR2: MaybeUninit<Arc<(Mutex<i32>, Condvar)>> = MaybeUninit::<_>::uninit();

pub fn pair() -> &'static Arc<(Mutex<i32>, Condvar)> {
    unsafe { & *PAIR.as_ptr() }
}
pub fn pair2() -> &'static Arc<(Mutex<i32>, Condvar)> {
    unsafe { & *PAIR2.as_ptr() }
}


pub fn handle_signals<F>(signals: Vec<i32>, handler: F)
where
    F: Fn(i32) -> ! + Send,
{
    unsafe {
        START.call_once(|| {
            PAIR.as_mut_ptr().write(Arc::new((Mutex::new(0), Condvar::new())));
            PAIR2.as_mut_ptr().write(pair().clone());
        });
    }

    fn signal_handler(sig: c_int, _info: *mut siginfo_t, _data: *mut c_void) {
        let &(ref lock, ref cvar) = &*(*pair2());
        let mut signal = lock.lock().unwrap();
        *signal = sig as i32;
        // We notify the condvar that the value has changed.
        cvar.notify_one();
    }

    let mut new: sigaction = unsafe { mem::zeroed() };
    new.sa_sigaction = signal_handler as usize;

    for signal in signals.iter() {
        // C data structure, expected to be zeroed out.
        let mut old: sigaction = unsafe { mem::zeroed() };
        // FFI ‒ pointers are valid, it doesn't take ownership.
        if unsafe { sigaction(*signal, &new, &mut old) } != 0 {
            panic!("Could not install signal handler")
        }
    }

    let builder = thread::Builder::new();

    let _ = unsafe {
        builder
            .spawn_unchecked(move || {
                let &(ref lock, ref cvar) = &*(*pair());
                let mut signal = lock.lock().unwrap();
                while *signal == 0 {
                    signal = cvar.wait(signal).unwrap();
                }
                handler(*signal);
            })
            .unwrap()
    };
}
