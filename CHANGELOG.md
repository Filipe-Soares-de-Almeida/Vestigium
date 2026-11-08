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

- Many list rows now open a detail page, with a back link to the list they came from and the navigation
  pane staying where you were: a test in the history, a SMART attribute, a Health alert that was
  resolved, a program, a Windows event of a freeze report, and a driver of a capture report. Each page
  shows only what Vestigium has kept: it has no chart for what it does not record yet, such as the
  history of one SMART attribute or one program over time.
- Settings are now chosen from the navigation pane, with its seven pages (Appearance, Reports, Language,
  Monitoring, Data retention, Updates, Privacy and data) under one group, reached from the Settings card
  on the home screen or from the search. Each page is a flat list of settings under its own title.
- Captures has a new layout: the permission note of a new capture is highlighted, past captures are rows
  with their result and time, and a capture's report is made of titled sections with a back link to the
  list. The navigation pane keeps a single Captures entry, highlighted on the report too.
- Dumps are now browsed from the navigation pane: the overview, and one list each for blue screens,
  kernel reports and apps that crashed, with their counts. A dump opens on a page of its own with a back
  link to its list. Reading the dumps shows the administrator badge, a group's check box ticks or clears
  what that list shows (with a dash when only some are ticked), and the check marks sit centred in their
  boxes.
- Optimizations are now browsed from the navigation pane: Recommended, No sign on this PC, No change,
  Applied and All tweaks, each with its count. A tweak opens on a page of its own, with the administrator
  badge when it needs permission, and the back link returns to the list the tweak belongs to now, so
  applying or undoing it moves it to the right one.
- Tests has a new layout. The overview lists the memory and each disk as rows with a status symbol, and
  each device opens as a page of titled sections; the devices are chosen from the navigation pane instead
  of a list inside the page. The Windows Memory Diagnostic shows the administrator badge.
- Disks and Memory have an overview each. Disks shows, for every disk, how it stands, the SMART values that
  warn once read, and the Health alerts about it. Memory shows its Health checks, the memory tests, any
  memory overload today and the history of memory tests. Each links to the Health and Tests pages it draws
  from, and the navigation pane stays on Disks or Memory when you follow them.
- Health has a new layout. The overview lists the areas of the PC as rows, with a status symbol only for
  the areas that have alerts, and the full scan shows a badge that it asks for administrator permission.
  Each area page puts every alert in its own section, with the checks that found nothing marked by a green
  check mark and the alerts resolved in the last 30 days.
- A Day's events screen lists every freeze, stutter, unresponsive program and overload of the day, with a
  filter by kind. Overload periods now open on a page of their own instead of expanding in the list, and
  freezes and overloads have a back link to the list or the History day they were opened from.
  The freeze report uses the same layout of titled sections.
- Now, the live view, has its own place in the navigation pane, with a Programs screen: the full list of
  running programs, with a filter and sortable columns, that keeps updating every second. Now itself uses
  the new layout, and its top programs link to the full list instead of opening a pop-up.
- A History screen, under Activity, lists every day since the first record. A calendar colours each
  day by its freezes, with arrows to change month; a Period list narrows it to the month, the last week or
  the last two weeks; a chart shows freezes by weekday; and Day by day lists the days, or the freezes of the
  day you pick. Opening a freeze from there and going back returns to the same month and day.
- Today has a new layout: the title with the date, the PC states chart, then the day's events and
  Health side by side on wide windows, and the last 14 days. Each event shows its status as a symbol, and
  the word appears when you point at the row. The Today and Now tabs are gone; the navigation pane
  switches between them.
- Opening a card on the home screen now grows it into its page, and going back shrinks the page into
  the same card. The effect is skipped when Windows is set to reduce motion.
- The window now opens on a home screen: the status of the watcher, three quick cards (Health, Activity,
  Optimizations), a card for Settings, a search field and a grid of the ten subjects. Search finds
  screens, optimizations and each Settings page, ignoring accents, and opens the match with Enter.
- The navigation pane now follows the subject you are in: Home, search, the subject's pages and the
  watcher status at the bottom. Disks and Memory gather the health alerts and tests of each.
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
- Dumps and Optimizations have a box at the top of each group that chooses every item in it at
  once, and shows a dash while only some are chosen. The bar below still says how many and how
  much before anything is deleted or applied.
- "Who is using the most" on the Now screen has a "See all" button that opens every program
  running, live, up to 100 at a time: sort by any column, find one by name, hold Ctrl to pause it,
  and close it with Esc or a click outside.
