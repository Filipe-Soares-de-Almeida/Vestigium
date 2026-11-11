What Vestigium keeps, where it keeps it, what leaves the PC, and how to delete it.

## What leaves the PC

Only the requests listed below. There is no account and no telemetry.

- **The update check**, when on, asks GitHub for the newest version when the window opens, at most once a day. It sends no PC data; like any request on the internet, it reaches GitHub from your connection's address.
- **The downloads of the installer and of an update.** The installer downloads the WebView2 runtime when the PC lacks it, and an update downloads its installer after you select **Update and restart**. Vestigium sends no PC data with either.
- **Reading crash dumps** happens on the PC, with no internet connection.
- **Copy report** and **Copy result** put text on your clipboard. Where it goes next is up to you; see [Reporting a problem](./Reporting-a-problem.md).

## What Vestigium keeps

- Readings of the CPU, the memory, the disks and the GPU, once a second, and a summary of each minute.
- The busiest programs every few seconds: their names, and how much CPU, memory, disk and GPU each used.
- The Windows events that matter for freezes and crashes, read from Windows' own logs: disk errors, shutdowns, blue screens, crashes and hardware errors, with the details Windows logs for each, which can include the path of a program or file and so your user name.
- Reports, overload periods, Health alerts, test results, captures and what each dump said.
- The Windows settings changed in Optimizations, with their previous values.
- Your settings, and the PC's processor count, memory size and disk list.

**Settings › Data retention** sets how long each kind of data is kept. See [Settings](./Settings.md#data-retention).

## Where the data lives

| Path | What it is |
|---|---|
| `%LOCALAPPDATA%\Vestigium\data\vestigium.db` | The history and the settings. Two companion files, ending in `-wal` and `-shm`, sit beside it while Vestigium runs. |
| `%LOCALAPPDATA%\Vestigium\data\blackbox.ring` | The last few minutes of readings, kept on disk so a report after an unexpected shutdown can show what came before it. |
| The other folders in `%LOCALAPPDATA%\Vestigium\data` | Results of tests, captures, scans and administrator tasks, waiting to be read into the history. Each file is removed once read; a file that cannot be read stays for 30 days without change and is then deleted. |
| `%ProgramData%\Vestigium\tweaks` | A journal of the tweaks that needed administrator rights, written by the helper: one small file per change, with the settings it changed and their values before and after. Only an administrator can write in it. Vestigium reads it to tell a real change from a forged one when you select **Undo**. It holds no personal data, **Delete the history** does not remove it and uninstalling leaves it; an administrator can delete the folder, after which those changes can no longer be undone from Vestigium. |
| `%LOCALAPPDATA%\app.vestigium\updates.json` | Whether to check for updates, when it last checked, and the last version whose notes you saw. |
| `%LOCALAPPDATA%\app.vestigium\EBWebView` | The window's own browser files. |

**Settings › Privacy and data › Open the data folder** opens the first folder.

## Delete the history

In **Settings › Privacy and data**, select **Delete the history…**, then **Delete everything**. It deletes the timeline, freezes, reports, Health alerts, tests, captures, what the dumps said and the Windows events read. It cannot be undone.

Your settings stay, and so does the list of Windows settings changed in Optimizations, so each change can still be undone.

## Delete everything

Uninstall Vestigium with **Delete the application data** ticked, or delete both folders above after uninstalling. See [Uninstalling](./Uninstalling.md).
