// Rust port of h6.c -- waitpid() with WNOHANG instead of wait().
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{fork, getpid, ForkResult};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    match unsafe { fork() } {
        Err(_) => {
            eprintln!("fork failed");
            std::process::exit(1);
        }
        Ok(ForkResult::Child) => {
            sleep(Duration::from_secs(2)); // give the parent something to poll for
            println!("child (pid:{})", getpid());
        }
        Ok(ForkResult::Parent { child }) => {
            println!("parent (pid:{}) forked child (pid:{child})", getpid());

            // WNOHANG is the thing a plain wait() simply cannot do: check
            // on a specific child WITHOUT blocking. Useful whenever the
            // parent has other work to do while waiting -- e.g. a shell
            // polling a background job instead of freezing the prompt.
            let mut polls = 0;
            loop {
                match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
                    Ok(WaitStatus::StillAlive) => {
                        polls += 1;
                        println!("parent: child not done yet, doing other work (poll {polls})...");
                        sleep(Duration::from_millis(500));
                    }
                    Ok(WaitStatus::Exited(pid, code)) => {
                        println!(
                            "parent: waitpid() returned pid {pid} after {polls} polls, child exit status {code}"
                        );
                        break;
                    }
                    Ok(status) => {
                        println!("parent: waitpid() returned {status:?} after {polls} polls");
                        break;
                    }
                    Err(e) => {
                        eprintln!("waitpid failed: {e}");
                        break;
                    }
                }
            }
        }
    }
}
