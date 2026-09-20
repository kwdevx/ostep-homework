// Rust port of h3.c -- child prints first, parent never calls wait().
use nix::sys::mman::{mmap_anonymous, MapFlags, ProtFlags};
use nix::unistd::{fork, ForkResult};
use std::num::NonZeroUsize;
use std::process;

fn main() {
    // Shared flag, visible to both parent and child after fork(), so the
    // parent can tell when the child is done without calling wait().
    //
    // mmap_anonymous() args, one by one:
    //   None                     -> let the kernel pick the address
    //   NonZeroUsize size        -> map just enough for one i32 (one
    //                               page, rounded up)
    //   PROT_READ | PROT_WRITE   -> the mapping can be read and written
    //   MAP_SHARED               -> the key part: writes go to the
    //                               actual page, not a private copy,
    //                               and that page is what fork()
    //                               duplicates a *reference* to (not
    //                               the data itself). `nix` adds
    //                               MAP_ANONYMOUS for us since we
    //                               called mmap_anonymous().
    //
    // Because this call happens *before* fork(), the mapping already
    // exists when fork() copies the parent's page table, so both
    // parent and child point at the same physical page. A normal Rust
    // stack/heap value instead would get fork()'s usual copy-on-write
    // treatment: each process would get its own private copy the
    // instant either wrote to it, and neither side would ever see the
    // other's update.
    let child_done = unsafe {
        mmap_anonymous(
            None,
            NonZeroUsize::new(std::mem::size_of::<i32>()).unwrap(),
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MapFlags::MAP_SHARED,
        )
        .expect("mmap failed")
    }
    .as_ptr() as *mut i32;

    // volatile read/write, same reason as `volatile int *` in h3.c:
    // stop the compiler from assuming the loop condition can't change
    // and caching the read across iterations.
    unsafe { child_done.write_volatile(0) };

    let f1 = unsafe { fork() };

    match f1 {
        Err(_) => {
            eprintln!("fork failed");
            process::exit(1);
        }
        Ok(ForkResult::Child) => {
            // child: print first, then signal the parent
            println!("hello");
            unsafe { child_done.write_volatile(1) };
        }
        Ok(ForkResult::Parent { .. }) => {
            // parent: spin-wait on the shared flag instead of calling wait()
            while unsafe { child_done.read_volatile() } == 0 {}
            println!("goodbye");
        }
    }
}
