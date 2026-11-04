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
  Today and Now, and Settings at the bottom next to
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
  do the same from the keyboard. Ctrl and the mouse wheel, or + and −, zoom into a stretch of the
  day, which can then be dragged sideways; "Whole day" or 0 zooms back out. Then come the day's
  freezes and moments of pressure, each freeze a link to its report, and the number of freezes on
  each of the last 14 days. Before anything has been saved, the screen says when watching started
  and offers to show the PC now.
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
- Settings › Privacy and data can delete the history: after a confirmation, everything Vestigium
  kept on the PC (the timeline, the freezes and their reports, the Windows events it read) is
  deleted and the space is given back. Settings stay. The window and the background watcher must
  be updated together for this.
- Captures: "Investigate stutters" measures every driver and app for a few minutes, after Windows asks
  for administrator permission once, and names the driver or app behind the stutters; each capture
  is kept in a history. Nothing leaves the PC.
- A capture can last any whole number of minutes from 1 to 60: a fourth choice, Custom, opens a
  field for the length, for stutters too rare to show up in ten minutes.
- The watcher also keeps the Windows warning that a crash dump could not be prepared, which
  explains a blue screen that left no dump behind; it shows among the events of an occurrence.
- The tray icon asks for attention while a critical health point is open, such as a disk that
  reports it is failing or a system drive almost full, and its tooltip says how many. A quiet day
  with no critical point keeps the icon calm.
- Health, in the navigation pane, shows the PC by area (disks, falls and shutdowns, memory,
  startup, hardware and graphics, protection), each with its worst mark. An area opens the points
  that ask for attention, each with its value next to the healthy range, why Vestigium says so and
  what to do, and lists what it checked without an alert and what resolved in the last 30 days.
  Marks use a shape and a word, never colour alone. Today shows a Health card while a point is
  open, and the navigation pane counts the open points.
- When the PC goes down while Vestigium watches it (a blue screen, the power button held down, a
  fall while going to sleep, or a power cut), the next start turns it into its own report once
  Windows logs the fall: what went down and how, what the PC was doing in the two minutes before,
  and what to do. The watcher keeps those last seconds in a small file it writes to disk once a
  minute while the PC is calm and every five seconds while it is strained. Closing Vestigium or
  ending it from Task Manager is never taken for a fall.
- A full scan, on the Health page, reads what Windows shows only with administrator permission,
  such as what slows startup, after Windows asks for that permission once. It only reads and
  changes nothing.
- Dumps, in the navigation pane, lists the files Windows keeps after a blue screen, a video reset
  it recovered from, or an app that closed by itself, grouped by kind, with the space they take.
  "Read the dumps" asks Windows for permission once and reads each one on the PC, with Windows' own
  debugger engine and without the internet: the stop code in plain words, the driver or module it
  most likely points at and how sure that is, and what to do. Dumps can be ticked and deleted after
  a confirmation; what each one said stays. The page also says whether Windows is set to write a
  dump at all. A fall's report shows the dump of that fall, and the Health point about a blue
  screen leaving no dump leads to the setting.
- Optimizations, in the navigation pane, lists a fixed set of Windows settings (fast startup,
  hibernation, power savings, program preloading, search indexing, Game Bar background
  recording, GPU scheduling, multiplane overlay, apps that open at sign-in, and the dumps Windows
  keeps), each with what it holds now. A tweak is recommended only when Vestigium saw a sign on
  this PC that it would help, and says which; the others say why not. Applying asks for
  confirmation, shows what changes and when it takes effect, makes a restore point first when
  Windows asks for permission, and keeps the value from before so every change can be undone in
  one click. Each change shows whether it helped, by the same sign counted before and after.
  Security software, Windows Update and the page file are never offered, and a startup app is
  only ever turned off, never removed. Health points and the Dumps page link to the tweak that
  answers them.
- The background watcher keeps the results of the Windows Memory Diagnostic, and Health shows a
  critical point when it found memory errors, until a later run comes out clean.
- Tests, in the navigation pane, checks the memory and each disk without changing anything on
  them. For the memory: it schedules the Windows Memory Diagnostic for the next restart and shows
  what it covered, and it runs a quick test on the part of the memory that is free now, for 1, 5
  or 15 minutes, in the background. For each disk: what the disk says about itself (SMART or the
  NVMe health log; a hard disk is read only when asked, since reading wakes it), a read of the
  whole disk that maps how long each part takes and finds the parts that do not read, with stop
  and continue, a check of each NTFS volume's file system, and a short speed measurement with a
  temporary file next to the range typical for that kind of disk. The memory and the disks are
  listed on the left and the chosen one's report scrolls on its own. Every result says what it
  covered and what it cannot see, a test that runs shows its progress and can be stopped, and
  the tests that need it ask Windows for permission. Health shows the errors these tests find
  and links to them; Today and the occurrence report say when a Vestigium test was running.
- Holding Ctrl on the Now screen pauses it on the second it was pressed, like Task Manager: the
  chart, its readings and the program list stay still while you look, and letting go returns to
  live without losing a second. Switching to another window also lets go.
- The chart around an occurrence shows each second's values under the pointer, as the Now chart
  does, and zooms the same way: Ctrl and the wheel, + and −, dragging, and a button to see it all
  again.

### Changed

- On its first run the watcher reads the last 30 days of the Windows events it follows instead of
  the last 7, so the health checks that look back a month see the whole month from the start.
