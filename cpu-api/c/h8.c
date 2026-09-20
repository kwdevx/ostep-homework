#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>

// Wires up `ls | wc -l` by hand: two children connected through one
// pipe(), the shell's job normally hidden from you.
int main() {
  int fds[2];
  if (pipe(fds) < 0) {
    fprintf(stderr, "pipe failed\n");
    exit(1);
  }
  int read_end = fds[0], write_end = fds[1];

  pid_t p1 = fork();
  if (p1 < 0) {
    fprintf(stderr, "fork failed\n");
    exit(1);
  }

  if (p1 == 0) {
    // child 1 (writer): its stdout becomes the pipe's write end
    close(read_end);
    dup2(write_end, STDOUT_FILENO);
    close(write_end); // dup2'd onto fd 1 already; drop the spare fd
    execlp("ls", "ls", (char *)NULL);
    fprintf(stderr, "exec ls failed\n");
    exit(1);
  }

  pid_t p2 = fork();
  if (p2 < 0) {
    fprintf(stderr, "fork failed\n");
    exit(1);
  }

  if (p2 == 0) {
    // child 2 (reader): its stdin becomes the pipe's read end
    close(write_end);
    dup2(read_end, STDIN_FILENO);
    close(read_end);
    execlp("wc", "wc", "-l", (char *)NULL);
    fprintf(stderr, "exec wc failed\n");
    exit(1);
  }

  // parent: MUST close both ends. If the parent keeps write_end open,
  // the pipe never reports EOF to child 2 (the kernel only signals EOF
  // once EVERY write-end fd, across every process, is closed) -- child
  // 2's `wc -l` would then block on read() forever.
  close(read_end);
  close(write_end);

  waitpid(p1, NULL, 0);
  waitpid(p2, NULL, 0);

  return 0;
}
