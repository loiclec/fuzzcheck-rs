#import <stdint.h>

/// Returns the return address of the current function
void* return_address() {
    // This function returns the return address of the current function, or of
    // one of its callers. The level argument is number of frames to scan up
    // the call stack. A value of 0 yields the return address of the current
    // function, a value of 1 yields the return address of the caller of the
    // current function, and so forth. When inlining the expected behavior is
    // that the function returns the address of the function that is returned
    // to. To work around this behavior use the noinline function attribute.
    //
    // The level argument must be a constant integer.
    // On some machines it may be impossible to determine the return address of
    //  any function other than the current one; in such cases, or when the top
    // of the stack has been  reached, this function returns 0 or a random value. 
    // In addition, __builtin_frame_address may be used to determine if the top 
    // of the stack has been reached.
    //
    // Additional post-processing of the returned value may be needed, 
    // see __builtin_extract_return_addr. 
    //
    // Calling this function with a nonzero argument can have unpredictable 
    // effects, including crashing the calling program. As a result, calls that
    // are considered unsafe are diagnosed when the -Wframe-address option is
    // in effect. Such calls should only be made in debugging situations. 
    //
    // source: https://gcc.gnu.org/onlinedocs/gcc/Return-Address.html

    // Notice that the function is called with 1 as argument. Because of that, 
    // we have to ensure that the frame pointers are present, which is done
    // by passing -Cforce-frame-pointers=yes to `rustc`. 
	return __builtin_return_address(1);
}
