What a report says about a freeze, a stutter, a program that stopped responding or an unexpected shutdown, and how to share it.

## What gets a report

| Kind | When |
|---|---|
| **Freeze** | The whole PC stopped responding for at least a quarter of a second. Under 1 second it is light, up to 5 seconds medium, longer severe. |
| **Stutters** | Several short hitches in a row, the kind that break a game, a call or a video. Caught while sensitive mode is on. |
| **Program not responding** | One program's window stopped responding while the rest of the PC worked. Windows marks a window this way after 5 seconds. |
| **Windows event** | Windows logged a problem by itself, such as resetting its connection to a disk, retrying a disk operation, restarting the graphics driver or a hardware error. |
| **Unexpected shutdown** | The PC went off without shutting down properly: a blue screen, the power button held down, a power cut, or a failure during sleep. |

Open a report from the marks on **Activity › Today**, from **Day events**, **All events** or a day in **History**, or from the **Activity** card on Home.

## Reading a report

- **The headline** says what happened and how long it lasted, with its **Severity**, **Duration** and **Where**: a disk, a program or the whole PC.
- **Probable cause** names what the readings point to, with its confidence (high, medium or low) and the signs behind it. Possible causes include a disk and its controller, a disk overloaded by a program, low memory, a CPU maxed out, a driver, the graphics driver, power or thermal throttling, and one program on its own. When no resource was overloaded, it says **Cause not identified**.
- **May also have contributed** lists other suspects, each with its signs.
- **What to do** gives the steps to try, most likely first.
- **Sequence** lists, in order and with the time of each, what happened before, during and after.
- **Minute before, moment and after** charts CPU, memory, disk, latency, GPU and response delay, from a minute before to half a minute after.

At the **Technical** detail level, **Show processes and Windows events** adds the programs' figures and the Windows events of that window of time.

## When a driver is the suspect

If a report points at a driver or at stutters, it offers **Investigate stutters**. A [capture](./Captures.md) of a few minutes, started while the problem happens, names the driver or the app.

## Unexpected shutdowns

The monitor keeps its last few minutes on disk, so a report after an unexpected shutdown shows what the PC was doing in the two minutes before it went off. The headline says which kind it was: a blue screen, the PC turned off with the power button while not responding, a shutdown during sleep or wake, a power cut during shutdown, or a shutdown with nothing logged. When Windows saved a crash dump, the report links to what [Dumps](./Dumps.md) read from it.

## Copy report

**Copy report** puts the report on the clipboard as Markdown, ready to paste into a forum or an issue. [Reporting a problem](./Reporting-a-problem.md) explains what it contains and where to post it.
