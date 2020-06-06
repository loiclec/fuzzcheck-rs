
use libc::pid_t;

use std::fmt;
/// Process identifier
///
/// Newtype pattern around `pid_t` (which is just alias). It prevents bugs caused by accidentally
/// passing wrong value.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Pid(pid_t);

impl Pid {
    /// Creates `Pid` from raw `pid_t`.
    pub fn from_raw(pid: pid_t) -> Self {
        Pid(pid)
    }

    /// Returns PID of calling process
    pub fn this() -> Self {
        getpid()
    }

    /// Returns PID of parent of calling process
    pub fn parent() -> Self {
        getppid()
    }

    /// Get the raw `pid_t` wrapped by `self`.
    pub fn as_raw(self) -> pid_t {
        self.0
    }
}

impl From<Pid> for pid_t {
    fn from(pid: Pid) -> Self {
        pid.0
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}


/// Get the pid of this process (see
/// [getpid(2)](http://pubs.opengroup.org/onlinepubs/9699919799/functions/getpid.html)).
///
/// Since you are running code, there is always a pid to return, so there
/// is no error case that needs to be handled.
#[inline]
pub fn getpid() -> Pid {
    Pid(unsafe { libc::getpid() })
}

/// Get the pid of this processes' parent (see
/// [getpid(2)](http://pubs.opengroup.org/onlinepubs/9699919799/functions/getppid.html)).
///
/// There is always a parent pid to return, so there is no error case that needs
/// to be handled.
#[inline]
pub fn getppid() -> Pid {
    Pid(unsafe { libc::getppid() }) // no error handling, according to man page: "These functions are always successful."
}
