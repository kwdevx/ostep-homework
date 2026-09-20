# cpu-api — condensed notes

## h1 — fork() basics
- `fork()` gives parent and child **separate copies** of memory (copy-on-write). A variable incremented after `fork()` diverges independently in each — no shared state.
- Which one (parent/child) runs first / prints first is **not guaranteed** — scheduler's call.
- Buffering gotcha: piped (non-tty) C stdio is fully buffered. If a `printf` happens *before* `fork()` and hasn't been flushed yet, both parent and child inherit that unflushed buffer and each flush it on exit → the pre-fork line gets printed **twice**. Rust's `println!` doesn't buffer this way, so it only prints once.

## h2 — shared file offset (two "OOPS")
- `open()` before `fork()` ⇒ parent and child share the same **open file description**, not just the same fd number — this includes one shared file offset.
- Each `write()` is atomic w.r.t. that offset: kernel locks it, writes, advances it, unlocks. So the second `write()` (whichever process gets there second) always lands *after* the first — never overwrites it.
- The "lock" isn't a dedicated queue for this file; it's the general kernel mutex/wait-queue mechanism (blocked callers sleep on a `wait_queue_head_t` until woken).
- Contrast: if each process had called its own independent `open()` instead of sharing one via `fork()`, they'd each get their own offset starting at 0 — real overwrite risk.

## h3 — child-prints-first without wait()
- Normal memory (heap/stack/global) is copy-on-write after `fork()` — writes in the child are invisible to the parent. So a plain flag variable can't be used to signal across processes.
- Fix: `mmap(MAP_SHARED | MAP_ANONYMOUS)` **before** `fork()` — creates a mapping backed by one physical page; `fork()` copies the *page table entry*, not the data, so both processes' pointers resolve to the same page. `MAP_SHARED` (not `MAP_PRIVATE`) is what makes writes visible across processes instead of triggering copy-on-write.
- Parent spin-waits (`while (*flag == 0);`) instead of calling `wait()` — busy-loops, but guarantees child-first ordering.
- Need `volatile` (C) / `read_volatile`/`write_volatile` (Rust) on the flag, or the compiler may cache the read and loop forever/never observe the update.

## h4 — fork() + exec() variants
- All `exec*()` variants are convenience wrappers around one syscall (`execve`), differing along 3 independent axes: **args as list vs vector** (`l` vs `v`), **full path vs `$PATH` search** (plain vs `p`), **inherit vs replace environment** (plain vs `e`). C exposes all 8 combinations (of which 6 exist as named functions); pick whichever matches what you already have on hand.
- Suitable use case per axis choice:

  | Axis | Choose... | ...when |
  |---|---|---|
  | args | `l` (list) | arg count is fixed/known at compile time (`execl(path, "ls", "-l", NULL)`) |
  | args | `v` (vector) | args are built at runtime (parsed input, loop, variable count) |
  | path | plain | you already have/require a specific full path |
  | path | `p` (PATH search) | you want shell-like behavior — run by name, let `$PATH` resolve it |
  | env | plain | child should inherit the caller's environment as-is |
  | env | `e` (explicit) | you need to control/sanitize the child's env (e.g. drop secrets before running an untrusted command, reproducible env for tests) |

- Buffering strikes again: `printf` before `fork()` is unflushed; `exec()` wipes the child's inherited stdio buffer entirely (fresh process image), so only the *parent's* buffer survives — all of it dumped together at final exit. Fix: `fflush(stdout)` before `fork()`. Rust's `println!` avoids this — it flushes per line even when piped, so `h4.rs` didn't need the equivalent fix.
- Rust/`nix` only has the `v`-style calls (`execv`, `execve`, `execvp`, `execvpe`) — no `l` equivalent, because that axis exists in C purely due to its variadic-argument ABI, which Rust can't call into generically. Building a `&[CString]` array is no less convenient than listing args, so the axis just doesn't exist in Rust.

