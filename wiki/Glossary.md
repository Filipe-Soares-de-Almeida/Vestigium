The terms Vestigium's screens use, in plain words.

**Bug check**: another name for a stop code.

**chkdsk**: Windows' file system check. Vestigium runs it on NTFS volumes in check-only mode (`chkdsk /scan`), and Windows does any repair, at the next restart or from the drive's **Properties › Tools › Check**.

**Crash dump, minidump**: the file Windows writes when it stops with a blue screen, holding the memory of that moment: all of it or the kernel's in `MEMORY.DMP`, or a small part in a minidump. An app that crashes can leave a dump of its own, and either kind shows which driver or module was running when things failed.

**Detail levels**: how much each screen says. **Simple** gives the summary and the actions, **Detailed** adds the measured values, and **Technical** adds the counters, events and IDs; set it in **Settings › Reports**.

**DPC**: deferred procedure call, the work a driver schedules to finish handling a device such as a network or sound card. It runs at high priority, and nothing else runs on that processor core meanwhile.

**DPC/ISR latency**: how long drivers hold a processor core with their ISRs and DPCs. Each call should last at most about 0.1 ms, and longer ones make audio crackle, games stutter and the mouse lag; a [capture](./Captures.md) measures it for every driver.

**Frame time**: the time between two frames an app draws on screen. Vestigium counts a frame as slow when it takes more than 2.5 times the app's usual frame and at least 20 ms.

**ISR**: interrupt service routine, the first, very short part of a driver that runs the moment a device signals the processor.

**Kernel-Power 41**: the event Windows logs at startup when the previous shutdown was not clean, meaning the PC restarted without shutting down properly.

**NVMe health log**: an NVMe SSD's own health report: how much of its rated life is used, its spare capacity, its temperature, media errors and unsafe shutdowns. It plays the part SMART plays on other disks.

**Overload period**: a stretch when the CPU, the memory or a disk stayed at its limit: every CPU core above 95%, memory in use above 85%, or a disk active 99% of the time or more. [Activity](./Activity.md) lists them.

**Sensitive mode**: for 30 minutes, Vestigium checks the PC every 16 ms instead of every 100 ms, to catch the short hitches that break games, audio and video. Turn it on from the tray or in **Settings › Monitoring**; it turns itself off.

**SMART**: the self-check a disk keeps on itself, with counters such as reallocated sectors and read errors and its own prediction of failure. Vestigium reads it when you ask, in [Tests](./Tests.md).

**Stop code**: the code Windows shows on a blue screen, also called a bug check, such as 0x133. It names the kind of failure, and [Dumps](./Dumps.md) explains it in plain words.

**Surface scan**: a read of every block of a disk, without writing, to find unreadable sectors and, on an HDD, regions that read very slowly.

**TDR**: timeout detection and recovery, Windows restarting a graphics driver that stopped responding. The screen blinks or freezes for a moment, and programs stay open.

**Unexpected shutdown**: the PC went off without shutting down properly: a blue screen, the power button held down, a power cut, or a failure during sleep. Vestigium makes a report from the minutes before it.

**WHEA**: Windows Hardware Error Architecture, the way the processor, the memory and the slots report hardware errors to Windows. A corrected error can come before a failure, and a fatal one takes the PC down.
