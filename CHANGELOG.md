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

- The release build no longer has the update signing key in its environment. The installer is built
  with no secret, a separate job that receives only the installer signs it, and a third job holds the
  permissions to publish the draft release, so a compromised build dependency cannot use the key or
  alter the installer before it is signed. The actions the release uses are pinned to commits.
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

- The timeline of an occurrence now names the disk, or the processor core, that reached the peak
  value it quotes. When two disks went past a limit in the same window, it used to name the disk
  that crossed first next to the peak of the other.
- A not-responding app that bridged two nearby incidents now joins them into one occurrence. The two
  used to be saved as separate occurrences with overlapping windows, which counted the same moment
  twice in the tray.
- An occurrence of a long freeze or a long not-responding app, whose first seconds had already left
  the ten minutes the monitor keeps in memory, is now marked as cut short instead of looking
  complete.
- The privacy page and the reporting page now say that the details Windows logs for an event, which
  Vestigium keeps and shows, can include the path of a program or file and so your user name.
- A retention setting that is out of range or unreadable no longer resets the other two. Only the
  bad value goes back to its default, and the monitor records which one it replaced.
- A deep capture whose frame tool wrote lines but no usable frame now shows the frames as not
  measured, with the reason, instead of as a run with no frames. The same goes for output with no
  header the monitor can read, such as a program name with a comma in it.
- A program started from a very deep folder now shows its full path. The path used to be lost when it
  was longer than about 1,000 characters, so the program appeared with no file.
- A boot log that fails while it is being read is no longer taken for a boot with nothing slow in
  it. The scan now counts as not read, so the health page shows no boot result instead of a clean
  one, and the scan keeps a count of the boot events it could not read. Scans saved by the earlier
  version are still read. The helper and the monitor must be updated together, as the scan file
  layout changed.
- A damaged black box file can no longer move the window of the post-mortem after a fall. Records
  with a time before 2020 or more than a day ahead of the clock are ignored, and a file written by
  another version of the monitor is reported as a stop that was not clean instead of as a clean one.
- After a start, the health check now runs again once the first batch of Windows events is saved.
  The check a minute after start could not see the boot and crash events still waiting to be
  saved, so an unexpected restart appeared in the health findings up to an hour late.
- A disk that was removed from the PC no longer keeps a health warning. The hourly check used to
  keep each disk's last reading until the monitor restarted; it now forgets disks that are no longer
  in the machine, so their warning closes at the next check.
- The hourly health check no longer wakes an idle hard disk whose type Windows would not report,
  such as one behind a RAID or host adapter. A disk of unknown type is read only while it is in use
  or when it is an NVMe drive.
- The tray icon now corrects itself when Windows rejects an update. If the icon or the tooltip could
  not be changed, for example while Explorer was restarting, the monitor tries again on its next
  refresh instead of showing the old level until the level changed again.
- Start with Windows now follows the program when it moves. If the entry points to another copy of
  Vestigium, the monitor rewrites it to itself at the next start, unless you turned it off in Task
  Manager, and it never creates an entry that was not there. Uninstalling Vestigium now removes
  its start entry, while an update keeps it.
- When the PC is about to sleep or hibernate, the monitor now writes what it holds first. The
  suspend notice wakes the sampler at once and waits up to a second and a half for it to save the
  open batch, pending events, gaps and the black box, where before this was done only after the PC
  woke up, or lost if it never did.
- A volume check can now be stopped, and it no longer runs without a limit. Stopping it ends
  chkdsk within a second or two and the check is recorded as not completed; a check that is still
  running after six hours is ended the same way.
- Stop on a disk surface scan now takes effect within seconds even on a failing disk. Before, it was
  only looked at between 1 MiB blocks, and a block that failed was read sector by sector with no
  limit, which could take minutes. The stop is now checked before every sector and while a read is in
  progress, a read that is still unanswered after 30 seconds is cancelled, and a block that takes more
  than 30 seconds to narrow down counts as unreadable. A block cut short by Stop is left out of the
  map and read again when the scan resumes.
- The crash dump scan no longer hangs on a dump the Windows debugger engine cannot load. Each dump
  gets at most one minute, the whole scan at most ten minutes, and a dump left out of the analysis is
  still listed with its size and date.
- A deep capture no longer waits forever for the frame recorder. If PresentMon is still running 15
  seconds after the capture should have ended, it is ended and the capture finishes with the frames
  read so far, or marked as not measured when there were none. PresentMon is also tied to the
  administrator helper, so ending the helper ends it too, and the monitor stops a leftover PresentMon
  trace session at start, as it already did for the kernel one.
- Startup entries of common antivirus products, such as Avast, AVG, ESET, Kaspersky, Norton, McAfee
  and Bitdefender, are now shown as protected on the Optimizations screen and can no longer be
  disabled from it. Before, only names with words such as Defender or Security were protected. A slow
  startup is also blamed on an entry only when the names are the same program, not when they share
  four letters, so an entry called Sync is no longer blamed for the delay of Dropbox Sync Helper.
- The Did it help comparison on the Optimizations screen no longer shows a result for a change made
  so long ago that the days before it fall outside the 30 days of history the watcher keeps. Before,
  a change applied more than 23 days ago was compared against a before period that was cut short or
  empty, so it could report fewer problems or no difference by mistake.
- Crash dumps kept in a subfolder of the crash dumps folder no longer disappear from the list and from
  the space count until the next dump scan. The watcher's own listing now reads one level of
  subfolders, as the dump scan does, so it no longer marks those dumps as deleted.
