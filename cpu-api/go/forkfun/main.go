// Go port of forkfun.c: forks repeatedly in a loop, building a process
// tree -- each fork's child sets x, so it exits after one more
// iteration, but only that immediate child; any process further down
// the tree (grandchildren, etc.) keeps looping.
package main

/*
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

static void print_child(void) {
  printf("after fork child \n");
}
static void print_parent(void) {
  printf("after fork parent \n");
}
static void print_fork_failed(void) {
  fprintf(stderr, "fork failed\n");
}
*/
import "C"

func main() {
	x := -1

	for i := 0; i < 7; i++ {
		if x == 999 {
			C.exit(0)
		}

		f1 := C.fork()

		if f1 < 0 {
			C.print_fork_failed()
			C.exit(1)
		}

		if f1 == 0 {
			x = 999
			// child
			C.print_child()
		} else {
			// parent path in here
			C.print_parent()
		}
	}

	C.exit(0)
}
