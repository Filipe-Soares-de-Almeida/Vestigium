<h1 align="center">Vestigium</h1>

<p align="center">
  <b>All-in-one freeze diagnostics for Windows PCs.</b><br>
  A tray agent in Rust, a Tauri app, one SQLite file. Windows 10 and 11.
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
changing, each one reversible. A small agent watches the PC in the background,
so the evidence is there when you need it.

It runs on Windows 10 22H2 and Windows 11 (x64), in English and Brazilian
Portuguese, for anyone chasing a freeze that is gone by the time Task Manager
opens. It watches the one PC it is installed on.

> [!WARNING]
> Vestigium is in early development. There is no build to install yet.

## Highlights

- **Every freeze gets a report, with evidence.** When the PC stops for a few
  seconds and comes back, or stutters in a game, a video or a call, the report
  says when it happened, how long it lasted, what was running and which cause
  the readings point to — a storage reset, the graphics driver, memory
  pressure, CPU throttling, one app. When nothing matches, it says so.
- **The minutes before are already stored.** The agent keeps a rolling
  window of system load, the busiest processes and Windows events, so a report
  covers what led up to a stop of a fraction of a second or a full minute, and
  a sensitive mode also catches the short hitches that break games, audio and
  video. A daily timeline marks each freeze and each long stretch of pressure,
  such as memory above 90% for twelve minutes and the process that held it.
- **Crash dumps, read for you.** It lists the blue-screen and app crash dumps
  Windows keeps, explains each one in plain language — the stop code and the
  driver or module most likely at fault — and deletes the ones you no longer
  need once you confirm.
- **The driver behind the lag.** A deeper capture measures how long each driver
  holds the processor and the frame times of the app in front, so stutter and
  audio crackle point to a driver or an app by name.
- **Health checks, tests and reversible fixes.** It flags what in the system is
  a risk — a failing disk, hardware errors, storage resets, throttling, a
  nearly full system drive — and runs RAM and disk tests. When the evidence
  supports it, it suggests Windows settings to change, each applied and undone
  in one click; security features stay as they are.
- **Light, local, no account.** The agent runs as your user from the tray and
  the app exits when its window closes. Tasks that need administrator rights —
  the driver capture, dump cleanup, some health checks, fixes and tests — ask
  for them one at a time. No telemetry: everything it stores stays in one file
  in your user profile. The project aims to keep the watcher as light as
  possible.

---

## Tested on

| System | Status |
|---|---|
| Windows 10 22H2, x64 | Tested |
| Windows 11, x64 | Not yet tested |

## License

Vestigium is released under the [MIT License](LICENSE).
