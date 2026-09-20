#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {
  pid_t pid = fork();

  if (pid < 0) {
    fprintf(stderr, "fork failed\n");
    exit(1);
  }

  if (pid == 0) {
    sleep(2); // give the parent something to poll for
    printf("child (pid:%d) done\n", getpid());
    exit(0);
  }

  printf("parent (pid:%d) forked child (pid:%d)\n", getpid(), pid);
  fflush(stdout); // see h1/h4 notes: neither process flushes until exit,
                   // so without this the child's exit()-triggered flush
                   // can land before the still-running parent's output

  // WNOHANG is the thing wait() simply cannot do: check on a specific
  // child WITHOUT blocking. Useful whenever the parent has other work
  // to do while waiting -- e.g. a shell polling a background job
  // instead of freezing the prompt.
  int status;
  pid_t rc;
  int polls = 0;
  while ((rc = waitpid(pid, &status, WNOHANG)) == 0) {
    polls++;
    printf("parent: child not done yet, doing other work (poll %d)...\n", polls);
    fflush(stdout);
    usleep(500 * 1000);
  }

  printf("parent: waitpid() returned %d after %d polls, child exit status %d\n",
         rc, polls, WEXITSTATUS(status));

  return 0;
}
