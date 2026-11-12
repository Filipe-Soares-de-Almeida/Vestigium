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

### Added

- License notices of every open-source component in the install folder.
- Release notes page with the highlights of each version.
- Summary line of each version in the release notes.
- About dialog with each program's version and the credits.
- Copy information and Report a problem buttons in the About dialog.

### Changed

- Move the release notes from Updates to their own page.
- Show the highlights of the new version in the update dialog.

## [0.1.1] - 2026-11-11

### Added

- Strip above every screen while any deep capture runs.
- Buttons to cancel captures, volume checks, dump reads and full scans.
- What was measured block and Coverage mark on a deep capture's page.
- Tray badges and window banners when the monitor is not reading the PC.
- Monitor not answering status, with the last values kept on screen.
- Dialog when the monitor set aside a damaged history file.

### Changed

- Install Vestigium for every account, in `C:\Program Files\Vestigium`.
- Keep the four cards on the home screen one height.
- Separate stretches with no reading from suspended ones in PC states.
- Open Dumps, History, Today and the charts faster with a long history.
- Keep the worst moments of every minute in a long deep capture.
- Show what a volume check marks for repair at the next restart.
- Compare a tweak's before and after per monitored hour.
- Put the probable cause and its confidence first in the copied report.
- Stop stating an uncertain cause as fact in reports, Now and Today.
- Stop presenting an interrupted shutdown as a power cut.
- Improve the text of Now, Today, Health and Optimizations.
- Update the README and the privacy, security and questions pages.

### Fixed

- Fix the window, a deep capture or a dump read waiting forever.
- Fix a second deep capture cutting the first one short.
- Fix a partial deep capture claiming no frames, drivers or stutters.
- Fix which drivers and apps a deep capture blames or leaves out.
- Fix reports blaming the wrong program or disk, or a nearby event.
- Fix generic next steps in blue screen and disk cause reports.
- Fix a report with no readings saying that nothing was overloaded.
- Fix occurrences lost, counted twice or shown whole when cut short.
- Fix storms of repeated Windows events flooding or splitting reports.
- Fix unexpected shutdowns lost, misreported or blamed on a power cut.
- Fix Today and All events failing when two overloads start at once.
- Fix Home and Health showing all clear for data they could not read.
- Fix Health alerts closing and reopening, or arriving an hour late.
- Fix false or stale disk alerts, and idle hard disks woken for a check.
- Fix failed checks reading as file system errors or a clean startup.
- Fix surface scans on a busy, failing or unplugged disk.
- Fix a running surface scan hidden after the window is reopened.
- Fix the speed test filling a small volume or leaving its file behind.
- Fix Windows Memory Diagnostic breaking History and other screens.
- Fix test and program pages left blank or silent on an error.
- Fix tweaks suggested on weak evidence or the app's own disk tests.
- Fix antivirus startup entries offered for removal.
- Fix Undo and failed tweaks leaving settings you did not choose.
- Show what an interrupted tweak changed, with Undo when it is known.
- Fix a tweak applied, or a wrong message, when a restore point failed.
- Fix Dumps missing subfolder dumps or reading another account's.
- Show why some crash dumps could not be deleted.
- Fix gaps in readings when the disk, history or Windows counters fail.
- Fix readings and events lost on sleep or a failed history write.
- Fix the monitor failing on a damaged or briefly unavailable history.
- Fix Delete the history losing unsaved data or leaving old items shown.
- Fix one bad retention setting resetting the other two.
- Fix the monitor piling up result files or memory without limit.
- Fix history deleted when the PC clock is set far ahead.
- Fix freezes and the Now chart thrown off by a PC clock change.
- Fix a pause or sensitive mode ending at the wrong time.
- Fix the window freezing on a busy monitor or losing it on a restart.
- Fix a second start of Vestigium running two monitors or closing silently.
- Fix the tray icon and menu freezing or showing an old state.
- Fix the monitor left running at logoff or during an install.
- Fix Start with Windows pointing at another copy of Vestigium.
- Fix the live view and the not-responding time on Now going stale.
- Fix the Programs list missing programs in search or their file path.
- Fix a failed update check saying Latest version installed.

### Security

- Refuse to start the administrator helper from a folder the account can change.
- Keep the update signing key out of the build of the installer.
- Refuse a tweak plan swapped while the administrator prompt is open.
- Refuse to run a library planted beside the administrator helper.
- Ignore links planted inside a crash dump folder.
- Stop a link from moving the helper's result file to another folder.
- Stop a folder swapped for a link from redirecting a helper job.
- Refuse an Undo that the administrator helper did not record.

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

[Unreleased]: https://github.com/Filipe-Soares-de-Almeida/Vestigium/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/Filipe-Soares-de-Almeida/Vestigium/releases/tag/v0.1.1
[0.1.0]: https://github.com/Filipe-Soares-de-Almeida/Vestigium/releases/tag/v0.1.0
