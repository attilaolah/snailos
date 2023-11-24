#include <unistd.h>

#include <emscripten.h>

EM_ASYNC_JS(pid_t, js_vfork, (), {
  return await OS.vfork();
});

pid_t __wrap_vfork() {
  return js_vfork();
}
