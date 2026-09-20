// Rust port of h4.c -- fork() + exec*() variants running /bin/ls.
//
// nix/Rust only exposes the "v" (argument-vector) family: execv,
// execve, execvp, execvpe. There's no execl/execle/execlp here --
// those exist in C only because C's variadic-argument ABI makes
// listing args individually convenient. Rust has no safe way to call
// a C variadic function generically, and doesn't need one: building
// a `&[CString]` array is exactly as easy as listing args would be.
// So the "list vs vector" axis C has collapses to just one option.
use nix::sys::wait::waitpid;
use nix::unistd::{execv, execve, execvp, execvpe, fork, ForkResult};
use std::convert::Infallible;
use std::ffi::CString;
use std::process;

fn run(label: &str, body: fn() -> nix::Result<Infallible>) {
    println!("=== {label} ===");
    match unsafe { fork() } {
        Err(_) => {
            eprintln!("fork failed");
            process::exit(1);
        }
        Ok(ForkResult::Child) => {
            let err = body().unwrap_err(); // only reached on failure
            eprintln!("{label}: exec failed: {err}");
            process::exit(1);
        }
        Ok(ForkResult::Parent { child }) => {
            let _ = waitpid(child, None);
        }
    }
}

fn do_execv() -> nix::Result<Infallible> {
    // full path, argument vector, inherits environment
    let path = CString::new("/bin/ls").unwrap();
    let argv = [CString::new("ls").unwrap(), CString::new("-l").unwrap()];
    execv(&path, &argv)
}

fn do_execve() -> nix::Result<Infallible> {
    // full path, argument vector, explicit environment (replaces caller's)
    let path = CString::new("/bin/ls").unwrap();
    let argv = [CString::new("ls").unwrap(), CString::new("-l").unwrap()];
    let envp = [CString::new("PATH=/bin").unwrap()];
    execve(&path, &argv, &envp)
}

fn do_execvp() -> nix::Result<Infallible> {
    // PATH search (bare name), argument vector, inherits environment
    let file = CString::new("ls").unwrap();
    let argv = [CString::new("ls").unwrap(), CString::new("-l").unwrap()];
    execvp(&file, &argv)
}

fn do_execvpe() -> nix::Result<Infallible> {
    // PATH search, argument vector, explicit environment
    let file = CString::new("ls").unwrap();
    let argv = [CString::new("ls").unwrap(), CString::new("-l").unwrap()];
    let envp = [CString::new("PATH=/bin").unwrap()];
    execvpe(&file, &argv, &envp)
}

fn main() {
    run("execv", do_execv);
    run("execve", do_execve);
    run("execvp", do_execvp);
    run("execvpe", do_execvpe);
}
