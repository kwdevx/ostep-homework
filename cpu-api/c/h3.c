#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <unistd.h>

int main() {
  // Shared flag, visible to both parent and child after fork(), so the
  // parent can tell when the child is done without calling wait().
  //
  // mmap() args, one by one:
  //   NULL                     -> let the kernel pick the address
  //   sizeof(int)              -> map just enough for one int (one page,
  //                               rounded up)
  //   PROT_READ | PROT_WRITE   -> the mapping can be read and written
  //   MAP_SHARED               -> the key part: writes go to the actual
  //                               page, not a private copy, and that
  //                               page is what fork() duplicates a
  //                               *reference* to (not the data itself)
  //   MAP_ANONYMOUS            -> not backed by a file, just zero-filled
  //                               memory from the kernel
  //   -1, 0                    -> fd and offset, unused/ignored because
  //                               of MAP_ANONYMOUS
  //
  // Because this call happens *before* fork(), the mapping already
  // exists when fork() copies the parent's page table, so both parent
  // and child point at the same physical page. Had this been a normal
  // stack/heap variable instead, fork()'s copy-on-write semantics would
  // give each process its own private copy the instant either wrote to
  // it, and neither side would ever see the other's update.
  volatile int *child_done = mmap(NULL, sizeof(int), PROT_READ | PROT_WRITE,
                                  MAP_SHARED | MAP_ANONYMOUS, -1, 0);
  *child_done = 0;

  int f1 = fork();

  if (f1 < 0) {
    fprintf(stderr, "fork failed\n");
    exit(1);
  }

  if (f1 == 0) {
    // child: print first, then signal the parent
    printf("hello\n");
    *child_done = 1;
  } else {
    // parent: spin-wait on the shared flag instead of calling wait()
    while (*child_done == 0) {
      ;
    }
    printf("goodbye\n");
  }

  return 0;
}
