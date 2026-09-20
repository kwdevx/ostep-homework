// Rust port of h8.c -- wires up `ls | wc -l` by hand via pipe() and
// two forked children.
use nix::sys::wait::waitpid;
use nix::unistd::{dup2_stdin, dup2_stdout, execvp, fork, pipe, ForkResult};
use std::ffi::CString;

fn main() {
    let (read_end, write_end) = pipe().expect("pipe failed");

    match unsafe { fork() }.expect("fork failed") {
        ForkResult::Child => {
            // child 1 (writer): its stdout becomes the pipe's write end.
            // Dropping an OwnedFd runs close() for us.
            drop(read_end);
            dup2_stdout(&write_end).expect("dup2 failed");
            drop(write_end); // already dup2'd onto fd 1; drop the spare fd

            let argv = [CString::new("ls").unwrap()];
            execvp(&argv[0], &argv).expect("exec ls failed");
        }
        ForkResult::Parent { child: p1 } => {
            match unsafe { fork() }.expect("fork failed") {
                ForkResult::Child => {
                    // child 2 (reader): its stdin becomes the pipe's read end
                    drop(write_end);
                    dup2_stdin(&read_end).expect("dup2 failed");
                    drop(read_end);

                    let argv = [CString::new("wc").unwrap(), CString::new("-l").unwrap()];
                    execvp(&argv[0], &argv).expect("exec wc failed");
                }
                ForkResult::Parent { child: p2 } => {
                    // parent: MUST drop both ends. If the parent keeps
                    // write_end open, the pipe never reports EOF to
                    // child 2 (the kernel only signals EOF once EVERY
                    // write-end fd, across every process, is closed) --
                    // child 2's `wc -l` would then block forever.
                    drop(read_end);
                    drop(write_end);

                    let _ = waitpid(p1, None);
                    let _ = waitpid(p2, None);
                }
            }
        }
    }
}
