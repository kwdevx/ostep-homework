// Go port of h7.c. This demo is specifically about C stdio's userspace
// buffering (printf "succeeds" on a closed fd because it hasn't written
// yet; fflush is what actually hits write(2) and fails) -- Go's
// fmt.Println/os.Stdout.Write call write(2) directly with no such
// buffer, so using them here would silently lose the entire lesson.
// Everything below goes through real libc stdio via cgo instead.
package main

/*
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

static int open_trunc(const char *path, int flags, int mode) {
  return open(path, flags, mode);
}
static void print_fork_failed(void) {
  fprintf(stderr, "fork failed\n");
}
static int try_printf(void) {
  return printf("can you see me?\n");
}
static void print_printf_result(int n) {
  fprintf(stderr, "child: printf() returned %d (looks successful)\n", n);
}
static void print_fflush_result(int rc) {
  fprintf(stderr, "child: fflush(stdout) returned %d, errno=%s\n", rc,
          strerror(errno));
}
static void print_write_result(long w) {
  fprintf(stderr, "child: write(1, ...) returned %ld, errno=%s\n", w,
          strerror(errno));
}
static void print_new_fd(int fd) {
  fprintf(stderr, "child: new open() got fd %d (reused old stdout slot)\n", fd);
}
*/
import "C"
import "unsafe"

func main() {
	pid := C.fork()
	if pid < 0 {
		C.print_fork_failed()
		C.exit(1)
	}

	if pid == 0 {
		C.close(C.STDOUT_FILENO)

		// printf() just appends to stdio's userspace buffer -- it does
		// NOT call write(2) immediately, so the closed fd isn't touched
		// yet and printf() "succeeds" (returns the byte count, as if
		// nothing is wrong).
		n := C.try_printf()
		C.print_printf_result(n)

		// fflush() is what forces the real write(2) to fd 1. THIS is
		// where the closed descriptor actually bites -- it fails with
		// EBADF, silently, unless the caller checks the return value.
		rc := C.fflush(C.stdout)
		C.print_fflush_result(rc)

		// a raw write() to fd 1 fails immediately -- no userspace buffer
		// to hide behind.
		raw := C.CString("raw write\n")
		w := C.write(C.STDOUT_FILENO, unsafe.Pointer(raw), C.strlen(raw))
		C.print_write_result(C.long(w))

		// fd reuse gotcha: fd 1 is free now, so the kernel hands it to
		// the very next open(). Any code that still writes to fd 1
		// directly ends up writing into THIS file instead of the
		// terminal -- a classic silent-redirection bug.
		fd := C.open_trunc(C.CString("./h7.output"), C.O_CREAT|C.O_WRONLY|C.O_TRUNC, 0644)
		C.print_new_fd(fd)
		msg := C.CString("surprise -- this landed in h7.output\n")
		C.write(fd, unsafe.Pointer(msg), C.strlen(msg))

		C.exit(0)
	}

	C.wait(nil)
	C.exit(0)
}
