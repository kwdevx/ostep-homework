// Go port of h1.c, using cgo to call libc's fork() directly.
//
// Go's runtime is multi-threaded (GC, sysmon, scheduler all run on
// separate OS threads), and fork() only duplicates the calling thread --
// the child ends up with a runtime that thinks it has many threads but
// actually has one. That's fine here because the child does nothing but
// a plain arithmetic op and a C printf call before exiting; it never
// touches the Go scheduler, allocates through the Go runtime, or spawns
// goroutines between fork() and exit.
package main

/*
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

// cgo can't call C's variadic printf/fprintf directly, so each format
// string gets a tiny fixed-arity C wrapper instead.
static void print_x(const char *label, int x) {
  printf("%s (x:%d)\n", label, x);
}
static void print_fork_failed(void) {
  fprintf(stderr, "fork failed\n");
}
*/
import "C"

func main() {
	x := 0
	C.print_x(C.CString("before fork"), C.int(x))

	f1 := C.fork()

	if f1 < 0 {
		C.print_fork_failed()
		C.exit(1)
	}

	if f1 == 0 {
		x++
		C.print_x(C.CString("after fork child"), C.int(x))
	} else {
		// parent path in here
		x++
		C.print_x(C.CString("after fork parent"), C.int(x))
	}

	// Go's runtime exits main() via a raw syscall, not libc's exit(), so
	// it never runs libc's stdio-flush atexit hooks -- an explicit
	// C.exit() is what actually flushes each process's buffered printf
	// output (and is what reproduces the "before fork" line printing
	// twice, matching h1.c).
	C.exit(0)
}
