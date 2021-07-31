// Portions of this file are Copyright 2014 The Rust Project Developers.
// See http://rust-lang.org/COPYRIGHT.

use super::errno::Errno;
///! Operating system signals.
use super::{Error, Result};
use std::convert::TryFrom;
use std::fmt;
use std::mem;
#[cfg(any(target_os = "dragonfly", target_os = "freebsd"))]
use std::os::unix::io::RawFd;

use std::str::FromStr;

libc_enum! {
    // Currently there is only one definition of c_int in libc, as well as only one
    // type for signal constants.
    // We would prefer to use the libc::c_int alias in the repr attribute. Unfortunately
    // this is not (yet) possible.
    #[repr(i32)]
    pub enum Signal {
        SIGHUP,
        SIGINT,
        SIGQUIT,
        SIGILL,
        SIGTRAP,
        SIGABRT,
        SIGBUS,
        SIGFPE,
        SIGKILL,
        SIGUSR1,
        SIGSEGV,
        SIGUSR2,
        SIGPIPE,
        SIGALRM,
        SIGTERM,
        #[cfg(all(any(target_os = "android", target_os = "emscripten", target_os = "linux"),
                  not(any(target_arch = "mips", target_arch = "mips64", target_arch = "sparc64"))))]
        SIGSTKFLT,
        SIGCHLD,
        SIGCONT,
        SIGSTOP,
        SIGTSTP,
        SIGTTIN,
        SIGTTOU,
        SIGURG,
        SIGXCPU,
        SIGXFSZ,
        SIGVTALRM,
        SIGPROF,
        SIGWINCH,
        SIGIO,
        #[cfg(any(target_os = "android", target_os = "emscripten", target_os = "linux"))]
        SIGPWR,
        SIGSYS,
        #[cfg(not(any(target_os = "android", target_os = "emscripten",
                      target_os = "linux", target_os = "redox")))]
        SIGEMT,
        #[cfg(not(any(target_os = "android", target_os = "emscripten",
                      target_os = "linux", target_os = "redox")))]
        SIGINFO,
    }
}

impl FromStr for Signal {
    type Err = Error;
    #[no_coverage]
    fn from_str(s: &str) -> Result<Signal> {
        Ok(match s {
            "SIGHUP" => Signal::SIGHUP,
            "SIGINT" => Signal::SIGINT,
            "SIGQUIT" => Signal::SIGQUIT,
            "SIGILL" => Signal::SIGILL,
            "SIGTRAP" => Signal::SIGTRAP,
            "SIGABRT" => Signal::SIGABRT,
            "SIGBUS" => Signal::SIGBUS,
            "SIGFPE" => Signal::SIGFPE,
            "SIGKILL" => Signal::SIGKILL,
            "SIGUSR1" => Signal::SIGUSR1,
            "SIGSEGV" => Signal::SIGSEGV,
            "SIGUSR2" => Signal::SIGUSR2,
            "SIGPIPE" => Signal::SIGPIPE,
            "SIGALRM" => Signal::SIGALRM,
            "SIGTERM" => Signal::SIGTERM,
            #[cfg(all(
                any(target_os = "android", target_os = "emscripten", target_os = "linux"),
                not(any(target_arch = "mips", target_arch = "mips64", target_arch = "sparc64"))
            ))]
            "SIGSTKFLT" => Signal::SIGSTKFLT,
            "SIGCHLD" => Signal::SIGCHLD,
            "SIGCONT" => Signal::SIGCONT,
            "SIGSTOP" => Signal::SIGSTOP,
            "SIGTSTP" => Signal::SIGTSTP,
            "SIGTTIN" => Signal::SIGTTIN,
            "SIGTTOU" => Signal::SIGTTOU,
            "SIGURG" => Signal::SIGURG,
            "SIGXCPU" => Signal::SIGXCPU,
            "SIGXFSZ" => Signal::SIGXFSZ,
            "SIGVTALRM" => Signal::SIGVTALRM,
            "SIGPROF" => Signal::SIGPROF,
            "SIGWINCH" => Signal::SIGWINCH,
            "SIGIO" => Signal::SIGIO,
            #[cfg(any(target_os = "android", target_os = "emscripten", target_os = "linux"))]
            "SIGPWR" => Signal::SIGPWR,
            "SIGSYS" => Signal::SIGSYS,
            #[cfg(not(any(
                target_os = "android",
                target_os = "emscripten",
                target_os = "linux",
                target_os = "redox"
            )))]
            "SIGEMT" => Signal::SIGEMT,
            #[cfg(not(any(
                target_os = "android",
                target_os = "emscripten",
                target_os = "linux",
                target_os = "redox"
            )))]
            "SIGINFO" => Signal::SIGINFO,
            _ => return Err(Error::invalid_argument()),
        })
    }
}

