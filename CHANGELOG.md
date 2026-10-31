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

- A tray icon for the background recorder, with options to pause it, start it with Windows, and
  exit. Only one recorder runs per user; starting it again while it runs does nothing.

[Unreleased]: https://github.com/Filipe-Soares-de-Almeida/WatchMyPC/commits/main
