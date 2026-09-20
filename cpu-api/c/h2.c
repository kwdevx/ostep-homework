#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {
  int fid = open("./h2.output", O_CREAT | O_WRONLY | O_TRUNC, S_IRWXU);

  int f1 = fork();

  printf("opened (fd:%d), (pid:%d)\n", fid, (int)getpid());

  if (f1 < 0) {
    fprintf(stderr, "fork failed\n");
    exit(1);
  }

  const char *msg = "OOPS\n";

  //
  // Output: h2.output will contain two "OOPS" lines (never one
  // overwriting the other).
  //
  // Why: open() happens before fork(), so parent and child share
  // the same kernel file description, including its file offset.
  // The kernel locks that offset for each write(), so:
  //   1. first write()  -> lands at offset 0, offset moves to 5
  //   2. second write() -> lands at offset 5, offset moves to 10
  // Whichever process writes second just appends after the first.
  // (Order of parent vs child is not guaranteed.)
  //
  if (f1 == 0) {
    write(fid, msg, strlen(msg));
  } else {
    // parent path in here
    write(fid, msg, strlen(msg));
  }

  return 0;
}
