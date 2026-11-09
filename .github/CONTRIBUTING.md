# Contributing to Vestigium

Thank you for helping. Vestigium finds out why a Windows PC freezes, stutters or crashes. This page covers
how to report a problem, how to send a change and what a change has to satisfy to be merged. Everyone
taking part agrees to the [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to help

- **A freeze Vestigium did not explain, or a report that looks wrong.** Open a *Diagnosis help* issue and
  paste the report: in the app, open the freeze and select **Copy report**. The wiki explains
  [how to report a problem](https://github.com/Filipe-Soares-de-Almeida/Vestigium/wiki/Reporting-a-problem)
  and [what a freeze report contains](https://github.com/Filipe-Soares-de-Almeida/Vestigium/wiki/Freeze-reports).
- **A bug, an idea or an accessibility barrier.** Use the matching form in
  [New issue](https://github.com/Filipe-Soares-de-Almeida/Vestigium/issues/new/choose).
- **A wrong or unclear text.** Every text of the window and the tray menu is in
  [`locales/en.json`](../locales/en.json) and [`locales/pt-BR.json`](../locales/pt-BR.json). Fix it there and
  open a pull request, or use the *Translation problem* form.
- **Code and documentation.** Issues labelled `good first issue` and `help wanted` are the best place to
  start. For anything larger than a small fix, open an issue first so the approach can be agreed before you
  write it.
- **A security problem.** Do not open an issue. Follow [SECURITY.md](SECURITY.md).

Questions are welcome in
[Discussions](https://github.com/Filipe-Soares-de-Almeida/Vestigium/discussions).

## Build and test

You need Windows 10 22H2 or Windows 11 (x64), [Rust](https://rustup.rs/) with the MSVC toolchain and the
Visual Studio C++ build tools, Node.js 24 with npm, and the WebView2 runtime. The repository pins the Rust
toolchain in `rust-toolchain.toml`.

These are the commands the CI runs, in order. A pull request must pass all of them:

```powershell
npm ci --prefix apps/ui
npm run check --prefix apps/ui
npm run build --prefix apps/ui
rustup toolchain install
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p vestigium-collect --test events --locked -- --ignored
cargo build --workspace --release --locked
```

The `--ignored` test reads the real Windows event log, so it runs only on Windows.

For work on the window, `npm run dev --prefix apps/ui` serves it in a browser with sample data and no
watcher. To try the whole app, `powershell -ExecutionPolicy Bypass -File tools/bundle.ps1` builds an
installer under `target/release/bundle/nsis`.

## Branches and pull requests

1. Fork the repository and branch from `dev`: `<type>/<short-slug>`, for example `fix/hang-probe-lag`.
2. Open the pull request **into `dev`**. The `main` branch only receives releases, prepared by the
   maintainer.
3. One pull request covers one concern. A change that touches three modules for three reasons is three
   pull requests.
4. Fill in the pull request template. Link the issue it closes.
5. Pull requests are squash-merged, so the pull request title becomes the commit message on `dev`.

A maintainer approves the first workflow run of a new contributor. Reviews can take a few days.

## Commit messages and pull request titles

The format is [Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>): <subject>`.

- **Types:** `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `build`, `chore`, `spike`.
- **Scopes:** `core`, `collect`, `store`, `ipc`, `agent`, `helper`, `ui`, `ci`, `project` (documents), and
  `deps` (dependency updates).
- **Subject:** imperative, lowercase, no trailing period, 50 characters or fewer. For example
  `fix(collect): read the event log from the saved bookmark`.
- Add a body, wrapped at 72 columns, only when the reason is not clear from the diff.
- One logical change per commit, and formatting changes never ride along with behavior.
- A `perf` commit states the measured before and after in its body. A performance claim with no number is
  not a `perf` commit.

A check on every pull request reads the title against these rules.

## The changelog

Every change that a user can notice adds an entry to [`CHANGELOG.md`](../CHANGELOG.md) under
`## [Unreleased]`, in the same pull request. Use the headings `Added`, `Changed`, `Deprecated`, `Removed`,
`Fixed` and `Security`, in that order, and write for a person using Vestigium, not for the diff:

- Yes: "Notifications now appear once per event instead of repeating while the text is on screen."
- No: "Refactored `NotificationRecognizer` to use `ChangeGate`."

A change with no visible effect, such as a pure refactor, needs no entry. Say so in the pull request and a
maintainer adds the `no-changelog` label, which turns the changelog check off for it.

## Language and translations

- Everything in the repository is in English: source, identifiers, comments, commit messages,
  documentation, log output, test names and branch names.
- Text the user sees lives only in `locales/en.json` and `locales/pt-BR.json`, never inline in source.
  Brazilian Portuguese appears only in `locales/pt-BR.json`. Add a key to both files; `cargo test -p
  vestigium-i18n` fails when the two catalogs differ.
- The release notes are translated by the maintainer when a version is released. You do not need to
  translate changelog entries.
- To propose another language, open an issue first: it needs code changes as well as a catalog.

## Rules the code follows

These four rules are part of how Vestigium measures a PC without disturbing it. A pull request that breaks
one is not merged.

1. **`timeBeginPeriod` is never called.** It raises the timer resolution for the whole machine. The hang
   probe times itself with a high-resolution waitable timer (`CREATE_WAITABLE_TIMER_HIGH_RESOLUTION`).
2. **Performance counters are added with `PdhAddEnglishCounterW` only.** Counter names are English on every
   Windows language. `PdhAddCounterW` and `PdhAddCounterA` are not used.
3. **Event tracing runs in a session the project names and owns.** The global `NT Kernel Logger` session is
   never used.
4. **No WMI and no PowerShell in the collection path** (`crates/collect/` and `apps/agent/`): no `IWbem*`
   interface, no `wmi` crate, no `Win32_System_Wmi` feature and no `powershell` or `pwsh` process.

A few more rules shape the design:

- The monitor runs as the user and opens no network connection. The helper (`apps/helper/`) is the only
  elevated program; it runs one job from a closed list and exits.
- A Windows setting change records the previous value first, so that undo restores exactly that value.
  Security software, UAC, Windows Update and the page file are never changed.

## Code style

- A comment is two lines at most and says what the code does, never what changed. No commented-out code.
- A `TODO` points to an open issue.
- Names, fixtures and sample data are invented. Never commit a real capture from a PC: crash dumps, ETW
  traces, event logs or recorded databases. Screenshots must not show a real desktop.

## Licence

Vestigium is released under the [MIT License](../LICENSE). By opening a pull request you license your
contribution under the same terms.
