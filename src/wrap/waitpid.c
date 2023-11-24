#include <sys/types.h>
#include <sys/wait.h>

#include <emscripten.h>

EM_ASYNC_JS(pid_t, js_waitpid, (pid_t pid, int *status, int options), {
  return await OS.waitpid(pid, status, options);
});

pid_t __wrap_waitpid(pid_t pid, int *status, int options) {
  return js_waitpid(pid, status, options);
}

