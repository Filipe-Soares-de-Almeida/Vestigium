Which tasks ask Windows for administrator rights, and what the prompt shows.

## What runs without them

Monitoring and the window run as your user. The monitor never runs as administrator. These need no prompt either:

- **Start with Windows** and every setting;
- the quick memory test, reading a disk's SMART and the speed test;
- deleting the history;
- tweaks marked **No administrator rights needed**.

## Installing and updating

Vestigium installs for every account in `C:\Program Files\Vestigium`, so installing, updating and uninstalling it ask Windows for permission too. After that, the program is not something your own account can change.

## Tasks that ask

Some of what Windows keeps is open only to an administrator, such as crash dumps and the causes of a slow startup. For those tasks Vestigium starts a separate helper, and Windows asks you first. Each prompt covers one task: the helper does it and closes.

| Task | Where | Button |
|---|---|---|
| Latency capture | Captures | **Start capture** |
| Full scan of the startup | Health | **Run full scan** |
| Reading crash dumps | Dumps | **Read the dumps** |
| Deleting crash dumps | Dumps | **Delete selected**, **Delete this dump** |
| A tweak marked **Requires administrator** | Optimizations | **Apply**, **Undo** |
| Surface scan | Tests, on a disk | **Scan the whole disk** |
| File system check | Tests, on a disk | **Check now** |
| Windows Memory Diagnostic | Tests, on Memory | **Schedule the diagnostic** |

Windows Memory Diagnostic is Windows' own tool, so its prompt names Microsoft Windows as the publisher. Every other task opens Vestigium's helper.

## What the prompt shows

Vestigium's releases are unsigned, so Windows presents the helper as an app from an unknown publisher and shows **Publisher: Unknown**. Select **Yes** to run the task.

On an account that is not an administrator, Windows asks for an administrator's password instead. Vestigium is built for this case. The helper sits in the program folder, where a program running as your account cannot replace it or a file it loads, and the window does not start a helper that your account could replace. A program running as you, on a PC where you have just typed an administrator's password, therefore cannot ride on that prompt.

Two limits remain. The helper is unsigned, so the prompt cannot confirm who made it: approve a prompt only after you selected one of the buttons above yourself. And a copy from before this version, in `%LOCALAPPDATA%\Vestigium`, sits in a folder your account can write; update to remove it. [Updating](./Updating.md) and [Installation](./Installation.md) cover it.

If the prompt does not appear, look for it in the taskbar: Windows can open it behind other windows.

## Saying no

Select **No**, and nothing runs or changes. Vestigium says the permission was denied, and the rest of the app keeps working.

## Restore points

Before a tweak that requires administrator rights, Windows makes a restore point if System Restore is on. If it is off, Vestigium asks **Continue without a restore point?** Each tweak keeps its previous value either way, so **Undo** still works.
