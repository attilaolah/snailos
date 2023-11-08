# SnailOS

SnailOS is a terminal and shell that runs in the browser.

## CoreUtils

GNU `coreutils` 9.4 is available, more-or-less. Currently **90 binaries** are
included, almost entirely unmodified (except for minor syscall / function
signature changes). The following **16 are still missing:**

- chgrp
- chown
- chroot
- cp
- df
- du
- install
- mkdir
- mv
- nice
- pinky
- sort
- stat
- stdbuf
- users
- who

They require a bit more work to compile.
