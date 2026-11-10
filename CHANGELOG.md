# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!--
How to use this file:

- Every change adds its entries under the Unreleased heading, in the same commit that makes the
  change. A change with no entry is an incomplete change.
- Write for a human reading it six months from now. Say what changed for someone using the tool,
  not which function was refactored.
- Use only these headings, in this order: Added, Changed, Deprecated, Removed, Fixed, Security.
- A version number changes only when `main` moves. Work on `dev` stays under Unreleased no matter
  how much of it piles up. On a merge into `main` that ships a build, rename Unreleased to
  `[x.y.z] - YYYY-MM-DD`, open a fresh empty Unreleased above it, add the link references at the
  bottom, and tag the merge commit.
- Pre-1.0.0 versioning: breaking changes bump the minor, everything else bumps the patch.
-->

## [Unreleased]

### Security

- The administrator helper now loads the Windows libraries it needs from the Windows system folder
  only, so a file with the name of one of them left beside the helper is never run with administrator
  rights.
- The crash dump scan no longer follows a junction or link inside a dump folder, so a link planted
  there cannot make the administrator helper or the monitor list or read another dump.
- The administrator helper now checks, from the open result file itself, that it was created in the
  folder it was asked for, and deletes it through that same handle if a link was swapped in on the
  way, so a result file can no longer end up in another folder.
- The administrator helper now reads a tweak plan or a dump deletion list through one open handle that
  no one else can write to, and removes the surface scan's progress file through the handle that
  created it, so a folder swapped for a link while a job runs cannot redirect those steps.

### Fixed

- A wall clock set far ahead, by a time correction, a manual change or a second operating system on the
  same PC, no longer makes the monitor delete real history: the hourly cleanup now skips a run when
  the clock is more than an hour past the newest stored data.
- The watcher can no longer freeze for good when the window asks for its status at the very moment the
  tray icon refreshes. Before, two of its threads could each wait for the other, which stopped the
  tray menu and the icon, kept suspend notices from being handled and left the window showing a
  stale state until Windows ended the watcher.
- An install, update or uninstall that finds the monitor still running after asking it to quit and
  ending it now stops with a message, before any file changes, instead of going on. The message says
  to choose Exit on the tray icon and run the installer again.
- A deep capture cut short by the helper being killed no longer leaves its kernel trace session
  running: the monitor stops it when it starts, as long as no helper is running.
- A deep capture no longer gives a driver high confidence, nor tells you to update it, because of one
  long routine. A driver now needs three routines of 1 ms or more, or one next to a stall or a
  dropped frame; a single one is reported with medium confidence, in the capture and in the
  occurrence it covers.
- Looking up what a deep capture saw around an occurrence no longer misses a long routine or a
  dropped frame just because a long capture had worse ones elsewhere: the capture now keeps the
  worst of every minute, and the monitor keeps the probe's stalls for a whole day instead of 15
  minutes, so a capture opened late still shows them. The capture format changes, so the window,
  the monitor and the helper must be updated together; captures already stored still open.
- A deep capture whose frame recorder stopped partway no longer reads as if frames were measured for
  the whole capture: the window now says the frames were not measured and why. The capture format
  changes, so the window, the monitor and the helper must be updated together.
- A deep capture whose driver trace could not be read, or received no events at all, now says the
  driver measurements are unavailable instead of reporting that zero drivers were measured.
- A run of the Windows Memory Diagnostic no longer breaks History, Events, Period, Dumps and
  Settings with an error page, and memory errors it finds now open an alert in Health. History
  already recorded this way is repaired when the monitor starts. The history format changes, so the
  window and the monitor must be updated together.
- A disk surface scan that is still running now shows its progress when you close and reopen the
  window, keeps the test buttons busy, and can be stopped from the window.
- The disk speed test no longer starts on a volume with less than 1.5 GiB free, which is its 1 GiB
  file plus a margin; a volume that small used to fill up completely while the test ran. The
  Tests page now says how much space is needed instead.
