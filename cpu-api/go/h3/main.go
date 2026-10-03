// Go port of h3.c: mmap(MAP_SHARED|ANONYMOUS) before fork() gives parent
// and child the same physical page (fork() copies the page-table entry,
// not the data), so the child can signal completion through it without
// wait(). See h3.c for the full mmap()-args walkthrough.
package main

/*
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <unistd.h>

static void print_line(const char *s) {
  printf("%s\n", s);
}
static void print_fork_failed(void) {
  fprintf(stderr, "fork failed\n");
}
static int spin_read(volatile int *p) {
  return *p;
}
*/
import "C"
import "unsafe"

func main() {
	addr := C.mmap(nil, C.sizeof_int, C.PROT_READ|C.PROT_WRITE,
		C.MAP_SHARED|C.MAP_ANONYMOUS, -1, 0)
	if addr == C.MAP_FAILED {
		C.print_fork_failed()
		C.exit(1)
	}
	// childDone aliases the shared mmap'd page directly; every read/write
	// through it is a real memory access to that page, no copying.
	childDone := (*C.int)(unsafe.Pointer(addr))
	*childDone = 0

	f1 := C.fork()

	if f1 < 0 {
		C.print_fork_failed()
		C.exit(1)
	}

	if f1 == 0 {
		// child: print first, then signal the parent
		C.print_line(C.CString("hello"))
		*childDone = 1
	} else {
		// parent: spin-wait on the shared flag instead of calling wait().
		// C's compiler is told this int may change from outside the
		// current thread's view via `volatile int *` in h3.c; Go has no
		// such qualifier, so instead each iteration reads through a cgo
		// call, which crosses the Go/C boundary and can't be hoisted out
		// of the loop the way a plain Go pointer dereference could be.
		for C.spin_read(childDone) == 0 {
		}
		C.print_line(C.CString("goodbye"))
	}

	C.exit(0)
}
