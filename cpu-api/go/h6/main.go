// Go port of h6.c: waitpid()/WNOHANG only reports state TRANSITIONS, and
// can't tell you what a still-running child is doing right now -- for
// that, /proc/<pid>/status is Linux's only interface. See h6.c and
// Condensed.md for the state-letter table.
package main

/*
#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>

static void print_child_forked(int parentPid, int childPid) {
  printf("parent (pid:%d) forked child (pid:%d)\n", parentPid, childPid);
}
static void print_polling(char state, int polls) {
  printf("parent: child not done yet (state:%c), doing other work (poll %d)...\n",
         state, polls);
}
static void print_waitpid_result(int rc, int polls, int status) {
  printf("parent: waitpid() returned %d after %d polls, child exit status %d\n",
         rc, polls, WEXITSTATUS(status));
}
static void print_child_done(int pid) {
  printf("child (pid:%d) done\n", pid);
}
static void print_fork_failed(void) {
  fprintf(stderr, "fork failed\n");
}
*/
import "C"

import (
	"bufio"
	"fmt"
	"os"
	"strings"
	"time"
)

// peekState reads /proc/<pid>/status -- plain text, so plain Go I/O
// works fine here; no libc call needed, unlike the process-control
// calls elsewhere in this file.
func peekState(pid int) byte {
	f, err := os.Open(fmt.Sprintf("/proc/%d/status", pid))
	if err != nil {
		return '?'
	}
	defer f.Close()

	scanner := bufio.NewScanner(f)
	for scanner.Scan() {
		line := scanner.Text()
		if rest, ok := strings.CutPrefix(line, "State:\t"); ok && len(rest) > 0 {
			return rest[0]
		}
	}
	return '?'
}

func main() {
	pid := C.fork()

	if pid < 0 {
		C.print_fork_failed()
		C.exit(1)
	}

	if pid == 0 {
		C.sleep(2) // give the parent something to poll for
		C.print_child_done(C.int(C.getpid()))
		C.exit(0)
	}

	C.print_child_forked(C.int(C.getpid()), C.int(pid))

	// WNOHANG is the thing wait() simply cannot do: check on a specific
	// child WITHOUT blocking.
	var status C.int
	polls := 0
	for {
		rc := C.waitpid(pid, &status, C.WNOHANG)
		if rc != 0 {
			C.print_waitpid_result(rc, C.int(polls), status)
			break
		}
		polls++
		C.print_polling(C.char(peekState(int(pid))), C.int(polls))
		time.Sleep(500 * time.Millisecond)
	}

	C.exit(0)
}
