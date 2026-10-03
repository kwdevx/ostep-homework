#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main() {
  int x = -1;

  for (int i = 0; i < 7; i++) {
    if (x == 999) {
      return 0;
    }

    int f1 = fork();

    if (f1 < 0) {
      fprintf(stderr, "fork failed\n");
      exit(1);
    }

    if (f1 == 0) {
      x = 999;
      // child
      printf("after fork child \n");
    } else {
      // parent path in here
      printf("after fork parent \n");
    }
  }

  return 0;
}
