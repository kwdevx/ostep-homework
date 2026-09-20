// Rust port of h1.c, using the `nix` crate for a safe fork() wrapper
// instead of hand-written `extern "C"` FFI.
use nix::unistd::{fork, ForkResult};
use std::process;

fn main() {
    let mut x = 0;
    println!("before fork (x:{x})");

    // fork() here returns a Result<ForkResult, Errno> instead of a bare
    // pid_t, so "did it fail" and "which process am I" are both
    // expressed in the type instead of by comparing an int to -1/0.
    match unsafe { fork() } {
        Err(_) => {
            eprintln!("fork failed");
            process::exit(1);
        }
        Ok(ForkResult::Child) => {
            x += 1;
            println!("after fork child (x:{x})");
        }
        Ok(ForkResult::Parent { child: _ }) => {
            x += 1;
            println!("after fork parent (x:{x})");
        }
    }
}
