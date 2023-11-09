# BusyBox

This package compiles the Almquist shell from the source code provided in the
Busybox package. No other BusyBox applets will be compiled.

1. Download the BusyBox source distribution, version
   [1.36.1](https://busybox.net/downloads/busybox-1.36.1.tar.bz2).
2. Unpack and copy `busybox.conf` to `.config`.
3. Edit the `Makefile` to compile against either WASIX or Emscripten.
