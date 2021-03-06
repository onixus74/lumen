// wasm32 proptest cannot be compiled at the same time as non-wasm32 proptest, so disable tests that
// use proptest completely for wasm32
//
// See https://github.com/rust-lang/cargo/issues/4866
#[cfg(all(not(target_arch = "wasm32"), test))]
mod test;

use liblumen_alloc::erts::exception;
use liblumen_alloc::erts::process::Process;
use liblumen_alloc::erts::term::prelude::*;
use lumen_runtime_macros::native_implemented_function;

use crate::otp;

/// Returns a binary that is made from the integers and binaries given in iolist
#[native_implemented_function(iolist_to_binary/1)]
pub fn native(process: &Process, iolist_or_binary: Term) -> exception::Result<Term> {
    otp::erlang::list_to_binary_1::native(
        process,
        if iolist_or_binary.is_binary() {
            process.list_from_slice(&[iolist_or_binary]).unwrap()
        } else {
            iolist_or_binary
        },
    )
}
