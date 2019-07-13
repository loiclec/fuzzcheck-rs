
pub type c_int = i32;
pub type pid_t = i32;
pub type uid_t = u32;
pub type size_t = usize;

pub type sighandler_t = size_t;
pub type sigset_t = u32;

// Use repr(u8) as LLVM expects `void*` to be the same as `i8*` to help
// enable more optimization opportunities around it recognizing things
// like malloc/free.
#[repr(u8)]
#[allow(missing_copy_implementations)]
#[allow(missing_debug_implementations)]
pub enum c_void {
    // Two dummy variants so the #[repr] attribute can be used.
    #[doc(hidden)]
    __variant1,
    #[doc(hidden)]
    __variant2,
}

#[allow(unused_macros)]
macro_rules! __item {
    ($i:item) => {
        $i
    };
}

#[allow(unused_macros)]
macro_rules! s {
    ($($(#[$attr:meta])* pub $t:ident $i:ident { $($field:tt)* })*) => ($(
        s!(it: $(#[$attr])* pub $t $i { $($field)* });
    )*);
    (it: $(#[$attr:meta])* pub union $i:ident { $($field:tt)* }) => (
        compile_error!("unions cannot derive extra traits, use s_no_extra_traits instead");
    );
    (it: $(#[$attr:meta])* pub struct $i:ident { $($field:tt)* }) => (
        __item! {
            #[repr(C)]
            #[cfg_attr(feature = "extra_traits", derive(Debug, Eq, Hash, PartialEq))]
            #[allow(deprecated)]
            $(#[$attr])*
            pub struct $i { $($field)* }
        }
        #[allow(deprecated)]
        impl Copy for $i {}
        #[allow(deprecated)]
        impl Clone for $i {
            fn clone(&self) -> $i { *self }
        }
    );
}

s! {
    pub struct sigaction {
        // FIXME: this field is actually a union
        pub sa_sigaction: sighandler_t,
        pub sa_mask: sigset_t,
        pub sa_flags: c_int,
    }
    pub struct siginfo_t {
        pub si_signo: c_int,
        pub si_errno: c_int,
        pub si_code: c_int,
        pub si_pid: pid_t,
        pub si_uid: uid_t,
        pub si_status: c_int,
        pub si_addr: *mut c_void,
        //Requires it to be union for tests
        //pub si_value: ::sigval,
        _pad: [usize; 9],
    }
}

extern {
    pub fn sigaction(signum: c_int,
                     act: *const sigaction,
                     oldact: *mut sigaction) -> c_int;
}