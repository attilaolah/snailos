#include <errno.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#include <emscripten.h>

// TODO: It is probably a better idea to mock waitpid,
// which is likely the ones who ends up calling the wait4 syscall.

EM_ASYNC_JS(pid_t, js_wait4, (pid_t pid, int *status, int options, struct rusage *rusage), {
  return await OS.wait4(pid, status, options, rusage);
});

pid_t __wrap___syscall_wait4(pid_t pid, int *status, int options, struct rusage *rusage) {
  return js_wait4(pid, status, options, rusage);
}

