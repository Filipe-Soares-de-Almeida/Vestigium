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

- A tweak plan can no longer be swapped while the administrator permission prompt is open. The window
  now keeps the plan file locked against writing and passes the helper its SHA-256, and the helper
  refuses to run a plan that does not match it.
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

- A watcher that stops answering no longer leaves the window waiting forever: a request to it now
  gives up after three seconds. A program that connects to the watcher's channel and says nothing, or
  sends half a request, is dropped after a few seconds instead of blocking the window.
- If the watcher's connection channel to the window ever fails, the watcher now notes the reason, opens
  it again after a second, and exits only when that keeps failing, instead of carrying on with no
  channel while the window says it is not running.
- Starting Vestigium a second time no longer starts a second watcher, even when the first one has lost
  its connection channel to the window, so two watchers never record into the same history.
- When another program is already using Vestigium's connection channel, Vestigium now says so in a
  message, naming the program when Windows allows it, instead of closing without a word.
- After the watcher restarts, the window now reconnects on its own instead of showing the warning
  about another program or version for a few seconds, and then the Start button. A request that may
  already have been carried out, such as clearing the history, is never sent a second time.
- The history screens, Today included, now open straight from the database while the watcher is stuck
  or busy, instead of freezing until it answers; only the last few minutes not yet written are missing
  until it replies. Applying or undoing a tweak no longer freezes the other screens while the
  permission prompt is open or the elevated helper runs.
- A disk surface scan now notes when other programs were using the disk while it ran, shows that on its
  result with a suggestion to repeat it with the PC idle, and no longer raises the "very slow reads"
  attention finding on a hard disk for it. Unreadable sectors are still critical. Scan results are now
  stored in a new layout, so the window, the monitor and the helper must be updated together; earlier
  results still read.
- A second deep capture started while one is already running, for example after the window was closed and
  reopened, is now refused by the administrator helper instead of stopping the first one's recording and
  leaving it truncated. The helper also no longer accepts a trace file path.
- A deep capture now credits each slow driver routine to the driver that was loaded at its address when
  it ran, so a driver that took the place of an unloaded one is no longer blamed for the earlier one's
  routines. A stalled frame is also matched with the stall the probe saw over the time each one lasted,
  so stalls longer than 50 ms back each other, and the capture's times follow the wall clock over a long
  run.
- Searching the Programs list now looks through every program the monitor reports, not only the 100
  with the highest processor use, so a program further down the list is found by name.
- Undo no longer overwrites a setting you changed yourself after applying a tweak. If Windows now holds
  another value than the one Vestigium set, the undo is refused with a message and nothing is written,
  and a refused undo no longer marks the original change as undone. The administrator helper also
  refuses a plan that asks for a value outside what the setting takes, and each written value is read
  back to check it.
- A tweak that changes a Windows service, or a power setting, and then fails on a later step no longer
  leaves the first change behind with no way back. The old start type or power value is now written
  back, and the history shows the real error. If even that cannot be written back, the change is kept
  as partly written, with Undo offered, instead of being listed as refused.
- Undoing a power tweak, such as the PCIe power saving or USB suspend setting, now puts the old value
  back in the power plan that was changed, not in whichever plan is active when you click Undo, and
  refuses with an error if that plan no longer exists. Before, switching plans between applying and
  undoing left the first plan changed and altered the second. Changes made before this update are
  undone as they were. The watcher and the window changed how they talk to each other, so they must
  be updated together.
- A helper that stops halfway, because of a full disk, a power cut or the process being ended, no
  longer leaves an empty result file that the monitor reads again at every import. The helper now writes
  its result under a temporary name and gives it the final one only when it is complete, and the
  monitor deletes an empty result or a temporary file that has not changed for a week.
- A tweak is no longer written to Windows when the restore point it asked for was not confirmed,
  unless you agreed to go on without one. Before, a point that Windows logged and then dropped, or
  one that Windows never confirmed, let the change through while the history said a point was made.
- A disk surface scan on a disk that disappears halfway, as when a USB cable is pulled or the port
  is suspended, now ends as failed because the disk stopped answering, and a later scan resumes where
  it stopped. Before, every block after that point was recorded as unreadable and the disk was told to
  be replaced. A really damaged area of a disk that still answers is still reported as unreadable.
- A Seagate hard disk that logged a single command timeout no longer shows an impossible count of
  4294967295 damaged sectors. The command timeout attribute is now read as the 16-bit counter it holds,
  and the number on a disk-health alert is the highest of the watched counters instead of their sum.
- A hard disk that was idle when the monitor restarted no longer has its open disk-health alert
  closed and reopened as a new one, which could hide a critical alert and reset its date. The alert now
  stays open until the disk is read again, and the idle disk is still never woken up to be read.
- A wall clock that steps back, as a time correction does after a PC with two operating systems
  starts, no longer hides freezes, app hangs and stutters for as long as it stepped back. Before, a
  stretch of lost watching recorded while the clock was ahead kept covering the time that followed
  and silently discarded everything new that touched it.
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