- The disk speed test now deletes the 1 GiB temporary file an earlier run left behind (after a power
  cut, for instance), and when the disk stops answering it waits for the cancelled reads and writes
  to finish before releasing the memory they use.
- The volume check now records whether Windows had the volume marked for repair before and after the
  run, which shows when the check itself marked it. The test result format changes, so the window,
  the monitor and the helper must be updated together; results saved earlier are still read.
- The watcher no longer stops measuring for the whole session when the Windows performance counters
  are missing or damaged at start, or when its precise timer cannot be created. It keeps recording
  freezes, crashes and events, tries to open the counters again with a growing wait of up to a
  minute, and resumes the per-second readings as soon as they open. Before, nothing was recorded
  until the watcher was restarted, and the fall of the previous session was lost.
- A reading that fails in the watcher (memory, processor times, the performance counters or the
  process list) no longer cancels the rest of that second. The other readings, the detection of
  freezes and crashes, the pending events and the periodic writes carry on, and the saved list of
  the machine's disks is no longer emptied when the disk counters return nothing. Before, a reading
  that kept failing left freezes it had already seen unfinished and events unsaved.
- An event logged just before a freeze no longer moves the report's attention off the freeze: when an
  occurrence holds a freeze and an earlier event, the causes are judged around the freeze, so a busy
  processor or a slow disk next to the freeze is found instead of reading as unclear.
- A disk event or a graphics driver restart in the log no longer becomes the first suspect just
  because it fell within a minute and a half of a stall. Only events from 10 seconds before the stall
  to 5 seconds after it count; one further away no longer hides a busy processor or another measured
  cause. A disk event near the stall is reported with high confidence only when the disk was also
  answering slowly (over 200 ms) in that span, and with medium confidence otherwise.

## [0.1.0] - 2026-11-09

The first release.

### Added

- A light agent that monitors the PC from the tray, as your user and with no network connection. It
  keeps the last minutes of load, the busiest programs and Windows events, so a freeze can be
  explained after it is gone.
- A report for every freeze, program that stopped responding, stutter and unexpected shutdown: when it
  happened, how long it lasted, the likely cause with the evidence behind it, the steps to try, what
  happened second by second and a chart of the minute before. The Copy report button puts it on the
  clipboard as Markdown.
- A sensitive mode that checks the PC far more often for 30 minutes, to catch the short stutters that
  break games, audio and calls.
- Activity: the PC's states since midnight with every freeze and overload period marked, the history
  of past days, and what weighs on the PC right now, by program.
- Health: the PC checked by area every hour (disks, unexpected shutdowns, memory, startup, hardware
  and graphics, protection), with alerts that show whether they grew since they opened, and a full
  scan with administrator rights.
- Tests: the Windows Memory Diagnostic, a quick memory test, each disk's SMART or NVMe health, a
  read-only surface scan, a file system check and a speed test, with each disk's earlier scans kept.
- Dumps: the blue-screen, graphics-reset and app crash dumps Windows keeps, explained on the PC
  without the internet, with the stop code in plain words and the driver most likely at fault, and
  deleted once you confirm.
- Captures: a measurement of every driver's DPC/ISR latency and the frame times of the apps on screen,
  from 2 minutes to 24 hours, that names the driver or app behind stutters; each capture stays in a
  history.
- Optimizations: Windows settings worth changing on this PC, suggested only when a sign showed they
  would help, each applied and undone in one click. Security software, Windows Update and the page
  file are never offered.
- Tasks that need administrator rights run in a separate helper that asks for them one at a time.
- Updates in the window: with your permission it checks GitHub for a newer version at most once a
  day, shows what changed and installs it.
- The window in English and Brazilian Portuguese, light or dark, in four colour themes, at a Simple,
  Detailed or Technical detail level.
- A per-user installer in English and Brazilian Portuguese that needs no administrator rights and, when
  you ask, deletes the history on uninstall.

[Unreleased]: https://github.com/Filipe-Soares-de-Almeida/Vestigium/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Filipe-Soares-de-Almeida/Vestigium/releases/tag/v0.1.0
