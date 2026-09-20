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

## C → Rust port
- Hand-written `extern "C"` blocks (raw `libc` signatures) work but give zero ergonomics: bare ints for pid/fd, manual errno checks, magic numbers for flags (`O_CREAT`, `PROT_READ`, ...).
- `nix` crate wraps the same syscalls safely: `fork() -> Result<ForkResult, Errno>`, `open()/write()` with typed `OFlag`/`Mode`, `mmap_anonymous()` with typed `ProtFlags`/`MapFlags` — same syscalls, real error handling, no manual constant-guessing.
- `libc`/`nix`/`rustix` are on a spectrum: `libc` = raw unsafe bindings, `nix`/`rustix` = safe wrappers on top (pick `nix` for broad POSIX coverage, `rustix` if avoiding a `libc` dependency matters).