impl Signal {
    /// Returns name of signal.
    ///
    /// This function is equivalent to `<Signal as AsRef<str>>::as_ref()`,
    /// with difference that returned string is `'static`
    /// and not bound to `self`'s lifetime.
    #[no_coverage]
    pub fn as_str(self) -> &'static str {
        match self {
            Signal::SIGHUP => "SIGHUP",
            Signal::SIGINT => "SIGINT",
            Signal::SIGQUIT => "SIGQUIT",
            Signal::SIGILL => "SIGILL",
            Signal::SIGTRAP => "SIGTRAP",
            Signal::SIGABRT => "SIGABRT",
            Signal::SIGBUS => "SIGBUS",
            Signal::SIGFPE => "SIGFPE",
            Signal::SIGKILL => "SIGKILL",
            Signal::SIGUSR1 => "SIGUSR1",
            Signal::SIGSEGV => "SIGSEGV",
            Signal::SIGUSR2 => "SIGUSR2",
            Signal::SIGPIPE => "SIGPIPE",
            Signal::SIGALRM => "SIGALRM",
            Signal::SIGTERM => "SIGTERM",
            #[cfg(all(
                any(target_os = "android", target_os = "emscripten", target_os = "linux"),
                not(any(target_arch = "mips", target_arch = "mips64", target_arch = "sparc64"))
            ))]
            Signal::SIGSTKFLT => "SIGSTKFLT",
            Signal::SIGCHLD => "SIGCHLD",
            Signal::SIGCONT => "SIGCONT",
            Signal::SIGSTOP => "SIGSTOP",
            Signal::SIGTSTP => "SIGTSTP",
            Signal::SIGTTIN => "SIGTTIN",
            Signal::SIGTTOU => "SIGTTOU",
            Signal::SIGURG => "SIGURG",
            Signal::SIGXCPU => "SIGXCPU",
            Signal::SIGXFSZ => "SIGXFSZ",
            Signal::SIGVTALRM => "SIGVTALRM",
            Signal::SIGPROF => "SIGPROF",
            Signal::SIGWINCH => "SIGWINCH",
            Signal::SIGIO => "SIGIO",
            #[cfg(any(target_os = "android", target_os = "emscripten", target_os = "linux"))]
            Signal::SIGPWR => "SIGPWR",
            Signal::SIGSYS => "SIGSYS",
            #[cfg(not(any(
                target_os = "android",
                target_os = "emscripten",
                target_os = "linux",
                target_os = "redox"
            )))]
            Signal::SIGEMT => "SIGEMT",
            #[cfg(not(any(
                target_os = "android",
                target_os = "emscripten",
                target_os = "linux",
                target_os = "redox"
            )))]
            Signal::SIGINFO => "SIGINFO",
        }
    }
}

