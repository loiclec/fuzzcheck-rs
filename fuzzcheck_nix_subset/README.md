# fuzzcheck_nix_subset

This crate is a tiny subset of the [nix] crate, used by [fuzzcheck].

I only needed a way to implement signal handlers, and I wanted a nicer API than
[libc] to do so, but I didn't want to pay the compile-time cost of the full
nix crate.

[nix]: https://crates.io/crates/nix
[fuzzcheck]: https://crates.io/crates/fuzzcheck
[libc]: https://crates.io/crates/libc