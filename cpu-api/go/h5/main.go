// Go port of h5.c: wait() called in a process with no children of its
// own (the child here) fails immediately, vs. blocking in the parent
// until its one real child exits.
package main

/*
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

static void print_child_start(int pid) {
  printf("child (pid:%d)\n", pid);
}
static void print_child_wait_result(int rc) {
  printf("child: wait() returned %d, errno=%s\n", rc, strerror(errno));
}
static void print_parent_start(int pid, int childPid) {
  printf("parent (pid:%d) forked child (pid:%d)\n", pid, childPid);
}
static void print_parent_wait_result(int rc, int status) {
  printf("parent: wait() returned %d (child's pid), child exit status %d\n",
         rc, WEXITSTATUS(status));
}
static void print_fork_failed(void) {
  fprintf(stderr, "fork failed\n");
}
*/
import "C"

func main() {
	pid := C.fork()

	if pid < 0 {
		C.print_fork_failed()
		C.exit(1)
	}

	if pid == 0 {
		// child: it has no children of its own, so wait() here has
		// nothing to wait for -- demonstrates the "wait() in the child"
		// question.
		C.print_child_start(C.int(C.getpid()))
		rc := C.wait(nil)
		C.print_child_wait_result(rc)
	} else {
		C.print_parent_start(C.int(C.getpid()), C.int(pid))
		var status C.int
		rc := C.wait(&status) // blocks here until the child exits
		C.print_parent_wait_result(rc, status)
	}

	C.exit(0)
}
