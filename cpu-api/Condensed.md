# cpu-api — condensed notes

| # | Topic | One-liner |
|---|---|---|
| [h1](#h1--fork) | `fork()` basics | Copy-on-write memory; C buffers duplicate a pre-fork line, Rust doesn't |
| [h2](#h2--shared-file-offset-two-oops) | Shared file offset | `open()` before `fork()` ⇒ shared offset ⇒ writes append, never collide |
| [h3](#h3--child-first-without-wait) | Child-first without `wait()` | `mmap(MAP_SHARED\|ANON)` before `fork()` = only way to share a flag |
| [h4](#h4--exec-variants) | `exec()` variants | 3 independent axes (list/vector, path/`$PATH`, inherit/explicit env) |
| [h5](#h5--wait) | `wait()` | Blocks for *any* child; `ECHILD` if called with none; exit status is 8 bits |
| [h6](#h6--waitpid) | `waitpid()` | Target a specific pid + non-blocking `WNOHANG`; live peek needs `/proc` |
| [h7](#h7--closing-stdout_fileno-then-printf) | Closed stdout + `printf()` | Buffered writes fail *silently*; freed fd gets reused by the next `open()` |
| [h8](#h8--pipe-connecting-two-children) | `pipe()` between two children | Parent must close **both** ends or the reader blocks forever |

---

## h1 — fork()

| Fact | Detail |
|---|---|
| Memory model | Copy-on-write: parent/child get separate copies, diverge independently after `fork()` |
| Execution order | Not guaranteed — scheduler's call |
| Buffering (C) | Piped stdio is fully buffered. A pre-`fork()` `printf` not yet flushed gets inherited by both processes → each flushes its own copy at exit → line prints **twice** |
| Buffering (Rust) | `println!` flushes per line even off-tty → prints **once** |

---

## h2 — shared file offset (two "OOPS")

```
open()  →  ONE open file description (incl. offset)
fork()  →  parent + child both reference it
write() →  kernel locks offset, writes, advances, unlocks
```

| Fact | Detail |
|---|---|
| Why shared | `open()` **before** `fork()` ⇒ shared open file description, not just the same fd number |
| Why no overwrite | Each `write()` is atomic w.r.t. the offset — second writer always lands *after* the first |
| The "lock" | Generic kernel mutex/wait-queue, not a dedicated per-file queue |
| Contrast | Independent `open()` per process ⇒ each gets offset 0 ⇒ real overwrite risk |

---

## h3 — child-first without wait()

| Approach | Works? | Why |
|---|---|---|
| Plain global/heap flag | ❌ | Copy-on-write — child's write is invisible to parent |
| `mmap(MAP_PRIVATE\|ANON)` | ❌ | Still copy-on-write |
| `mmap(MAP_SHARED\|ANON)` **before** `fork()` | ✅ | One physical page; `fork()` copies the *page-table entry*, not the data |

- Parent **spin-waits** (`while (*flag == 0);`) instead of `wait()` — busy-loops but guarantees ordering.
- Must mark the flag `volatile` (C) / use `read_volatile`/`write_volatile` (Rust), or the compiler may cache the read and never observe the update.

---

## h4 — exec() variants

All are wrappers around one syscall (`execve`), differing on 3 independent axes:

| | `l` list | `v` vector |
|---|---|---|
| **plain path** | `execl` | `execv` |
| **`$PATH` search (`p`)** | `execlp` | `execvp` |
| **explicit env (`e`)** | `execle` | *(execvpe = v + p + e)* |

| Axis | Pick this | ...when |
|---|---|---|
| args | `l` | arg count fixed at compile time |
| args | `v` | args built at runtime |
| path | plain | you already have a full path |
| path | `p` | shell-like — resolve by name via `$PATH` |
| env | plain | child should inherit caller's env |
| env | `e` | need to control/sanitize child's env (drop secrets, reproducible tests) |

> **Rust/`nix` only has the `v` row.** The `l`/`v` axis exists in C purely because of its variadic ABI — Rust can't call into that generically, and doesn't need to (`&[CString]` is no less convenient).

> **Buffering strikes again:** `exec()` wipes the child's *inherited* stdio buffer entirely (fresh process image) — only the parent's buffer survives, dumped all at once at exit. Fix: `fflush(stdout)` before `fork()`. Rust needed no fix (flushes per line).

---

## h5 — wait()

| Caller | Result |
|---|---|
| Parent (has a child) | **Blocks**, returns that child's pid + status |
| Child (has no children) | Fails immediately — `-1` (C) / `Err(ECHILD)` (Rust) — "No child processes" |

**Exit status is only 8 bits** — `exit(n)` truncates to `n & 0xFF`:

| Call | `WEXITSTATUS` |
|---|---|
| `exit(1)` | `1` |
| `exit(255)` | `255` |
| `exit(-1)` | `255` ← byte pattern of `-1` is `0xFF` |
| `exit(256)` | `0` ← wraps around |

Rust's `nix::sys::wait::wait()` returns a typed `WaitStatus` enum (`Exited`, `Signaled`, ...) instead of raw int + `WIFEXITED`/`WEXITSTATUS` macros.

---

## h6 — waitpid()

| | `wait()` | `waitpid()` |
|---|---|---|
| Which child | Whichever exits first | A **specific** pid |
| Blocking | Always | Optional — `WNOHANG` returns immediately |

Useful whenever the parent has other work to do — e.g. a shell polling a background job instead of freezing the prompt.

**Peeking a still-alive child** — `waitpid`/`WNOHANG` only reports *transitions* (running→stopped→continued→exited), never live state:

| Need | Use |
|---|---|
| Did it exit / how | `waitpid(..., WNOHANG)` |
| Exit status, without consuming it | `waitid(pid, &info, WEXITED\|WNOWAIT)` |
| Live scheduler state (still running) | Read `/proc/<pid>/status` — no syscall for this exists |

`/proc/<pid>/status` state letters (same as `ps`/`top`'s `STAT`):

| Letter | Meaning |
|---|---|
| `R` | Running / runnable |
| `S` | Interruptible sleep — what `sleep()` shows as |
| `D` | Uninterruptible sleep (disk I/O) — can't even be `SIGKILL`ed until it completes |
| `Z` | Zombie — exited, not yet `wait()`ed |
| `T` | Stopped (`SIGSTOP` / traced) |

> **`/proc` is a virtual filesystem, not stored data.** The kernel's only real copy of process state is the in-memory `struct task_struct`. `procfs` intercepts `open()`/`read()` under `/proc` and formats live kernel fields as text **at read time** — nothing is written ahead of time or cached. So polling `/proc/<pid>/status` in a loop always gets a fresh answer; the only race is between your `read()` and whatever you do next (the process can change/vanish in between). Same mechanism powers `/proc/<pid>/fd`, `/proc/<pid>/maps`, `/proc/meminfo`, etc.

---

## h7 — closing STDOUT_FILENO then printf()

| Call (after `close(1)`) | Looks like | Actually happened |
|---|---|---|
| `printf("...")` | Returns byte count, "succeeds" | Just buffered — no syscall yet |
| `fflush(stdout)` | Returns `-1`, `errno=EBADF` | Real `write(1,...)` finally attempted, fails |
| `write(1, ...)` raw | `-1`, `EBADF` immediately | No buffer to hide behind |
| Rust `println!("...")` | Returns normally, no panic | `strace`: real `write(1,...)` fails `EBADF`, error dropped silently |
| Rust `write!(io::stdout(),...)` | Returns `Ok(())` | `strace`: same failed `write(1,...)`, error **not** surfaced |
| Rust `nix::write()` raw | `Err(EBADF)` | Bypasses `io::Stdout` — only path that tells the truth |

> **fd reuse trap:** freeing fd 1 means the kernel hands it to the **next `open()`** (lowest-available-fd rule). Any code still writing to fd 1 directly — including library code — silently writes into that unrelated new file instead of erroring. Never bare `close()` a standard fd; `dup2` something else onto it instead.

> `stdout` (fd 1) and `stderr` (fd 2) are fully independent table entries — closing one never touches the other, which is why every diagnostic line in this exercise (routed to `stderr`) kept working.

---

## h8 — pipe() connecting two children

Wiring `ls | wc -l` by hand:

```
pipe() → fds[0] (read) / fds[1] (write)
fork() child 1 (writer): dup2(write_end → stdout), close both ends, exec("ls")
fork() child 2 (reader): dup2(read_end  → stdin),  close both ends, exec("wc","-l")
parent:                  close BOTH ends,          wait for both children
```

| Who | Must close | Or... |
|---|---|---|
| Child 1 (writer) | read end + spare write end | leaks an unused fd |
| Child 2 (reader) | write end + spare read end | leaks an unused fd |
| **Parent** | **both ends** | reader **blocks forever** — pipe only EOFs once *every* write-end fd, in every process, is closed |

- Rust's `OwnedFd` makes this safer: `drop(fd)` closes it, and using an fd after dropping it is a compile error — harder to leak the wrong end than in C, where a forgotten `close()` compiles fine.
- `nix::unistd::dup2_stdout()`/`dup2_stdin()` are purpose-built for "redirect this fd onto the standard slot" — simpler than the generic `dup2()`, which in this nix version wants ownership of the *target* fd.

---

## h9 — mmap-sync struct sharing (fun side-quest off h3)
- h3 shares one fixed-size `int` flag via raw `mmap(MAP_SHARED|ANON)`, with the caller hand-rolling correctness: `volatile` so the compiler doesn't cache the read, and a single one-way 0→1 transition so a spin-wait can't observe a torn value.
- `mmap-sync` (Rust-only, [cloudflare/mmap-sync](https://github.com/cloudflare/mmap-sync)) generalizes that to an arbitrary, variable-size struct — safely, without hand-rolled `volatile` tricks. It keeps **two** mmap'd data files plus a small state file behind `Synchronizer::new(path)`: the writer always fills the *inactive* data file via `write()`, then atomically flips a pointer in the state file once the write is complete.
- This is RCU-style (Read-Copy-Update, same technique the Linux kernel uses): copy into a fresh version, then swap a pointer — never mutate the version a reader might currently be looking at. A reader's `read()` always lands on a complete, fully-written struct; it can never observe a partial write, and it never blocks the writer (or vice versa).
- `h9.rs` forks a writer (child, publishes a new `Snapshot{tick, pid, note}` every ~50ms) and a reader (parent, polls every ~12ms and prints each new version it sees) — same fork()-before-sharing shape as h3, but sharing a growing struct across two real *processes* instead of one flag, and without needing `volatile` since `mmap-sync`'s atomic state-pointer flip is what h3's `volatile` int + spin-loop was manually approximating.
- Data must implement `rkyv`'s `Archive`/`Serialize`/`Deserialize` (+ `bytecheck::CheckBytes` for validated reads) — zero-copy deserialization means the reader accesses the struct's fields straight out of mapped memory, no parse step.

## C → Rust port

| | `libc` | `nix` / `rustix` |
|---|---|---|
| What it is | Raw unsafe FFI bindings | Safe wrappers on top |
| Errors | Bare ints, manual `errno` checks | `Result<_, Errno>` |
| Flags | Magic numbers (`O_CREAT`, `PROT_READ`, ...) | Typed (`OFlag`, `ProtFlags`, `Mode`, ...) |
| Pick when | Need something not wrapped yet | Default choice — `nix` for broad POSIX coverage, `rustix` to avoid a `libc` dependency |