impl AsRef<str> for Signal {
    #[no_coverage]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Signal {
    #[no_coverage]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

pub use self::Signal::*;

#[cfg(target_os = "redox")]
const SIGNALS: [Signal; 29] = [
    SIGHUP, SIGINT, SIGQUIT, SIGILL, SIGTRAP, SIGABRT, SIGBUS, SIGFPE, SIGKILL, SIGUSR1, SIGSEGV, SIGUSR2, SIGPIPE,
    SIGALRM, SIGTERM, SIGCHLD, SIGCONT, SIGSTOP, SIGTSTP, SIGTTIN, SIGTTOU, SIGURG, SIGXCPU, SIGXFSZ, SIGVTALRM,
    SIGPROF, SIGWINCH, SIGIO, SIGSYS,
];
#[cfg(all(
    any(target_os = "linux", target_os = "android", target_os = "emscripten"),
    not(any(target_arch = "mips", target_arch = "mips64", target_arch = "sparc64"))
))]
const SIGNALS: [Signal; 31] = [
    SIGHUP, SIGINT, SIGQUIT, SIGILL, SIGTRAP, SIGABRT, SIGBUS, SIGFPE, SIGKILL, SIGUSR1, SIGSEGV, SIGUSR2, SIGPIPE,
    SIGALRM, SIGTERM, SIGSTKFLT, SIGCHLD, SIGCONT, SIGSTOP, SIGTSTP, SIGTTIN, SIGTTOU, SIGURG, SIGXCPU, SIGXFSZ,
    SIGVTALRM, SIGPROF, SIGWINCH, SIGIO, SIGPWR, SIGSYS,
];
#[cfg(all(
    any(target_os = "linux", target_os = "android", target_os = "emscripten"),
    any(target_arch = "mips", target_arch = "mips64", target_arch = "sparc64")
))]
const SIGNALS: [Signal; 30] = [
    SIGHUP, SIGINT, SIGQUIT, SIGILL, SIGTRAP, SIGABRT, SIGBUS, SIGFPE, SIGKILL, SIGUSR1, SIGSEGV, SIGUSR2, SIGPIPE,
    SIGALRM, SIGTERM, SIGCHLD, SIGCONT, SIGSTOP, SIGTSTP, SIGTTIN, SIGTTOU, SIGURG, SIGXCPU, SIGXFSZ, SIGVTALRM,
    SIGPROF, SIGWINCH, SIGIO, SIGPWR, SIGSYS,
];
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "emscripten",
    target_os = "redox"
)))]
const SIGNALS: [Signal; 31] = [
    SIGHUP, SIGINT, SIGQUIT, SIGILL, SIGTRAP, SIGABRT, SIGBUS, SIGFPE, SIGKILL, SIGUSR1, SIGSEGV, SIGUSR2, SIGPIPE,
    SIGALRM, SIGTERM, SIGCHLD, SIGCONT, SIGSTOP, SIGTSTP, SIGTTIN, SIGTTOU, SIGURG, SIGXCPU, SIGXFSZ, SIGVTALRM,
    SIGPROF, SIGWINCH, SIGIO, SIGSYS, SIGEMT, SIGINFO,
];

pub const NSIG: libc::c_int = 32;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SignalIterator {
    next: usize,
}

impl Iterator for SignalIterator {
    type Item = Signal;

    #[no_coverage]
    fn next(&mut self) -> Option<Signal> {
        if self.next < SIGNALS.len() {
            let next_signal = SIGNALS[self.next];
            self.next += 1;
            Some(next_signal)
        } else {
            None
        }
    }
}

// impl Signal {
//     #[no_coverage] pub fn iterator() -> SignalIterator {
//         SignalIterator { next: 0 }
//     }
// }

impl TryFrom<libc::c_int> for Signal {
    type Error = Error;

    #[no_coverage]
    fn try_from(signum: libc::c_int) -> Result<Signal> {
        if 0 < signum && signum < NSIG {
            Ok(unsafe { mem::transmute(signum) })
        } else {
            Err(Error::invalid_argument())
        }
    }
}

#[cfg(not(target_os = "redox"))]
type SaFlags_t = libc::c_int;
#[cfg(target_os = "redox")]
type SaFlags_t = libc::c_ulong;

libc_bitflags! {
    #[allow(dead_code)]
    pub struct SaFlags: SaFlags_t {
        SA_NOCLDSTOP;
        SA_NOCLDWAIT;
        SA_NODEFER;
        SA_ONSTACK;
        SA_RESETHAND;
        SA_RESTART;
        SA_SIGINFO;
    }
}

libc_enum! {
    #[allow(dead_code)]
    #[repr(i32)]
    pub enum SigmaskHow {
        SIG_BLOCK,
        SIG_UNBLOCK,
        SIG_SETMASK,
    }
}

#[derive(Clone, Copy)]
pub struct SigSet {
    sigset: libc::sigset_t,
}

