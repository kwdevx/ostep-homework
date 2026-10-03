// Go port of h4.c: all six exec*() variants, differing on the same 3
// independent axes as the C original (list/vector, path/$PATH, inherit/
// explicit env). See h4.c and Condensed.md for the full axis table.
package main

/*
#define _GNU_SOURCE // execvpe is a GNU extension, needs this before any #include
#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>

// execl/execle/execlp are variadic in C, so cgo can't call them
// directly -- each gets a fixed-arity wrapper, same shape as h4.c's
// do_execl/do_execle/do_execlp.
static void do_execl(void) {
  // l = arg*l*ist, path must be a full path
  execl("/bin/ls", "ls", "-l", (char *)NULL);
}
static void do_execle(void) {
  // le = arg list + *e*xplicit environment (replaces, doesn't inherit)
  char *envp[] = {"PATH=/bin", NULL};
  execle("/bin/ls", "ls", "-l", (char *)NULL, envp);
}
static void do_execlp(void) {
  // lp = arg list + *p*ath search (bare filename, searches $PATH)
  execlp("ls", "ls", "-l", (char *)NULL);
}
static void print_header(const char *label) {
  printf("=== %s ===\n", label);
}
static void print_exec_failed(const char *label) {
  fprintf(stderr, "%s: exec failed\n", label);
}
static void print_fork_failed(void) {
  fprintf(stderr, "fork failed\n");
}
*/
import "C"

// v = argument *v*ector (array), full path
func doExecv() {
	argv := []*C.char{C.CString("ls"), C.CString("-l"), nil}
	C.execv(C.CString("/bin/ls"), &argv[0])
}

// vp = argument vector + *p*ath search
func doExecvp() {
	argv := []*C.char{C.CString("ls"), C.CString("-l"), nil}
	C.execvp(C.CString("ls"), &argv[0])
}

// vpe = argument vector + path search + explicit environment
func doExecvpe() {
	argv := []*C.char{C.CString("ls"), C.CString("-l"), nil}
	envp := []*C.char{C.CString("PATH=/bin"), nil}
	C.execvpe(C.CString("ls"), &argv[0], &envp[0])
}

// run forks a child to run body, then waits for it before returning.
// body calls one exec*() variant and never returns on success (exec
// replaces the process image); if it does return, that exec failed.
func run(label string, body func()) {
	C.print_header(C.CString(label))
	C.fflush(nil) // exec() wipes the child's stdio buffer, so flush first
	pid := C.fork()
	if pid < 0 {
		C.print_fork_failed()
		C.exit(1)
	}
	if pid == 0 {
		body()
		C.print_exec_failed(C.CString(label)) // only reached on failure
		C.exit(1)
	}
	C.wait(nil)
}

func main() {
	run("execl", func() { C.do_execl() })
	run("execle", func() { C.do_execle() })
	run("execlp", func() { C.do_execlp() })
	run("execv", doExecv)
	run("execvp", doExecvp)
	run("execvpe", doExecvpe)
	C.exit(0)
}
