// Rust port of h2.c -- same "why two OOPS" exercise, using `nix`'s
// safe wrappers instead of hand-written `extern "C"` FFI.
use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
use nix::unistd::{fork, getpid, write, ForkResult};
use std::os::fd::AsRawFd;
use std::process;

fn main() {
    let fid = open(
        "./h2.output",
        OFlag::O_CREAT | OFlag::O_WRONLY | OFlag::O_TRUNC,
        Mode::S_IRWXU,
    )
    .expect("open failed");

    let f1 = unsafe { fork() };

    println!("opened (fd:{}), (pid:{})", fid.as_raw_fd(), getpid());

    let msg = b"OOPS\n";

    //
    // Output: h2.output will contain two "OOPS" lines (never one
    // overwriting the other).
    //
    // Why: open() happens before fork(), so parent and child share
    // the same kernel file description, including its file offset.
    // The kernel locks that offset for each write(), so:
    //   1. first write()  -> lands at offset 0, offset moves to 5
    //   2. second write() -> lands at offset 5, offset moves to 10
    // Whichever process writes second just appends after the first.
    // (Order of parent vs child is not guaranteed.)
    //
    match f1 {
        Err(_) => {
            eprintln!("fork failed");
            process::exit(1);
        }
        Ok(ForkResult::Child) => {
            write(&fid, msg).expect("write failed");
        }
        Ok(ForkResult::Parent { .. }) => {
            // parent path in here
            write(&fid, msg).expect("write failed");
        }
    }
}
