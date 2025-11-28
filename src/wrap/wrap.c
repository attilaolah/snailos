#include <errno.h>
#include <unistd.h>

#include <sys/types.h>
#include <sys/wait.h>

#include <emscripten.h>

// JavaScript connectors wrappers:

EM_JS(pid_t, js_getpid, (), { return OS.pid; });

EM_JS(pid_t, js_getppid, (), { return OS.ppid; });

EM_ASYNC_JS(pid_t, js_vfork, (), { return await OS.vfork(); });

EM_ASYNC_JS(pid_t, js_waitpid, (pid_t pid, int *status, int options),
            { return await OS.waitpid(pid, status, options); });

EM_ASYNC_JS(pid_t, js_wait4,
            (pid_t pid, int *status, int options, struct rusage *rusage),
            { return await OS.wait4(pid, status, options, rusage); });

EM_ASYNC_JS(ssize_t, js_read, (int fd, void *buf, size_t count),
            { return await OS.read(fd, buf, count); });

EM_JS(ssize_t, js_write, (int fd, const void *buf, size_t count),
      { return OS.write(fd, buf, count); });

// Process management:

pid_t __wrap_getpid() { return js_getpid(); }

pid_t __wrap_getppid() { return js_getppid(); }

pid_t __wrap_vfork() { return js_vfork(); }

pid_t __wrap_waitpid(pid_t pid, int *status, int options) {
  return js_waitpid(pid, status, options);
}

pid_t __wrap___syscall_wait4(pid_t pid, int *status, int options,
                             struct rusage *rusage) {
  // TODO: It is probably a better idea to only mock waitpid,
  // which is likely the thing that ends up calling the wait4 syscall.
  return js_wait4(pid, status, options, rusage);
}

// I/O:

ssize_t __wrap_read(int fd, void *buf, size_t count) {
  return js_read(fd, buf, count);
}

ssize_t __wrap_write(int fd, const void *buf, size_t count) {
  return js_write(fd, buf, count);
}
