#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>

// waitpid()/WNOHANG only reports state TRANSITIONS (running -> stopped
// -> continued -> exited), and only the ones you asked for via flags.
// It can't tell you what a still-running child is doing right now.
// There's no dedicated syscall for that either -- /proc IS Linux's
// interface for it. /proc/<pid>/status is line-based ("Key:\tvalue"),
// simpler to parse than /proc/<pid>/stat's single packed line.
//
// State letters (same as ps/top's STAT column):
//   R - running (or runnable, waiting in the run queue)
//   S - interruptible sleep (blocked on I/O/timer/signal, can be woken
//       by a signal) -- what sleep() shows up as
//   D - uninterruptible sleep (usually blocked on disk I/O); can't
//       even be killed with SIGKILL until the I/O completes
//   Z - zombie (already exited, parent hasn't wait()ed yet)
//   T - stopped (by SIGSTOP, or being traced)
static char peek_state(pid_t pid) {
  char path[64];
  snprintf(path, sizeof(path), "/proc/%d/status", pid);
  FILE *f = fopen(path, "r");
  if (!f) return '?';
  char line[128];
  char state = '?';
  while (fgets(line, sizeof(line), f)) {
    if (sscanf(line, "State: %c", &state) == 1) break;
  }
  fclose(f);
  return state;
}

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
    printf("parent: child not done yet (state:%c), doing other work (poll %d)...\n",
           peek_state(pid), polls);
    fflush(stdout);
    usleep(500 * 1000);
  }

  printf("parent: waitpid() returned %d after %d polls, child exit status %d\n",
         rc, polls, WEXITSTATUS(status));

  return 0;
}
