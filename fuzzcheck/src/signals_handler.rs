// ! A small, naive implementation of signal handlers in order to detect and
// ! recover from crashes.

use crate::nix_subset as nix;
use nix::signal;

static mut SIGNAL_HANDLER: Option<Box<dyn Fn(nix::libc::c_int) -> !>> = None;

extern "C" fn os_handler(s: nix::libc::c_int) {
    // Assuming this always succeeds. Can't really handle errors in any meaningful way.
    unsafe {
        reset_signal_handlers();
        if let Some(h) = SIGNAL_HANDLER.as_mut() {
            (*h)(s);
        } else {
            std::process::exit(1);
        }
    }
}
// TODO: remove nix dependency, only use libc?
pub unsafe fn set_signal_handlers<F: 'static>(f: F)
where
    F: Fn(nix::libc::c_int) -> !,
{
    SIGNAL_HANDLER = Some(Box::new(move |x| (f)(x)));

    let stack_size = nix::libc::SIGSTKSZ;

    let stack_pointer = std::alloc::alloc_zeroed(std::alloc::Layout::array::<std::ffi::c_void>(stack_size).unwrap())
        as *mut std::ffi::c_void;

    let signal_stack = nix::libc::stack_t {
        ss_sp: stack_pointer,
        ss_size: stack_size,
        ss_flags: 0,
    };

    let stack = nix::libc::sigaltstack(&signal_stack, std::ptr::null_mut());
    if stack == -1 {
        panic!("could not set alternate stack for handling signals");
    }

    let handler = signal::SigHandler::Handler(os_handler);

    let mut flags = signal::SaFlags::empty();
    flags.insert(signal::SaFlags::SA_ONSTACK);

    let new_action = signal::SigAction::new(handler, flags, signal::SigSet::empty());

    use signal::Signal::*;
    for &signal in &[SIGINT, SIGTERM, SIGSEGV, SIGBUS, SIGABRT, SIGFPE, SIGALRM] {
        signal::sigaction(signal, &new_action).expect("couldn't register signal");
    }
}

unsafe fn reset_signal_handlers() {
    let reset_action = signal::SigAction::new(
        signal::SigHandler::SigDfl,
        signal::SaFlags::empty(),
        signal::SigSet::empty(),
    );

    use signal::Signal::*;
    for &signal in &[SIGINT, SIGTERM, SIGSEGV, SIGBUS, SIGABRT, SIGFPE, SIGALRM] {
        signal::sigaction(signal, &reset_action).expect("couldn't register signal");
    }
}

extern "C" {
    fn setitimer(
        which: nix::libc::c_int,
        new_value: *mut nix::libc::itimerval,
        old_value: *mut nix::libc::itimerval,
    ) -> nix::libc::c_int;
}

pub fn set_timer(milliseconds: usize) {

    let seconds = milliseconds / 1000;
    let microseconds = (((milliseconds - (1000 * seconds)) * 1000) % (i32::MAX) as usize) as libc::suseconds_t;

    let mut tval = nix::libc::itimerval {
        it_interval: nix::libc::timeval {
            tv_sec: 0,
            tv_usec: microseconds,
        },
        it_value: nix::libc::timeval { tv_sec: 1, tv_usec: 0 },
    };
    let result_timer = unsafe {
        setitimer(0 /* ITIMER_REAL */, &mut tval as *mut _, std::ptr::null_mut())
    };
    if result_timer == -1 {
        panic!("could not set timer");
    }
}
