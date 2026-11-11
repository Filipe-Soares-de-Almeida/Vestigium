# Security policy

## Reporting a vulnerability

Report a vulnerability privately. Do not open a public issue or discussion for it.

Open the [Security tab](https://github.com/Filipe-Soares-de-Almeida/Vestigium/security/advisories/new) and
choose **Report a vulnerability**. Only the maintainer can see the report, and you are credited on the
advisory if a fix is published.

Please include:

- the Vestigium version (Settings › Updates) and your Windows version and build,
- what an attacker can do, and what they need first: a local account, administrator rights or a network
  position,
- the steps to reproduce it, or a proof of concept,
- how you want to be credited, if at all.

Send no crash dumps, event logs or captures from a PC that is not yours.

## What to expect

- You get an acknowledgement within 7 days.
- Within 14 days you get an assessment: whether it is accepted, how severe it is and what happens next.
- A fix is released as soon as it is ready and tested. The advisory is published with the fix. We aim to make
  the details public no later than 90 days after your report, unless we agree on another date.

## Supported versions

Vestigium is before 1.0. Only the latest release receives security fixes, and the app offers it through its
update check.

| Version | Supported |
|---|---|
| The latest release | Yes |
| Older releases | No: update to the latest |

## Scope

Vestigium runs as your user, with an elevated helper that Windows asks you to approve each time it is needed.
It is installed for the whole PC, in `C:\Program Files\Vestigium`, where only an administrator can change it.
These parts matter most:

- **The elevated helper.** It runs a fixed list of jobs, takes its input from files in Vestigium's data folder
  and checks every path against the job it is running. Dump deletion, for example, touches only `.dmp` files
  in the folders where Windows keeps them. Anything that makes it run an arbitrary command, act on a path a job
  does not allow, follow a link out of the allowed folders or change a setting it promises never to touch is in
  scope, and is the most serious kind of report. So is a way for a program running as a standard account, on a PC
  where the prompt asks for an administrator's password, to replace the helper or a file it loads before it runs
  elevated.
- **The channel between the monitor and the window.** It is a named pipe open only to your own account and
  checked on both ends. A way for another account or program to use or impersonate it is in scope.
- **Updates.** The window checks GitHub for a signed release and installs it only after you confirm. A way to
  install an unsigned or modified build, or to tamper with the update feed, is in scope.
- **The installer and uninstaller.**
- **Your data.** Everything Vestigium stores is in a folder in your user profile, and the only outside
  connection is the update check. A connection anywhere else, or a way to read or change that data from another
  account, is in scope. The wiki describes
  [what is stored and where](https://github.com/Filipe-Soares-de-Almeida/Vestigium/wiki/Privacy-and-data).

Out of scope:

- An attacker who already runs code as an administrator on the PC. Code running as your user is in scope only
  where it gains administrator rights through the helper, the installer or an update; what your own account can
  do by itself, such as reading its data folder or stopping its monitor, is not a vulnerability.
- Physical access to the PC, and attacks on firmware or drivers.
- Flaws in Windows, the WebView2 runtime or PresentMon, which ships with Vestigium. Report those upstream, and
  tell us if a version we ship is affected.
- Missing hardening with no demonstrated way to exploit it, and automated scanner output without an impact.
- Social engineering, and a user stopping their own monitor.

## Limits today

- Releases are not code-signed. The administrator prompt shows **Publisher: Unknown** and cannot tell you that
  the file is Vestigium's, so approve it only for a task you started yourself in the window. Code signing is
  planned.
- Versions before the per-machine install put the program in `%LOCALAPPDATA%\Vestigium`, a folder the user can
  write, and their helper can be replaced from that account. Update to the latest release, which removes that
  copy for the account that is signed in; an account that is not signed in during the update keeps its old copy
  until it is removed there.
- The installer and the update run elevated. An update is installed only if it carries the project's update
  signature; a download you run by hand is protected only by the hash you check against the release.

Good-faith research on a PC you own is welcome. Please do not access data that is not yours.
