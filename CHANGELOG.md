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

- A tray icon for the background watcher, with options to pause it, start it with Windows, and
  exit. Only one watcher runs per user; starting it again while it runs does nothing.
- The background watcher samples CPU, per-core DPC and interrupt time, memory, physical disks, GPU
  use and the busiest processes every second, checks for system stalls ten times a second, and keeps the
  history in a local database. It needs no administrator rights, opens no network connection, and
  saves what it has before Windows signs you out or shuts down.
- A first application window, opened from the tray icon. Only one window opens at a time, and
  closing it ends the program completely: nothing keeps running in the background.
- Sensitive mode, in the tray menu, watches for stalls about sixty times a second instead of ten,
  to catch the short hitches that break games, audio and video. It turns itself off after
  30 minutes, and the tray tooltip says while it is on.
- The watcher now notices freezes of the whole system, apps that stop responding, and long
  stretches of memory, disk or CPU pressure. For each one it keeps the minute before and the half
  minute after, with the Windows events of that time (storage resets, display driver resets,
  hardware errors, unexpected shutdowns), and names the likely cause with the values behind it.
  Time the PC spent asleep or with watching paused never counts as a freeze. The tray tooltip says
  how many slowdowns the last 24 hours held.
- The window now has its own layout: a title bar with the Vestigium mark, a navigation pane with
  Today and Now (Health and Dumps are marked as coming soon), and Settings at the bottom next to
  whether Vestigium is watching, paused or not running. It shows in English or Brazilian
  Portuguese, light or dark, and in one of four colour themes (Red, Graphite, Slate, Indigo),
  following Windows until you choose otherwise. When watching is paused, a banner offers to resume
  it; when the watcher is not running, the window still shows what was saved and offers to start
  it.
- The window draws its own title bar in the Windows 11 style, with minimize, maximize and close
  buttons. Drag it by the title bar, double-click the bar to maximize, and resize it from any
  edge; Snap (Windows key and the arrow keys) works as with any other window.
- The Today screen shows when the PC struggled. A sentence sums up the day at the chosen detail
  level. Below it, a timeline of the PC's states since midnight (calm, busy, under pressure,
  asleep or not watched) marks each freeze: point at a stretch to see when it ran and what the
  processor, memory, disk and graphics were doing that minute, click a state's name or a stretch
  to show only that state, and click a freeze to open its report; the arrow keys, Enter and Esc
  do the same from the keyboard. Then come the day's freezes and moments of pressure, each freeze
  a link to its report, and the number of freezes on each of the last 14 days. Before anything
  has been saved, the screen says when watching started and offers to show the PC now.
- Each freeze, app that stopped responding, burst of stutters or problem Windows reported now opens
  as a report: how long it lasted, the probable cause with the evidence beside it and any other
  suspects, what to try (with boxes to tick, remembered for that report), the sequence of what
  happened, and a chart of the minute before, the moment and the half minute after. Simple,
  Detailed and Technical switch how much it shows at once and become the default; Technical adds
  the busiest programs and the Windows events with their provider and ID. When the cause needs a
  deeper capture than this version has, the report says so, and the program behind a busy disk is
  named only as probable. "Copy report" puts the whole report on the clipboard as Markdown, in the
  window's language, ready to paste into a forum or an issue.
- A Settings screen, with its options in six categories listed on the left. Appearance: light or
  dark (or like Windows) and the colour theme, picked from small previews and applied at once.
  Reports: the default detail level. Watching: start with Windows, sensitive mode (saying when it
  turns itself off) and pausing until you resume. Kept history: how long each kind of history is
  kept, within what Vestigium accepts, with the disk space it is expected to take; a value it turns
  down is explained next to its field. Language: like Windows, Brazilian Portuguese or English;
  the tray changes at once and the window the next time it opens. Privacy and data: a reminder
  that nothing leaves the PC, the version, the space the history uses now and a button that opens
  its folder once there is one. While Vestigium is not running, the settings can be read but not
  changed.
- The watcher tells the window while a stretch of CPU, disk or memory pressure is under way, so
  the live view can show the PC as under pressure for as long as it lasts, and not after one busy
  second.
- The Now screen answers what is weighing on the PC at this moment. A sentence at the chosen
  detail level says whether it is calm, busy or under pressure and which program weighs most. A
  chart shows the last ten minutes of processor, memory, disk and graphics card, readable with the
  mouse or the arrow keys. Beside it are the five programs using the most, any window that stopped
  responding (it already counts as an occurrence in Today), and a link to the last freeze. The live
  numbers run only while the screen is on view, and stop when the window is hidden or minimized.

### Changed

- The project is now called Vestigium, and its GitHub repository was renamed to match. Links to
  the old addresses redirect.
- The background watcher now runs as `Vestigium.exe` and the window as `Vestigium.Window.exe`.
  Task Manager and the startup apps list show the watcher as Vestigium, with its icon, instead of
  a bare file name.
- The background watcher saves its history once a minute, compressed, and hands memory it only
  needed at startup back to Windows: about 30 MB of disk writes a day and 5 MB of memory in use,
  measured on a typical desktop.
- The tray icon is now the Vestigium mark, a faceted V, in the taskbar's own white or black, and
  it follows Windows when the taskbar turns dark or light. A badge in its corner tells the state by
  its shape: a ring when the last 24 hours held slowdowns, a solid dot after a severe freeze, two
  bars over a dimmed mark while watching is paused, and a dashed ring with a dot while sensitive
  mode is on. The ring and the dot take the accent of the chosen colour theme, and the icon is
  drawn at the taskbar's own size, so it stays sharp at 125%, 150% and 200% display scale.
- The window, its taskbar button, the installer and the watcher in Task Manager now show the
  Vestigium icon, a red V cut like a gemstone, the same in every colour theme.
- Update the window and the background watcher together: this window does not talk to a watcher
  from an earlier version still running, and says so until that watcher is exited from the tray
  and Vestigium is opened again.

[Unreleased]: https://github.com/Filipe-Soares-de-Almeida/Vestigium/commits/main