impl SigSet {
    #[no_coverage]
    pub fn empty() -> SigSet {
        let mut sigset = mem::MaybeUninit::uninit();
        let _ = unsafe { libc::sigemptyset(sigset.as_mut_ptr()) };

        unsafe {
            SigSet {
                sigset: sigset.assume_init(),
            }
        }
    }
}

impl AsRef<libc::sigset_t> for SigSet {
    #[no_coverage]
    fn as_ref(&self) -> &libc::sigset_t {
        &self.sigset
    }
}

/// A signal handler.
#[allow(dead_code)]
#[allow(unknown_lints)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SigHandler {
    /// Default signal handling.
    SigDfl,
    /// Request that the signal be ignored.
    SigIgn,
    /// Use the given signal-catching function, which takes in the signal.
    Handler(extern "C" fn(libc::c_int)),
    /// Use the given signal-catching function, which takes in the signal, information about how
    /// the signal was generated, and a pointer to the threads `ucontext_t`.
    #[cfg(not(target_os = "redox"))]
    SigAction(extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void)),
}

/// Action to take on receipt of a signal. Corresponds to `sigaction`.
#[derive(Clone, Copy)]
pub struct SigAction {
    sigaction: libc::sigaction,
}

impl SigAction {
    /// Creates a new action.
    ///
    /// The `SA_SIGINFO` bit in the `flags` argument is ignored (it will be set only if `handler`
    /// is the `SigAction` variant). `mask` specifies other signals to block during execution of
    /// the signal-catching function.
    #[no_coverage]
    pub fn new(handler: SigHandler, flags: SaFlags, mask: SigSet) -> SigAction {
        #[cfg(target_os = "redox")]
        #[no_coverage]
        unsafe fn install_sig(p: *mut libc::sigaction, handler: SigHandler) {
            (*p).sa_handler = match handler {
                SigHandler::SigDfl => libc::SIG_DFL,
                SigHandler::SigIgn => libc::SIG_IGN,
                SigHandler::Handler(f) => f as *const extern "C" fn(libc::c_int) as usize,
            };
        }

        #[cfg(not(target_os = "redox"))]
        #[no_coverage]
        unsafe fn install_sig(p: *mut libc::sigaction, handler: SigHandler) {
            (*p).sa_sigaction = match handler {
                SigHandler::SigDfl => libc::SIG_DFL,
                SigHandler::SigIgn => libc::SIG_IGN,
                SigHandler::Handler(f) => f as *const extern "C" fn(libc::c_int) as usize,
                SigHandler::SigAction(f) => {
                    f as *const extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void) as usize
                }
            };
        }

        let mut s = mem::MaybeUninit::<libc::sigaction>::uninit();
        unsafe {
            let p = s.as_mut_ptr();
            install_sig(p, handler);
            (*p).sa_flags = match handler {
                #[cfg(not(target_os = "redox"))]
                SigHandler::SigAction(_) => (flags | SaFlags::SA_SIGINFO).bits(),
                _ => (flags - SaFlags::SA_SIGINFO).bits(),
            };
            (*p).sa_mask = mask.sigset;

            SigAction {
                sigaction: s.assume_init(),
            }
        }
    }
}

/// Changes the action taken by a process on receipt of a specific signal.
///
/// `signal` can be any signal except `SIGKILL` or `SIGSTOP`. On success, it returns the previous
/// action for the given signal. If `sigaction` fails, no new signal handler is installed.
///
/// # Safety
///
/// Signal handlers may be called at any point during execution, which limits what is safe to do in
/// the body of the signal-catching function. Be certain to only make syscalls that are explicitly
/// marked safe for signal handlers and only share global data using atomics.
#[no_coverage]
pub unsafe fn sigaction(signal: Signal, sigaction: &SigAction) -> Result<SigAction> {
    let mut oldact = mem::MaybeUninit::<libc::sigaction>::uninit();

    let res = libc::sigaction(
        signal as libc::c_int,
        &sigaction.sigaction as *const libc::sigaction,
        oldact.as_mut_ptr(),
    );

    Errno::result(res).map(|_| SigAction {
        sigaction: oldact.assume_init(),
    })
}

#[cfg(target_os = "freebsd")]
pub type type_of_thread_id = libc::lwpid_t;
#[cfg(target_os = "linux")]
pub type type_of_thread_id = libc::pid_t;
