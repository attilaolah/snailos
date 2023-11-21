#include <unistd.h>

#include <emscripten.h>

EM_ASYNC_JS(ssize_t, js_read, (int fd, void *buf, size_t count), {
  return await OS.read(fd, buf, count);
});

ssize_t __wrap_read(int fd, void *buf, size_t count) {
  return js_read(fd, buf, count);
}
