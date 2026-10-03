// Go port of h2.c: open() before fork() shares one kernel file
// description (and its offset) between parent and child, so both
// writes land in the file instead of one overwriting the other.
package main

/*
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

// open() is variadic (the mode arg only applies with O_CREAT), so it
// needs the same fixed-arity wrapper treatment as printf/fprintf.
static int open_trunc(const char *path, int flags, int mode) {
  return open(path, flags, mode);
}
static void print_opened(int fid, int pid) {
  printf("opened (fd:%d), (pid:%d)\n", fid, pid);
}
static void print_fork_failed(void) {
  fprintf(stderr, "fork failed\n");
}
static void write_msg(int fid) {
  const char *msg = "OOPS\n";
  (void)write(fid, msg, strlen(msg));
}
*/
import "C"

func main() {
	fid := C.open_trunc(C.CString("./h2.output"), C.O_CREAT|C.O_WRONLY|C.O_TRUNC, C.S_IRWXU)

	f1 := C.fork()

	C.print_opened(fid, C.int(C.getpid()))

	if f1 < 0 {
		C.print_fork_failed()
		C.exit(1)
	}

	// Output: h2.output will contain two "OOPS" lines (never one
	// overwriting the other) -- see h2.c for the full explanation.
	if f1 == 0 {
		C.write_msg(fid)
	} else {
		// parent path in here
		C.write_msg(fid)
	}

	C.exit(0)
}
