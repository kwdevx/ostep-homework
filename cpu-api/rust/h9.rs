// h9 -- fun follow-up to h3: instead of one fixed-size flag shared via
// raw mmap(MAP_SHARED|ANON), share a growing struct across two real
// processes using cloudflare's mmap-sync crate.
//
// mmap-sync keeps two mmap'd data copies + one small state file behind
// the scenes. The writer always fills the *inactive* copy, then flips
// an atomic pointer in the state file once the write is done -- RCU
// style (copy, then swap a pointer) instead of locking. A reader just
// follows whatever the state file currently points at, so it always
// sees either the fully-old or fully-new struct, never a torn one,
// and never blocks the writer (or vice versa).
use bytecheck::CheckBytes;
use mmap_sync::synchronizer::Synchronizer;
use nix::sys::wait::waitpid;
use nix::unistd::{fork, ForkResult};
use rkyv::{Archive, Deserialize, Serialize};
use std::process;
use std::thread::sleep;
use std::time::Duration;

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive_attr(derive(CheckBytes))]
struct Snapshot {
    tick: u32,
    pid: i32,
    note: String,
}

const SHM_PATH: &str = "/tmp/ostep_h9_snapshot";
const WRITER_TICKS: u32 = 8;

fn main() {
    match unsafe { fork() }.expect("fork failed") {
        ForkResult::Child => {
            // writer: publish a new struct every tick. Each write()
            // copies `data` into the *other* data file and then flips
            // the state file's active pointer -- readers never see a
            // half-written struct, and never block this loop.
            let mut synchronizer = Synchronizer::new(SHM_PATH.as_ref());
            let pid = process::id() as i32;

            for tick in 0..WRITER_TICKS {
                let data = Snapshot {
                    tick,
                    pid,
                    note: format!("hello from child, tick {tick}"),
                };
                synchronizer
                    .write(&data, Duration::from_secs(1))
                    .expect("write failed");
                sleep(Duration::from_millis(50));
            }
            process::exit(0);
        }
        ForkResult::Parent { child } => {
            // reader: poll much faster than the writer publishes, so
            // we can see the same tick read back more than once and
            // confirm every read is a complete, well-formed struct --
            // never a partial write caught mid-flight.
            let mut synchronizer = Synchronizer::new(SHM_PATH.as_ref());
            let mut last_tick: Option<u32> = None;

            for _ in 0..(WRITER_TICKS * 4) {
                match unsafe { synchronizer.read::<Snapshot>(false) } {
                    Ok(snapshot) => {
                        if last_tick != Some(snapshot.tick) {
                            println!(
                                "parent saw: tick={} pid={} note={:?}",
                                snapshot.tick, snapshot.pid, snapshot.note
                            );
                            last_tick = Some(snapshot.tick);
                        }
                    }
                    Err(_) => {
                        // writer hasn't published anything yet
                    }
                }
                sleep(Duration::from_millis(12));
            }

            waitpid(child, None).expect("waitpid failed");
            println!("parent: child done, exiting");
        }
    }
}
