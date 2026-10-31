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
- The background watcher samples CPU, per-core DPC and interrupt time, memory, physical disks and
  the busiest processes every second, checks for system stalls ten times a second, and keeps the
  history in a local database. It needs no administrator rights, opens no network connection, and
  saves what it has before Windows signs you out or shuts down.
- A first application window, opened from the tray icon. Only one window opens at a time, and
  closing it ends the program completely: nothing keeps running in the background.

### Changed

- The project is now called Vestigium, and its GitHub repository was renamed to match. Links to
  the old addresses redirect.
- The background watcher now runs as `Vestigium.exe` and the window as `Vestigium.Window.exe`.
  Task Manager and the startup apps list show the watcher as Vestigium, with its icon, instead of
  a bare file name.
- The background watcher saves its history once a minute, compressed, and hands memory it only
  needed at startup back to Windows: about 30 MB of disk writes a day and 5 MB of memory in use,
  measured on a typical desktop.

[Unreleased]: https://github.com/Filipe-Soares-de-Almeida/Vestigium/commits/main
