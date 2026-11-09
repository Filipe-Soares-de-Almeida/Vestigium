Short answers to the questions people ask most. For something that does not work, see [Troubleshooting](./Troubleshooting.md).

## Does it slow my PC?

The project aims to keep the monitor as light as possible. It takes one reading a second and ranks the busiest programs every few seconds. Tests and scans run at low priority and only when you start them. Sensitive mode checks the PC far more often, so it turns itself off after 30 minutes.

## Does it send anything?

No. There is no account and no telemetry; what Vestigium keeps stays in one folder in your user profile. The only request that leaves the PC is the update check, and only if you allow it: it asks GitHub for the newest version and sends no PC data. Reports leave the PC only when you copy and paste them yourself. See [Privacy and data](./Privacy-and-data.md).

## Why does it ask for administrator rights?

Windows opens some data only to an administrator: crash dumps, the causes of a slow startup, the raw disk for a surface scan, and the driver timings a capture measures. Changing some Windows settings needs the same. Vestigium asks one task at a time, and the monitor itself never runs as administrator. [Administrator prompts](./Administrator-prompts.md) lists every task.

## Why didn't it catch my freeze?

The usual reasons:

- The monitor was not running, or was paused. Turn on **Start with Windows** so it is always there.
- The stall was a burst of short hitches. Those are caught only in sensitive mode: turn on **Sensitive mode (30 min)** from the tray before you play or call.
- Only one program stopped. That shows as **Program not responding** in **All events**, not as a freeze.

[No freezes are listed](./Troubleshooting.md#no-freezes-are-listed) goes through each case.

## Why is nothing recommended in Optimizations?

A tweak is recommended only when Vestigium saw a sign on this PC that it would help. With no sign, it stays in **No sign on this PC**, where each tweak says **Why it is not recommended**. You can still apply one; every change can be undone.

## Is it safe to delete dumps?

Yes. A dump is a copy of memory Windows saved for diagnosis, and Windows does not need it to run. Read the dumps first: what each one said stays in Vestigium after the file is deleted. Keep the file if a driver or app maker asks you for it.

## Does it change security settings?

No. Vestigium never changes security software, Microsoft Defender, the firewall, Windows Update, User Account Control, SmartScreen, memory integrity or the page file, and never suggests turning a security feature off.

## Does it run on Windows 11?

Yes, on Windows 11 x64 as on Windows 10 22H2. The README says which versions [it has been tested on](https://github.com/Filipe-Soares-de-Almeida/Vestigium#tested-on). While releases are unsigned, a PC with Smart App Control on cannot install it; see [Smart App Control](./Windows-security-warnings.md#smart-app-control).

## Can I run it on several PCs and compare them?

No. Vestigium monitors the one PC it is installed on. You can install it on each PC, and each keeps its own history; nothing is shared between them.