- A Health finding compares the value now with the one it had when it opened ("Now: 24 damaged
  sectors · 4 days ago: 16 damaged sectors"), so you can see whether it is growing. A finding
  about a disk that was read in full also shows the sector map of that read.
- The installer carries everything Vestigium runs: the window, the background watcher, the helper
  that does the tasks needing administrator permission, and the PresentMon console the deep
  capture uses, with its licence.
- Installing Vestigium over itself, or uninstalling it, while the background watcher runs closes the
  watcher first and lets it save what it holds; after an install it starts again. Choosing to delete
  the app data when uninstalling also deletes the history.
- The window keeps Vestigium up to date. The first time it opens, it asks once whether to check
  for updates on its own; with yes, it asks GitHub for the newest version when it opens, at most
  once a day, and nothing Vestigium keeps goes along. A newer version opens a dialog with what
  changed in it, and "Update and restart" downloads it with its progress and installs it, closing
  and reopening the window while watching goes on. After an update, the window shows once what
  changed since the version you had, in English or Portuguese. Settings › Updates has the switch,
  "Check now" and the notes of every version.
- An overload period in the day's list opens in place under its row: when it ran, its peak and
  average, a chart of the period with the minutes around it, the programs that used the most CPU,
  memory or disk during it, the freezes inside it with a link to each report, and what to do.

### Changed

- The home screen and the navigation pane say more about this PC: the Freezes card counts today's
  occurrences and overload periods, Disks names each disk with its letter, Memory gives its size and
  free memory, Dumps the space they use and Settings the version. In the pane, All events shows
  today's count, disks carry their letter, Disk alerts and the test devices show a mark when they need
  a look, and every Optimizations list shows its count, zero included.
- Detail pages and every page opened from a list name only the place their back link returns to, such
  as Memory instead of Back to Memory. A detail page shows its facts in one line under the title and
  its blocks two by two on a wide window, each opened by a divider.
- Settings pages are titled sections instead of setting cards. Appearance is now Colors: the color
  mode is three radio buttons and each color theme is a tile with its accent and state colors. Reports
  chooses the detail level with radio buttons, each with its hint. Monitoring groups its switches
  under Startup and Sensitivity, each named over the switch with its note under it. Data retention
  lists each kind of data with its period and expected size, and the total expected space in large
  type. Updates shows the installed version with Check now and the release notes in the page. Privacy
  and data sets the kept data and the folder button apart from deleting the history, which asks for
  confirmation in place.
- Captures sets New capture beside the past captures. The length is chosen from radio buttons with a
  hint each, the permission note stands out above the other notes, and Start capture comes before the
  note that the capture goes on with the window closed. A running capture shows its elapsed time large
  over a thin bar. Past captures show the day and time with their result, and a mark for a suspect
  driver or nothing found. A capture's report opens with its duration and result, sets the conclusion
  beside what to do, shows the timeline with its zoom buttons, lists drivers with their longest call
  against the recommended 0.1 ms on a bar, and apps as plain rows.
- Dumps has one bar under its summary to select all, delete the selection and see its size, with the
  confirmation in place instead of a dialog. Each row keeps its check box beside it and shows whether
  it was read, when it was written and its size; deleted dumps are counted in one line. Space used,
  with a bar and its key, sits beside Read the dumps, and the Windows dump setting closes the page
  with its status. A dump's page shows its culprit beside the stop code, the call stack as a chain of
  modules, what to do beside the file, and a block to delete it.
- Optimizations opens on Optimizations with its summary, the last restore point and Recommended for
  this PC; every list row shows the change it makes, such as On → Off, instead of a status symbol.
  Tweaks are applied one at a time from their own page, so the check boxes and the bar to apply
  several at once are gone. A tweak's page sets What it does and Why beside What changes, which shows
  the current and new values in two boxes, its effect, permission and cost, and Apply or Undo. All
  tweaks lists every group under its own title with its count, and the other groups say what they hold
  under their title.
- Tests lists the memory and each disk under Devices. A disk's page is a regular page that scrolls as
  a whole: its title carries the status and the disk's connection, the surface check comes first with
  its map, result and estimate, then SMART beside the file system and the speed test beside the
  history. SMART values are rows; the speed test shows each measure large over a bar with the typical
  range. The memory page sets the Windows diagnostic beside the quick test, whose length is chosen
  with radio buttons, and the full test outside Windows beside the history.
- Health shows when the last full scan ran under its title and opens with the overall status beside
  its summary. Each area row shows its status, a green check when it has no alert, then the count; the
  full scan and a History of new, resolved and scanned alerts sit side by side. An area page carries
  its status and count under its title and no back link; each alert has its signs, what to do and, for
  a disk, its surface beside it, with the measure now and before in one box, and the checks with no
  alert close the page. Disks names each disk with its icon, its connection and letter, lists the
  SMART values read as rows that open them, and shows each alert with its measure. Memory sets its
  health beside the three memory tests and its overloads of the day beside the history of tests. Tests
  read Has failures, No errors or Partial.
- A freeze report opens with its title, date and Copy report at the right, then a line with its
  severity, duration and where it happened. The probable cause and what to do sit side by side, with
  what may also have contributed listed under the cause instead of folded away; the chart of the
  moment has taller lanes, the label of the moment above them and its zoom buttons beside its title;
  Sequence and a Technical block, whose switch shows the processes and Windows events, close the page.
  An overload period's page shows its numbers large and, under In the day, the day's States of the PC
  chart zoomed on 100 minutes around it, instead of its own resource chart. Toggle switches show their
  state after the switch, as in Windows.
- Now is titled Now, with its reading of the PC as the line under the title. The ten-minute chart has
  its live dot and zoom buttons beside its title, taller lanes with each value on the lane's line, a
  label every minute and its note under the chart. Programs using the most now can be sorted by
  processor, memory or disk, and its rows, like those of Programs, read as plain text that opens the
  program. Programs has a filter field and a sort list instead of sortable headers. All events
  (formerly Day events) is a table with the time, event, type, origin, length and severity of each,
  and History's weekday chart uses the bars of the 14-day chart.
- Today follows the new layout more closely: the date line also gives the hours covered, States of the
  PC opens with a divider and its zoom buttons, has taller rows and a wider legend, and its marks are
  thin lines. The day's list is titled Day events, each row shows how grave it was by its symbol
  (critical, overload or attention), and its second line no longer repeats the length. The Health
  block lists the open alerts themselves, critical first, with a link to Health, and the 14-day chart
  gives the total under it.
- The home screen's Activity card shows today's worst freeze, with its time and length, and opens its
  report; with none today it says so. The header band, its cards and the search sit closer together on
  short windows, and the cards narrow on windows under 1180 px.
- The window now has the Windows Settings look on every page: the navigation pane is a lighter band
  with the app's name at its top, the page sits on a flat plane instead of a framed card, page titles
  are light, links show no underline until pointed at, and buttons are plain Windows buttons. At 1500
  px and wider a Related column beside the page lists where to go next. Time charts zoom with − and +
  buttons and See all.
- The watcher writes its history every 5 minutes instead of every minute, which cuts what it writes
  to disk by about two thirds. Activity's Today tab and the overload period detail still show the
  latest minutes, read from the watcher's memory. The black box keeps the last 5 minutes instead of
  2, so a crash loses none of the minutes not written yet.
- Today and Now are one screen, Activity, with a tab for each. The side menu has one item for it,
  and links that pointed to Now open its tab. Switching tabs does not add a step to Back.

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
- "What to do" in reports, captures, Health findings and dumps is a numbered list of steps instead
  of boxes to tick.
- On Optimizations and Dumps the page holds still: the long list and the detail each scroll inside
  their own column, as on Tests, so the title and the action bar stay in view.
- A custom capture can run from one minute to 24 hours, set in hours and minutes, instead of at
  most 60 minutes. A long capture keeps a count of frame times rather than every frame, so a day
  of capturing stays light on memory; the numbers it reports are the same to a hundredth of a
  millisecond.
- On a Health finding, the link to Tests, Optimizations or Dumps moved out of "What to do" into a
  row at the foot of the card that says what that screen will show.
- A Health area, an occurrence report and a capture report open with an arrow before their name
  that goes back to the level above (Health, Today or Captures); Alt+← and the mouse's back button
  do the same. The breadcrumb they had before is gone.
- The side menu groups its screens by what you want to do: Follow (Today, Now), Look after the PC
  (Health, Tests, Optimizations) and Investigate (Dumps, Captures).
- The interface uses the words Windows uses: switches read On and Off (Ativado and Desativado in
  Portuguese), a program that starts by itself is a startup app, a tweak that needs an
  administrator says so, and Optimizations groups tweaks as Recommended for this PC, No sign on
  this PC and Nothing to change. Descriptions say what a setting does and leave its state to the
  Now and After rows.
- On Optimizations only the recommended tweaks can be chosen together; a tweak with no sign on
  this PC is applied on its own, from its detail. While some are chosen, the detail's button
  becomes a plain "Apply only this one", so one main Apply shows at a time.
- The shield marks every button that opens the Windows permission prompt, and only those: the full
  scan, reading and deleting dumps, applying or undoing a tweak that needs an administrator,
  scheduling the memory diagnostic, reading a whole disk and checking a volume. The permission
  reads the same everywhere, "Asks for administrator permission", and "Read only" has an eye of
  its own.
- The progress bar of a running test slides its fill instead of resizing it, so updating it no
  longer makes the page lay itself out again.
- A Health finding no longer draws a bar under its text.
- In Portuguese, the parts of a disk are called setores (sectors) everywhere, as the reports,
  SMART and Tests have them, and the map's squares are regions. The worn-disk finding has a
  shorter description, and the line under a finding no longer explains the button beside it; a
  test button only shows when no step already links to Tests.
- The disk map, in Tests and on a Health finding, now marks only what did not read, in red; every
  region that read is one plain colour, since a slow read is not a sign of wear. "Show the
  response time" brings back how long each region took, on a grey scale from quick to slow,
  with the slowest blocks and the note about SSDs, and the window remembers the choice. The
  labels under the map that marked the start, middle and end of the disk are gone.
- A disk's surface scan in Tests shows only the latest scan. "History" opens a dialog listing
  each scan of that disk, newest first, with its result, length and how much it covered; choosing
  one shows its map.
- A disk read that found sectors it could not read counts the sectors, "2 sectors could not be
  read", instead of the regions that hold them; when the read kept as many as it can, it says "at
  least".
- The interface speaks in a neutral, direct tone with the PC terms people know. Vestigium now
  monitors rather than watches; the PC states read Normal, High usage and Overloaded; the side
  menu groups are Monitoring, Maintenance and Diagnostics; Health counts alerts; and the tray,
  the banners and the error messages say what happened in a few words. Today and Now name CPU,
  GPU and overload periods, give the likely cause plainly, and call a fall of the PC an
  unexpected shutdown. Occurrence reports are reports, not "laudos" in Portuguese, and their
  causes, evidence, sequence and steps use the usual terms (BSOD, page file, DPC/ISR latency,
  surface scan, backup). Health names its findings as alerts, uses the SMART names
  (reallocated, pending, uncorrectable), WHEA, TDR and throttling, and says how many alerts
  each area has. Tests names its tests as they are known: SMART, Surface scan, File system
  (chkdsk), Speed test, Windows Memory Diagnostic, with "Requires administrator" and "Read-only".
  Optimizations describes each setting by what it does in Windows' own terms (kernel, PCIe
  link, Superfetch, minidump), states the downside in a few words, and labels its facts
  Current, After, Takes effect, Permission and Downside. Dumps gives stop codes and app errors
  their usual names (unhandled exception, access violation, IRQL, TDR, WHEA, heap corruption)
  and calls apps that closed by themselves app crashes. Captures speak of DPC/ISR latency, the CPU
  and slow frames, name Windows' own drivers by their usual names (Storport, NDIS, KMDF, xHCI),
  and keep their notes and steps short. Settings and the update dialogs follow the same tone; the
  categories read Reports, Monitoring and Data retention.