## h5 — wait()
- `wait(&status)` **blocks** until any one of the caller's children exits, then returns that child's pid. Called in a loop, it's how a parent reaps multiple children one at a time.
- `wait()` only waits for your *own* children — never siblings, never the parent. Called in the **child** (which has none of its own), it fails immediately: returns `-1` in C / `Err(Errno::ECHILD)` in Rust ("No child processes").
- Exit status only gets **8 bits**: `exit(n)` is truncated to `n & 0xFF` before `wait()`/`WEXITSTATUS()` ever sees it. `exit(1)` → status `1`. `exit(-1)` → the byte pattern of `-1` is `0xFF` → status **255**. This is why "255" shows up so often for negative/out-of-range exit codes — it's truncation, not a special meaning.
- Rust's `nix::sys::wait::wait()` returns a typed `WaitStatus` enum (`Exited(pid, code)`, `Signaled(...)`, ...) instead of a raw int + bitmask macros (`WEXITSTATUS`, `WIFEXITED`, ...) — same underlying info, no manual bit-twiddling.

## h6 — waitpid()
- `waitpid(pid, &status, options)` generalizes `wait()` two ways `wait()` can't: (1) target a **specific** child pid instead of reaping "whichever exits first" (matters once you have multiple children and need to reap them in a known relationship, not just exit order); (2) `WNOHANG` makes it **non-blocking** — returns immediately (`0` in C / `WaitStatus::StillAlive` in Rust) if that child hasn't exited yet, instead of freezing the caller.
- Useful whenever the parent has other work to do while a child runs — the canonical example is a shell polling a background job (`cmd &`) instead of hanging the prompt.
- Same buffering gotcha as h1/h4 shows up again: with a 2-second-sleeping child and a polling loop in between, piped/non-tty output can reorder (child's exit-triggered flush landing before the still-running parent's buffered lines) unless the parent `fflush(stdout)`s after each `printf`. Rust's `println!` needed no such fix, same as before.
- `waitpid()`/`WNOHANG` only reports state **transitions** (running→stopped→continued→exited), never "what is my still-running child doing right now." No syscall answers that directly — peeking live scheduler state means reading `/proc/<pid>/status` (state letters: `R` running, `S` interruptible sleep — what `sleep()` shows as, `D` uninterruptible/disk I/O — can't even be `SIGKILL`ed until it completes, `Z` zombie — exited but not yet `wait()`ed, `T` stopped).
- `waitid(pid, &info, WEXITED | WNOWAIT)` is the actual syscall-level way to peek *exit* status without consuming it (unlike `waitpid()`, which reaps on success) — different axis than the live-state peek above.

### `/proc` is a virtual filesystem, not stored data
- The kernel doesn't write process state to a file under `/proc`. Every process already has a `struct task_struct` living in kernel memory (created by `fork()`, updated continuously by the scheduler) — that's the one and only authoritative copy.
- `/proc` (`procfs`) is a kernel module that intercepts `open()`/`read()`/`close()` for paths under `/proc` and, instead of reading disk blocks, runs code that formats live `task_struct` fields as text **at the moment you call `read()`**. Nothing is cached or written ahead of time.
- Consequence: reading `/proc/<pid>/status` in a loop (as `h6`'s `peek_state()` does) always gets a fresh, current answer — there's no "stale copy" to worry about. The only race is time-of-check-to-time-of-use: the process can change state (or fully disappear) between your `read()` and whatever you do next, same as any live system query.
- This is also why the same mechanism scales to everything else under `/proc` — `/proc/<pid>/fd`, `/proc/<pid>/maps`, `/proc/meminfo`, etc. are all synthesized on read from live kernel data structures, not real files on any disk.

## h7 — closing STDOUT_FILENO then printf()
- `printf()` after `close(STDOUT_FILENO)` doesn't fail, and looks like it worked: it returns the byte count as normal, because it only appends to `stdio`'s userspace buffer — it doesn't call `write(2)` itself. The closed fd isn't touched until the buffer is actually flushed.
- Flushing is where it bites: `fflush(stdout)` (or a buffer filling up, or process exit) triggers the real `write(fd 1, ...)` — *that* fails with `EBADF`, but silently, since almost nothing checks `fflush`'s or `exit`'s return value. Net effect: the output just vanishes with no visible error.
- A raw `write(STDOUT_FILENO, ...)` (bypassing stdio's buffer entirely) fails immediately with `EBADF` — no buffer to hide behind.
- Bigger gotcha: closing fd 1 frees that slot, so the kernel hands it to the **next `open()`** (lowest-available-fd rule). Any code that still writes to fd 1 directly — including library code you don't control — ends up silently writing into that unrelated new file instead of erroring or reaching a terminal. Moral: never just `close()` a standard fd; redirect it (e.g. `dup2` onto `/dev/null` or a real file) so the slot doesn't go up for grabs.
- `stdout` (fd 1) and `stderr` (fd 2) are fully independent fd-table entries — closing/breaking one never touches the other, even though both point at the same terminal by default. That's why diagnostic `fprintf(stderr, ...)`/`eprintln!()` lines kept appearing throughout this whole exercise even after fd 1 was closed.
- Rust surprise, confirmed with `strace` (not assumed): after closing fd 1, both `println!("...")` and `write!(io::stdout(), "...")` have their real underlying `write(1, ...)` syscall fail with `EBADF` — yet neither one surfaces it. `println!` doesn't panic (older belief; not what this toolchain actually does) and just drops the error silently; `write!(io::stdout(), ...)` returns `Ok(())` even though its own `write(2)` call shows `-1 EBADF` in the trace. Only bypassing `io::Stdout` entirely (a raw `nix`/`libc` `write()` on the fd) surfaces the true `Err(EBADF)`. Net effect: Rust's stdio abstraction hides this failure even more thoroughly than C's — in C at least `fflush()`'s return value tells the truth if you check it.

## h8 — pipe() connecting two children
- `pipe(fds)` gives one fd pair: `fds[0]` read end, `fds[1]` write end — both inherited by every child forked afterward, since `pipe()` must run **before** either `fork()`.
- Wiring `ls | wc -l` by hand: child 1 gets `dup2(write_end, STDOUT_FILENO)`, child 2 gets `dup2(read_end, STDIN_FILENO)`, then each closes both its original pipe fds (the dup'd standard fd is enough) before `exec`.
- Critical, easy-to-miss step: the **parent** must close both pipe ends too. A pipe only reports EOF to the reader once *every* fd referencing its write end (in every process) is closed. If the parent keeps holding `write_end` open, `wc -l` blocks on `read()` forever even after `ls` has exited — the classic "pipeline hangs" bug.
- Rust's `OwnedFd` makes the close-discipline automatic: `drop(fd)` closes it, and the borrow checker complains if you try to use an fd after dropping it — harder to accidentally leak the wrong end open than in C, where a forgotten `close()` compiles fine.
- `nix::unistd::dup2_stdout()`/`dup2_stdin()` are purpose-built for this (redirect *a* fd onto the well-known stdout/stdin slot) — cleaner than the generic `dup2()`, which in this nix version wants ownership of the *target* fd (`&mut OwnedFd`) rather than a bare fd number.

## C → Rust port
- Hand-written `extern "C"` blocks (raw `libc` signatures) work but give zero ergonomics: bare ints for pid/fd, manual errno checks, magic numbers for flags (`O_CREAT`, `PROT_READ`, ...).
- `nix` crate wraps the same syscalls safely: `fork() -> Result<ForkResult, Errno>`, `open()/write()` with typed `OFlag`/`Mode`, `mmap_anonymous()` with typed `ProtFlags`/`MapFlags` — same syscalls, real error handling, no manual constant-guessing.
- `libc`/`nix`/`rustix` are on a spectrum: `libc` = raw unsafe bindings, `nix`/`rustix` = safe wrappers on top (pick `nix` for broad POSIX coverage, `rustix` if avoiding a `libc` dependency matters).
