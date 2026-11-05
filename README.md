<p align="center">
  <img src="apps/ui/src-tauri/icons/128x128@2x.png" alt="Vestigium logo, a red V cut like a gemstone" width="128" height="128">
</p>

<h1 align="center">Vestigium</h1>

<p align="center">
  <b>All-in-one freeze diagnostics for Windows PCs.</b>
</p>

<p align="center">
  <a href="https://github.com/Filipe-Soares-de-Almeida/Vestigium/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Filipe-Soares-de-Almeida/Vestigium/actions/workflows/ci.yml/badge.svg?branch=dev"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="Platform: Windows" src="https://img.shields.io/badge/platform-Windows-0078d6.svg">
  <img alt="Built with Rust and Tauri" src="https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri-b7410e.svg">
</p>

---

Vestigium is an all-in-one app for finding out why a Windows PC freezes,
stutters or crashes, and what to do about it. It turns each freeze into a report naming the
likely culprit with the evidence behind it, reads the crash dumps Windows keeps,
names the drivers that hold the system up, checks disks and hardware for
trouble, tests RAM and storage, and suggests the Windows settings worth
changing, each one reversible. A small agent monitors the PC in the background,
so the evidence is there when you need it.

It runs on Windows 10 22H2 and Windows 11 (x64), in English and Brazilian
Portuguese, for anyone chasing a freeze that is gone by the time Task Manager
opens. It monitors the one PC it is installed on.

> [!WARNING]
> Vestigium is in early development and there is no build to install yet.
> Built from source, everything on this page works: the background monitor,
> the tray icon and every screen of the window.

## Highlights

- **Every freeze gets a report, with evidence.** When the PC stops for a few
  seconds and comes back, or stutters in a game, a video or a call, the report
  says when it happened, how long it lasted, what was running and which cause
  the readings point to — a storage reset, the graphics driver, memory
  pressure, CPU throttling, one app. When nothing matches, it says so. An
  unexpected shutdown — a blue screen, the power button held down, a power cut —
  gets a report too, with what the PC was doing in the two minutes before.
- **The minutes before are already stored.** The agent keeps a rolling
  window of system load, the busiest programs and Windows events, so a report
  covers what led up to a stop of a fraction of a second or a full minute, and
  a sensitive mode also catches the short hitches that break games, audio and
  video. A daily timeline marks each freeze and each overload period,
  such as memory above 85% for twelve minutes and the program that held it.
- **Crash dumps, read for you.** It lists the blue-screen and app crash dumps
  Windows keeps, explains each one in plain language — the stop code and the
  driver or module most likely at fault — and deletes the ones you no longer
  need once you confirm.
- **The driver behind the lag.** A deep capture, from a minute to a whole day,
  measures each driver's DPC/ISR latency and the frame times of the
  apps on screen, so stutter and audio crackle point to a driver or an app by
  name. Each capture stays in a history, so a second one after a driver update
  shows whether it helped.
- **Health checks, tests and reversible fixes.** It flags what in the system is
  a risk — a failing disk, hardware errors, storage resets, throttling, a
  nearly full system drive — and shows whether each alert grew since it
  opened. It tests the memory and the disks without changing them, and each
  result says what it covered. When the evidence supports it, it suggests
  Windows settings to change, each applied and undone in one click; security
  features stay as they are.
- **Light, local, no account.** The agent runs as your user from the tray and
  the app exits when its window closes. Tasks that need administrator rights —
  the driver capture, dump cleanup, some health checks, fixes and tests — ask
  for them one at a time. No telemetry: everything it stores stays in one
  folder in your user profile, and the only request that leaves the PC is the
  update check, which the window asks about first. The project aims to keep the
  monitor as light as possible.

---

## The window

"Open Vestigium" in the tray menu opens it. The side menu groups the screens by what you want to
do, with Settings at the bottom beside whether Vestigium is monitoring. Every
screen shows in English or Brazilian Portuguese, light or dark, in one of four
colour themes, and at the detail level set in Settings: Simple, Detailed or
Technical.

### Monitoring

- **Activity, Today tab** — a sentence sums up the day, and a timeline of the PC's states
  since midnight (Normal, High usage, Overloaded, Suspended) marks each freeze. Point
  at a period to see what the CPU, memory, disk and GPU were
  doing, zoom with Ctrl and the mouse wheel, and click a freeze to open its
  report. Below come the day's freezes and overload periods, and the
  freezes of each of the last 14 days. An overload period opens in place: its
  peak and average, a chart of the period with the minutes around it, the
  programs that used the most during it, the freezes inside it and what to do.
