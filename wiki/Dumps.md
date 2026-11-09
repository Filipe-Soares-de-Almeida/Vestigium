The files Windows keeps after a blue screen, a crash it recovered from or an app crash, read and explained in plain language.

## The lists

| List | What it holds |
|---|---|
| **Blue screens** | The dump Windows writes when it stops with a blue screen: a complete or kernel `MEMORY.DMP`, or a minidump |
| **Kernel reports** | Reports Windows writes when part of the system fails and recovers, such as a graphics driver that stopped responding |
| **Apps that crashed** | Dumps of programs that closed with an error |

The **Overview** shows how much space the dumps take and the **Windows dump setting**: whether Windows writes a dump on a blue screen, and which kind. If it writes none, a blue screen leaves nothing to read; [Optimizations](./Optimizations.md) can change that, and can turn on dumps for app crashes.

## Read the dumps

Windows keeps dumps in folders only an administrator can open. **Read the dumps** reads them all after one [administrator prompt](./Administrator-prompts.md). Reading happens on the PC, read-only, with no internet connection. **Read again** reads them once more, for example after a new crash.

Each dump then shows:

- the **Stop code** of a blue screen, or the error of an app, in plain words;
- the **Culprit**: the driver or module it most likely points at, and why;
- the **Call stack**: the modules from where it failed down to the Windows kernel;
- what to do, such as updating that driver.

A blue screen links to the report of the [unexpected shutdown](./Freeze-reports.md#unexpected-shutdowns) it caused.

## Delete dumps

A complete dump can take as much space as the PC's memory.

1. Tick the dumps you want to delete, or **Select all**.
2. Select **Delete selected**, and confirm. A single dump also has **Delete this dump**.
3. Windows asks for administrator rights.

What each dump said stays in Vestigium after the file is gone. Only dump files in the folders where Windows keeps them are deleted.
