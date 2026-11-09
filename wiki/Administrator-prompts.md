Which tasks ask Windows for administrator rights, and what the prompt shows.

## What runs without them

Installing, monitoring and the window run as your user. The monitor never runs as administrator. These need no prompt either:

- updates, **Start with Windows** and every setting;
- the quick memory test, reading a disk's SMART and the speed test;
- deleting the history;
- tweaks marked **No administrator rights needed**.

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

On an account that is not an administrator, Windows asks for an administrator's password instead.

If the prompt does not appear, look for it in the taskbar: Windows can open it behind other windows.

## Saying no

Select **No**, and nothing runs or changes. Vestigium says the permission was denied, and the rest of the app keeps working.

## Restore points

Before a tweak that requires administrator rights, Windows makes a restore point if System Restore is on. If it is off, Vestigium asks **Continue without a restore point?** Each tweak keeps its previous value either way, so **Undo** still works.
