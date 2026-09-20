// Rust port of h7.c -- close stdout in the child, then try to write.
use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
use nix::sys::wait::wait;
use nix::unistd::{close, fork, write, ForkResult};
use std::io::{self, Write};
use std::os::fd::{AsRawFd, BorrowedFd};

fn main() {
    match unsafe { fork() } {
        Err(_) => {
            eprintln!("fork failed");
            std::process::exit(1);
        }
        Ok(ForkResult::Child) => {
            close(1).expect("close failed");

            // Verified with strace: the real write(1, ...) syscall
            // behind this DOES fail with EBADF -- but println!() has
            // no Result to hand back, and (on this toolchain) does NOT
            // panic on the failure either. The error is just dropped;
            // the program can't tell anything went wrong at all.
            println!("can you see me?");
            eprintln!("child: println!() returned normally (no panic, no error visible)");

            // write!()/io::stdout() DOES return a Result -- so surely
            // THIS surfaces the failure? strace shows its write(1,...)
            // also returns -1 EBADF, yet the Result below still comes
            // back Ok(()). io::Stdout's Write impl swallows it too.
            let res = write!(io::stdout(), "can you see me?\n");
            eprintln!("child: write!(stdout) returned {res:?} (but the real write(2) failed!)");

            // raw syscall-level write, same as h7.c's write(1, ...).
            // SAFETY: fd 1 was just closed above; this borrow is only
            // used to demonstrate the failed write, not to claim any
            // ownership over fd 1.
            let stdout_fd = unsafe { BorrowedFd::borrow_raw(1) };
            let w = write(stdout_fd, b"raw write\n");
            eprintln!("child: nix write(1, ...) returned {w:?}");

            // fd reuse gotcha: fd 1 is free now, so the next open()
            // grabs it -- same as h7.c.
            let fd = open(
                "./h7.output",
                OFlag::O_CREAT | OFlag::O_WRONLY | OFlag::O_TRUNC,
                Mode::from_bits_truncate(0o644),
            )
            .unwrap();
            eprintln!(
                "child: new open() got fd {} (reused old stdout slot)",
                fd.as_raw_fd()
            );
            write(&fd, b"surprise -- this landed in h7.output\n").unwrap();
        }
        Ok(ForkResult::Parent { .. }) => {
            let _ = wait();
        }
    }
}