### Fixed

- Opening a page from a home tile no longer flashes a light grey card in the dark theme: the card that
  grows into the page keeps the colour the tile shows.
- A surface check that was stopped and then continued shows on the Tests screen as one map of the
  whole disk, as on the Health screen, instead of only the last part with the start marked as not
  read.
- The restore point the Optimizations screen asks for before a change now really stays. Windows
  logged it as created and then discarded it; Vestigium now asks for it the way Windows' own tools
  do, and says it was created only after finding it in Windows' list of restore points.
- Errors Windows logs for a disk that is not one of the PC's, such as a virtual disk that was
  attached for a while, no longer raise a critical "storage errors" alert on the Health screen.
- The file system check on the Tests screen no longer repairs anything: it runs
  `chkdsk /scan /forceofflinefix`, which leaves every repair to Windows. A volume with problems
  now reads "Problems found" instead of "Check failed", and a volume chkdsk could not check reads
  "Check failed" instead of "Problems found".
- Dates from an earlier year show the year, so a blue screen from May 2025 no longer reads as if it
  happened this May.
- In the day's list, an overload period caused by a Vestigium test shows that note on its own line
  instead of running on from the program's name.
- The Technical line of a memory overload period says "above 85%", the threshold Vestigium uses,
  instead of 90%.
- Lines that put a drive letter before a colon no longer read "C::", as in the step to free up
  space and the system disk line.
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
- Tests no longer calls a disk or the memory fine while Health has an open point about it: the
  device shows the Health point first, with a link to it. Steps that send you to Tests or Now end
  with a link that opens that screen.
- The window itself never scrolls any more: on Tests, Dumps and Optimizations a hidden label could
  push the page down and take the title bar and its buttons off screen.
- A recommended tweak has a mark of its own instead of Health's warning triangle, and a screen
  reader no longer hears two states in a row ("Attention Recommended") on Optimizations, Tests and
  the side menu.
- Counts on Health that named nothing ("7 in 30 days") now name what they count, and the crash
  dump setting reads "No dump" instead of a bare "None".

[Unreleased]: https://github.com/Filipe-Soares-de-Almeida/Vestigium/commits/main