- **Reports** — each freeze, app that stopped responding or unexpected shutdown
  opens as a report: how long it lasted, the probable cause with the evidence
  beside it, the steps to try, the sequence of what happened, and a chart of
  the minute before and the half minute after. "Copy report" puts it on the
  clipboard as Markdown, ready to paste into a forum or an issue.
- **Activity, Now tab** — what weighs on the PC at this moment: the last ten minutes of
  CPU, memory, disk and GPU, and the top programs by usage,
  each with all its processes added up. "See all" lists every program running,
  sorted by any column and searchable by name. Hold Ctrl to pause the screen on
  one second, as in Task Manager.

### Maintenance

- **Health** — the PC by area (disks, unexpected shutdowns, memory, startup,
  hardware and GPU, protection), checked every hour, each area with its
  worst mark. An area opens its alerts: the value now beside the one it had
  when it opened, why it matters and what to do. A full scan, after one
  administrator prompt, reads what Windows shows only to an administrator, such
  as what slows startup.
- **Tests** — the memory and each disk, checked without changing anything on
  them: the Windows Memory Diagnostic at the next restart, a quick test of the
  free memory for 1, 5 or 15 minutes, each disk's SMART (or NVMe health log), a
  read-only surface scan that finds unreadable sectors and can show each region's
  response time, a file system check (chkdsk) of each NTFS volume, and a short
  speed test. A disk's earlier scans open in a history dialog. A running test shows its
  progress and can be stopped.
- **Optimizations** — a fixed set of Windows settings, such as fast startup,
  hibernation, search indexing and the apps that open at sign-in, each with
  what it holds now. A tweak is recommended only when Vestigium saw a sign on
  this PC that it would help. Every change keeps the value from before, so it
  can be undone in one click, and shows whether it helped by the same sign
  counted before and after. Security software, Windows Update and the page file
  are never offered.

### Diagnostics

- **Dumps** — the files Windows keeps after a blue screen, a video reset it
  recovered from, or an app crash, grouped by kind with the
  space they take. "Read the dumps" explains each one on the PC, without the
  internet: the stop code in plain words and the driver or module it most
  likely points at. Chosen dumps are deleted after a confirmation, and what
  each one said stays.
- **Captures** — "Investigate stutters" measures every driver and app for 2, 5
  or 10 minutes, or any length up to 24 hours, and names the driver or app
  behind the stutters. Each capture stays in a history.

### Settings

Appearance, the default detail level, monitoring (start with Windows, sensitive
mode, pause), how long each kind of data is kept, the language, updates
(checking on its own, "Check now" and what changed in each version), and privacy
and data: the space the history takes, a button that opens its folder and one
that deletes it.

### Updates

The first time the window opens it asks whether to check for updates on its own.
With yes, it asks GitHub for the newest version when it opens, at most once a
day; nothing Vestigium keeps goes along. A newer version opens a dialog with
what changed in it, and "Update and restart" downloads and installs it, closing
and reopening the window while monitoring goes on. After an update, the window
shows once what changed since the version you had.

## The tray

The Vestigium icon sits in the notification area. Its menu opens the window,
pauses monitoring, turns on sensitive mode for 30 minutes, starts Vestigium with
Windows, and exits. A badge in the icon's corner tells the state by its shape:
a ring when the last 24 hours held slowdowns or a critical Health alert is
open, a solid dot after a severe freeze, two bars while monitoring is paused, and
a dashed ring while sensitive mode is on. The tooltip says the same in words.

## Where the data lives

| Path | What it is |
|---|---|
| `%LOCALAPPDATA%\Vestigium\data\vestigium.db` | The history and the settings: the timeline, reports, Health alerts, tests, captures, dumps read and tweaks changed. |
| `%LOCALAPPDATA%\Vestigium\data\blackbox.ring` | The last seconds before an unexpected shutdown, read at the next start. |
| The other folders in `%LOCALAPPDATA%\Vestigium\data` | Results of administrator tasks, read into the history. |
| `%LOCALAPPDATA%\app.vestigium\updates.json` | Whether to check for updates, when it last checked and the last version whose notes you saw. |

Settings › Privacy and data opens this folder, and deletes the history after a
confirmation.

---

## Tested on

| System | Status |
|---|---|
| Windows 10 22H2, x64 | Tested |
| Windows 11, x64 | Not yet tested |

## License

Vestigium is released under the [MIT License](LICENSE).
