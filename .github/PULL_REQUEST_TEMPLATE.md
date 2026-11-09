<!--
Target the dev branch. The title becomes the squash commit, so write it as
type(scope): subject, for example fix(collect): read the event log from the saved bookmark.
Types, scopes and the 50-character subject limit are in .github/CONTRIBUTING.md.
-->

## What

<!-- One paragraph, in behavior terms: what does this change do? -->

## Linked issue

<!-- Closes #123. For a small fix with no issue, write "None". -->

## How to test

<!--
The exact commands to run and what a correct result looks like. Numbers for anything
performance-related: a performance claim without a measurement is not a verification.
-->

## Breaking changes and migration

<!--
Anything a user would notice change: behavior, configuration keys, file formats, command names,
minimum requirements, and what they must do to move over. Write "None" if there is nothing.
-->

## Risk

<!-- What could break, and what is explicitly out of scope for this change. -->

---

- [ ] `CHANGELOG.md` has an entry under `## [Unreleased]`, or this change has no visible effect and I said so above (a maintainer adds `no-changelog`)
- [ ] The pull request title and the commits follow Conventional Commits
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` and `npm run check --prefix apps/ui` pass
- [ ] New or changed texts are in both `locales/en.json` and `locales/pt-BR.json`
- [ ] The description makes sense to someone who only has this repository
- [ ] No personal data, real captures or machine-specific paths in source, fixtures or this description