- Sensitive mode now also catches hitches from 8 ms, where it caught only those of 50 ms and up,
  and the tray says when hitches were seen.
- The project is now called Vestigium, and its GitHub repository was renamed to match. Links to
  the old addresses redirect.
- The background watcher now runs as `Vestigium.exe` and the window as `Vestigium.Window.exe`.
  Task Manager and the startup apps list show the watcher as Vestigium, with its icon, instead of
  a bare file name.
- The background watcher saves its history once a minute, compressed, and hands memory it only
  needed at startup back to Windows: about 30 MB of disk writes a day and 5 MB of memory in use,
  measured on a typical desktop.
- The tray icon is now the Vestigium mark, the red faceted V of the app icon, on a light or a dark
  taskbar alike, and Task Manager shows it the same way. A badge in its corner tells the state by
  its shape: a ring when the last 24 hours held slowdowns, a solid dot after a severe freeze, two
  bars over a dimmed mark while watching is paused, and a dashed ring with a dot while sensitive
  mode is on. The ring and the dot take the accent of the chosen colour theme, and the icon is
  drawn at the taskbar's own size, so it stays sharp at 125%, 150% and 200% display scale.
- The window, its taskbar button, the installer and the watcher in Task Manager now show the
  Vestigium icon, a red V cut like a gemstone, the same in every colour theme. The V fills almost
  the whole icon, and the mark in the window's title bar is 22 px.
- Update the window and the background watcher together: this window does not talk to a watcher
  from an earlier version still running, and says so until that watcher is exited from the tray
  and Vestigium is opened again.
- The live chart on the Now screen and the timeline of a capture now zoom like the day's timeline:
  Ctrl and the mouse wheel zoom, dragging or Shift and the wheel move, and "Whole range" shows
  everything again. A zoomed live chart stays where it is while new seconds arrive.
- Durations of a second or more now read in seconds ("1,01 s") instead of thousands of milliseconds
  ("1.008 ms"), which looked like one millisecond in Portuguese.
- Moving to the chart around an occurrence with the keyboard shows the reading at the moment it
  happened instead of a ring around the chart.
- The detail level (Simple, Detailed, Technical) is set in one place, Settings, and applies to
  every screen. Reports, Health, Dumps, Optimizations, Tests and captures no longer carry their
  own switch.

### Fixed

- The Health, Dumps, Optimizations and Tests screens open with their data instead of "Something
  went wrong", and the counts beside them in the side menu show again.
- A program with a long name no longer pushes "Using the most" on the Now screen past the edge of
  its box. The name ends in "…" and shows in full on hover; the process and event tables of a
  slowdown report and the app and driver tables of a capture do the same.
- Moving to the live chart with the keyboard shows the newest reading, as on the day's timeline,
  instead of a ring around the chart.
- A capture no longer counts the Windows desktop compositor, or an app that draws only when
  something on screen changes, as stuttering: their pauses are idle time, not late frames. Captures
  already stored are read the same way.
- Text in the window can no longer be selected, with the mouse or with Ctrl+A, and a right-click
  no longer opens the browser's menu; text fields keep both.
- "Open the data folder" in Settings opens the folder where the history really is, also when
  Windows keeps it in another place for the program that started Vestigium.
- The PC states chart on Today no longer draws a white frame around itself when it has focus; the
  highlighted stretch and its tooltip show where the keyboard is.
- With a state picked on the PC states chart, clicking a stretch of another state now switches to
  that state, and dragging a zoomed chart no longer selects the text around it.
- "Using the most" on the Now screen lists each program once, with the processor, memory and disk
  of all its processes added up, instead of one row per process. A browser running a dozen
  processes no longer fills the list with its own name, and its row shows what it uses in total.
- When the Now screen opens during a stretch of CPU, disk or memory pressure, or within ten minutes
  of one, its chart marks those seconds as under pressure from where the stretch began, as Today's
  timeline does, instead of only from the moment the screen opened. The window and the background
  watcher must be updated together for this.
- The "Live" dot on the Now screen now dims and brightens with each new reading, once a second,
  instead of pulsing on its own. The pulse kept the window redrawing all the time, so while Now was
  open it used about a third of one processor; it now uses about 2% of one.
- On Windows 10 the window now shows its top edge like the other three; Windows leaves it out for
  windows that draw their own title bar. It is hidden while the window is maximized.
- Memory pressure is now recorded when more than 85% of memory stays in use for two minutes,
  instead of 90%: while a large program holds its memory, Windows pushes other programs out to
  disk and keeps use just under 90%, so real pressure went unrecorded.
- Ordinary interrupt work on the first processor, from disk or graphics activity, no longer
  raises a hint to turn on sensitive mode or names a driver as a suspect. Both now need a quarter
  of one processor's time spent on interrupts, instead of a tenth for the hint and a fifth for
  the suspect.
- A freeze caused by programs taking every processor is now blamed on the CPU even when the
  watcher's first reading after the freeze also covers the moment just before it.
- The busiest-process rankings now count programs that start or exit between two rankings,
  such as the short compiler processes of a build, instead of showing them idle or leaving them
  out.
- A CPU slowdown now names the program behind it with all its processes added up, so a build
  is blamed on the compiler rather than on a browser that used less CPU in total.
- Starting programs or reading files no longer makes memory a suspect of a freeze on a PC with
  memory to spare: hard faults now count only while more than 85% of memory is in use.

[Unreleased]: https://github.com/Filipe-Soares-de-Almeida/Vestigium/commits/main
