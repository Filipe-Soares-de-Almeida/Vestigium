What in the PC is a risk, area by area, and whether each alert is growing.

Vestigium checks the PC a minute after the monitor starts, then every hour, from what Windows logs and from its own readings. Most checks need no administrator rights.

## Areas

| Area | What it checks |
|---|---|
| **Disks** | SMART and NVMe health, disk errors and resets Windows logged, unreadable sectors found by a surface scan, file system errors, free space on the Windows disk, and whether Windows sits on an HDD |
| **Unexpected shutdowns** | Blue screens, shutdowns while in use, shutdowns that never finished, more failures than usual this week, and whether Windows can save a crash dump |
| **Memory** | Memory often nearly full, the page file and its headroom, and errors found by a memory test |
| **Startup** | How long the last startup took and what slowed it; needs a full scan |
| **Hardware and GPU** | Hardware errors (WHEA), graphics driver restarts and CPU throttling |
| **Protection** | Whether two antivirus programs are active at once |

Each area shows its worst mark: **Critical**, **Attention**, **No alert**, or **No data** when it needs a full scan. The **Disks** and **Memory** groups on Home show the same alerts beside the tests of each device.

## An alert

Open an alert to see:

- its value **When it opened** beside the **Last reading**, so you can tell whether it grew;
- the signs behind it;
- **Why it matters** and what to do, with links to the test, the tweak or the dumps that help.

An alert closes on its own when the readings return to normal, and shows as **Resolved**. Each area also lists its **Checks with no alert**, and the overview keeps a **History** of alerts opened and resolved and of full scans.

## Full scan

Some of what Windows keeps is open only to an administrator, such as the causes of a slow startup. **Run full scan** reads it after one [administrator prompt](./Administrator-prompts.md). The scan only reads; it changes nothing. Until one runs, **Startup** says **Requires a full scan**.

## Security software

Security software appears only as startup time. Vestigium never suggests turning a security feature off.
