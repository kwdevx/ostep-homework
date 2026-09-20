#define _GNU_SOURCE // execvpe is a GNU extension, needs this before any #include
#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>

// Fork a child to run `body`, then wait for it before returning. `body`
// calls one exec*() variant and never returns on success (exec replaces
// the process image); if it does return, that exec failed.
static void run(const char *label, void (*body)(void)) {
  printf("=== %s ===\n", label);
  fflush(stdout); // exec() wipes the child's stdio buffer, so flush first
  pid_t pid = fork();
  if (pid < 0) {
    fprintf(stderr, "fork failed\n");
    exit(1);
  }
  if (pid == 0) {
    body();
    fprintf(stderr, "%s: exec failed\n", label); // only reached on failure
    exit(1);
  }
  wait(NULL);
}

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

static void do_execv(void) {
  // v = argument *v*ector (array), full path
  char *argv[] = {"ls", "-l", NULL};
  execv("/bin/ls", argv);
}

static void do_execvp(void) {
  // vp = argument vector + *p*ath search
  char *argv[] = {"ls", "-l", NULL};
  execvp("ls", argv);
}

static void do_execvpe(void) {
  // vpe = argument vector + path search + explicit environment
  char *argv[] = {"ls", "-l", NULL};
  char *envp[] = {"PATH=/bin", NULL};
  execvpe("ls", argv, envp);
}

int main() {
  run("execl", do_execl);
  run("execle", do_execle);
  run("execlp", do_execlp);
  run("execv", do_execv);
  run("execvp", do_execvp);
  run("execvpe", do_execvpe);
  return 0;
}
