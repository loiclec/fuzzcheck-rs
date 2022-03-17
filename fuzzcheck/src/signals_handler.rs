// ! A small, naive implementation of signal handlers in order to detect and
// ! recover from crashes.

use std::ptr;

use libc::{
    sigaction, sigemptyset, SA_NODEFER, SA_ONSTACK, SA_SIGINFO, SIGABRT, SIGALRM, SIGBUS, SIGFPE, SIGINT, SIGSEGV,
    SIGTERM, SIGTRAP, SIG_DFL,
};

static mut SIGNAL_HANDLER: Option<Box<dyn Fn(libc::c_int) -> !>> = None;

#[no_coverage]
extern "C" fn os_handler(signal: libc::c_int, _: libc::siginfo_t, _: *mut libc::c_void) {
    // Assuming this always succeeds. Can't really handle errors in any meaningful way.
    unsafe {
        reset_signal_handlers();
        if let Some(h) = SIGNAL_HANDLER.as_mut() {
            (*h)(signal);
        } else {
            std::process::exit(1);
        }
    }
}

#[no_coverage]
pub unsafe fn set_signal_handlers<F: 'static>(f: F)
where
    F: Fn(libc::c_int) -> !,
{
    SIGNAL_HANDLER = Some(Box::new(f));

    let stack_size = libc::SIGSTKSZ;

    let stack_pointer = std::alloc::alloc_zeroed(std::alloc::Layout::array::<std::ffi::c_void>(stack_size).unwrap())
        as *mut std::ffi::c_void;

    let signal_stack = libc::stack_t {
        ss_sp: stack_pointer,
        ss_size: stack_size,
        ss_flags: 0,
    };

    let stack = libc::sigaltstack(&signal_stack, std::ptr::null_mut());
    if stack == -1 {
        panic!("could not set alternate stack for handling signals");
    }

    let mut sa: sigaction = std::mem::zeroed();
    sigemptyset(&mut sa.sa_mask as *mut libc::sigset_t);

    sa.sa_flags = SA_NODEFER | SA_SIGINFO | SA_ONSTACK;
    sa.sa_sigaction = os_handler as usize;

    let signals = [
        SIGALRM, SIGINT, SIGTERM, SIGSEGV, SIGBUS, SIGABRT, SIGFPE, SIGABRT, SIGTRAP,
    ];
    for sig in signals {
        if sigaction(sig as i32, &mut sa as *mut sigaction, ptr::null_mut()) < 0 {
            panic!("Could not set up signal handler");
        }
    }
}

#[no_coverage]
pub(crate) unsafe fn reset_signal_handlers() {
    let mut sa: sigaction = std::mem::zeroed();
    sigemptyset(&mut sa.sa_mask as *mut libc::sigset_t);
    sa.sa_sigaction = SIG_DFL;

    for &signal in &[
        SIGALRM, SIGINT, SIGTERM, SIGSEGV, SIGBUS, SIGABRT, SIGFPE, SIGABRT, SIGTRAP,
    ] {
        if sigaction(signal, &mut sa as *mut sigaction, ptr::null_mut()) < 0 {
            panic!("Could not set up signal handler");
        }
    }
}
