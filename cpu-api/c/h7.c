#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {
  pid_t pid = fork();
  if (pid < 0) {
    fprintf(stderr, "fork failed\n");
    return 1;
  }

  if (pid == 0) {
    close(STDOUT_FILENO);

    // printf() just appends to stdio's userspace buffer -- same
    // buffering seen in h1/h4/h6 -- it does NOT call write(2)
    // immediately, so the closed fd isn't touched yet and printf()
    // "succeeds" (returns the byte count, as if nothing is wrong).
    int n = printf("can you see me?\n");
    fprintf(stderr, "child: printf() returned %d (looks successful)\n", n);

    // fflush() is what forces the real write(2) to fd 1. THIS is
    // where the closed descriptor actually bites -- it fails with
    // EBADF, silently, unless the caller checks the return value.
    int rc = fflush(stdout);
    fprintf(stderr, "child: fflush(stdout) returned %d, errno=%s\n", rc,
             strerror(errno));

    // a raw write() to fd 1 fails immediately -- no userspace buffer
    // to hide behind.
    const char *raw = "raw write\n";
    ssize_t w = write(STDOUT_FILENO, raw, strlen(raw));
    fprintf(stderr, "child: write(1, ...) returned %zd, errno=%s\n", w,
             strerror(errno));

    // fd reuse gotcha: fd 1 is free now, so the kernel hands it to
    // the very next open(). Any code that still writes to fd 1
    // directly ends up writing into THIS file instead of the
    // terminal -- a classic silent-redirection bug.
    int fd = open("./h7.output", O_CREAT | O_WRONLY | O_TRUNC, 0644);
    fprintf(stderr, "child: new open() got fd %d (reused old stdout slot)\n", fd);
    const char *msg = "surprise -- this landed in h7.output\n";
    write(fd, msg, strlen(msg));

    return 0;
  }

  wait(NULL);
  return 0;
}
