#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {
  pid_t pid = fork();

  if (pid < 0) {
    fprintf(stderr, "fork failed\n");
    exit(1);
  }

  if (pid == 0) {
    // child: it has no children of its own, so wait() here has nothing
    // to wait for -- demonstrates the "wait() in the child" question.
    printf("child (pid:%d)\n", getpid());
    pid_t rc = wait(NULL);
    printf("child: wait() returned %d, errno=%s\n", rc, strerror(errno));
  } else {
    printf("parent (pid:%d) forked child (pid:%d)\n", getpid(), pid);
    int status;
    pid_t rc = wait(&status); // blocks here until the child exits
    printf("parent: wait() returned %d (child's pid), child exit status %d\n",
           rc, WEXITSTATUS(status));
  }

  return 0;
}
