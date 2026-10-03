// Go port of h8.c: wires up `ls | wc -l` by hand, two children connected
// through one pipe -- the shell's job normally hidden from you.
//
// Unlike h1-h7, both children here immediately exec another program, so
// this is the "fork+exec bundled" case os/exec is built for -- no cgo
// needed, since there's no fork-without-exec window where Go runtime
// safety would matter.
package main

import (
	"fmt"
	"os"
	"os/exec"
)

func main() {
	r, w, err := os.Pipe()
	if err != nil {
		fmt.Fprintln(os.Stderr, "pipe failed")
		os.Exit(1)
	}

	// child 1 (writer): its stdout becomes the pipe's write end
	ls := exec.Command("ls")
	ls.Stdout = w
	ls.Stderr = os.Stderr

	// child 2 (reader): its stdin becomes the pipe's read end
	wc := exec.Command("wc", "-l")
	wc.Stdin = r
	wc.Stdout = os.Stdout
	wc.Stderr = os.Stderr

	if err := ls.Start(); err != nil {
		fmt.Fprintln(os.Stderr, "exec ls failed")
		os.Exit(1)
	}
	if err := wc.Start(); err != nil {
		fmt.Fprintln(os.Stderr, "exec wc failed")
		os.Exit(1)
	}

	// parent: MUST close both ends. exec.Cmd.Start() dup2's each pipe fd
	// into the child and keeps its own copy open in the parent process,
	// so closing r/w here is the same "close both ends" step h8.c does
	// by hand -- otherwise the pipe never reports EOF to wc (the kernel
	// only signals EOF once EVERY write-end fd, across every process, is
	// closed), and `wc -l` blocks on read() forever.
	w.Close()
	r.Close()

	ls.Wait()
	wc.Wait()
}
