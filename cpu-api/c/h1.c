#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main() {
  int x = 0;
  printf("before fork (x:%d)\n", x);
  int f1 = fork();

  if (f1 < 0) {
    fprintf(stderr, "fork failed\n");
    exit(1);
  }

  if (f1 == 0) {
    x++;
    printf("after fork child (x:%d)\n", x);
  } else {
    // parent path in here
    x++;
    printf("after fork parent (x:%d)\n", x);
  }

  return 0;
}
