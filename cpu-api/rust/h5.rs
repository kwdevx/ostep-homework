// Rust port of h5.c -- wait() in parent vs in child.
use nix::sys::wait::{wait, WaitStatus};
use nix::unistd::{fork, getpid, ForkResult};

fn main() {
    match unsafe { fork() } {
        Err(_) => {
            eprintln!("fork failed");
            std::process::exit(1);
        }
        Ok(ForkResult::Child) => {
            // child: it has no children of its own, so wait() here has
            // nothing to wait for -- demonstrates the "wait() in the
            // child" question.
            println!("child (pid:{})", getpid());
            match wait() {
                Ok(status) => println!("child: wait() returned {status:?}"),
                Err(e) => println!("child: wait() returned Err({e})"),
            }
        }
        Ok(ForkResult::Parent { child }) => {
            println!("parent (pid:{}) forked child (pid:{child})", getpid());
            match wait() {
                // blocks here until the child exits
                Ok(WaitStatus::Exited(pid, code)) => {
                    println!("parent: wait() returned pid {pid}, child exit status {code}")
                }
                Ok(status) => println!("parent: wait() returned {status:?}"),
                Err(e) => println!("parent: wait() returned Err({e})"),
            }
        }
    }
}