- A health alert about two antivirus products running, or about the page file, now stays open when
  Windows does not answer the check. Before, the alert closed, or never opened, whenever the Security
  Center was not up yet (often just after logon) or the page file could not be read, and a failed
  antivirus check was not asked again for a day. The check is now repeated at the next health pass.
- A program that is still not responding 15 seconds or more after the PC wakes from sleep or the
  watcher resumes from a pause now gets a report. Before, a hang first seen right after a resume was
  always discarded as the settling time of waking, so a program that stayed frozen for minutes never
  appeared.
- A report with a very large number of entries, such as one made by thousands of repeated Windows
  events, is no longer written to the history in a form that could not be opened again: the history now
  reads reports up to four times larger than before, so one that was stored but showed an error opens.
- A storm of repeated Windows events, such as a disk or storage controller logging the same error
  many times a second, no longer fills one report with thousands of entries or splits into a report
  every few minutes. A report now keeps the first 16 events of each provider and event number, and
  events of one provider within two minutes of each other join one report. A report with such an event
  now appears about two minutes after the event instead of half a minute, because it waits for more.
- A program that stopped responding is now blamed with high confidence only when its own processor
  or disk use shows during the freeze, or when nothing else in the system explains the stall.
  Before, any hung window put its program first, ahead of a measured memory or disk cause, so a
  program that was only waiting on a starved PC was named as the culprit.
- A memory shortage now names the program that held the most memory in the 30 seconds before the
  stall, not the biggest of the whole report. Before, a program that grew after the stall began, or
  one that had left and been replaced by a bigger one, could be named, and the advice was to close
  the wrong program.
- The English title of a disk whose SMART predicts failure no longer puts a colon after the drive name, which showed a doubled colon for a drive letter. It now reads "SMART of disk C: predicts failure".
- Today, Events and a program's page no longer fail to open when two overload periods start in the
  same moment, such as two disks at full activity together or the processor and the memory filling at
  once. Before, the Moments list and the Events table showed an error for the rest of the day.
- The report of a PC that fell now includes the disk and display events Windows logged in the two
  minutes before the last second the watcher saw. Before, they arrived after the report was written, so
  a freeze caused by the disk or the graphics driver was reported as a power cut of low confidence;
  now it names the storage or the graphics driver.
- Settings, clearing the history, importing results and applying a tweak no longer stay unavailable
  for the rest of a run after the history could not be opened once at start: the watcher tries again
  at the next request that needs it and imports the results that were waiting. A history location
  that cannot use WAL mode, such as a network folder, is now refused with an error naming the mode
  it found instead of being used unsafely.
- A history file with a damaged header no longer keeps the watcher from starting at every logon: the
  watcher moves the file, with its -wal and -shm files, to a name ending in .corrupt and the time, never
  deletes it, and starts a new empty history. A file that is only locked, or that a newer version
  wrote, is left alone.
- A history write that fails, for example on a full disk or a locked file, no longer loses the data
  of that moment: the batch is kept and written with the next one, up to three batches, and the
  Windows events it held are still read after it, so none is skipped. A crash, freeze or pressure
  episode that could not be stored is tried again too, and the tray counts it only once it is saved.
- A watcher that stops answering no longer leaves the window waiting forever: a request to it now
  gives up after three seconds. A program that connects to the watcher's channel and says nothing, or
  sends half a request, is dropped after a few seconds instead of blocking the window.
- Clearing the history, importing results, saving settings or applying a tweak that takes several
  seconds no longer makes the window report a silent watcher: these requests now wait up to a minute
  for the answer, while a request that only reads the watcher's memory still gives up after three
  seconds.
- A deep capture, a health scan or a tweak result file that the watcher could not delete after storing
  it is no longer stored again at the next import: each is kept once. Copies a database already holds
  are removed when it is opened by this version, so the window and the watcher must be updated
  together.
- A tweak that the administrator helper started but never finished, because it was stopped or the PC
  lost power partway, now leaves an entry in the tweaks history instead of nothing. The helper writes
  down what it is about to change before it changes anything and changes nothing when it cannot. About
  ten minutes later the watcher compares each setting with what Windows holds: a setting that holds
  the new value is recorded as written and can be undone, one that holds the old value as unchanged,
  and anything else as unknown, with no Undo. The window and the watcher must be updated together.
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
- A wall clock that jumps, by a time correction, a manual change or a second operating system, no longer
  makes an app hang or a stutter last for the whole jump, invents a stutter, or fills a freeze's evidence
  with moments from another hour. The watcher now notices the jump, closes what was open, writes it,
  starts its short memory over and records the change in the timeline as "PC clock changed".
- A wall clock that steps back no longer keeps the watcher from writing its history for as long as it
  stepped back, which could lose minutes of data if the PC fell meanwhile. A batch that grew very large
  is now written in several parts, so the history can always read it back; before, a step of about a day
  back could leave a batch too large to be read.
- The watcher's memory no longer grows by about 3.5 MB for each hour the wall clock steps back. The
  program rankings of the last ten minutes are now limited by their number as well as by their age, and
  their list of program names is rebuilt only when it has doubled, which also saves processor time on a
  PC with many short-lived programs.
- The ten-minute chart on the Now screen no longer stops growing after the wall clock steps back. Before,
  every new point looked older than the last one and was ignored until the clock caught up; now the
  points that lie ahead of the new time are dropped and the chart goes on from there.
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
