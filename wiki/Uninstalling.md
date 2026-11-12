How to remove Vestigium, and what stays on the PC afterwards.

## Before you uninstall

- Windows settings you changed in Optimizations stay changed after Vestigium is gone. To go back to the earlier values, open **Optimizations › Applied** and select **Undo** on each one first.
- To keep a report, copy it now with **Copy report**.

## Uninstall

1. Open the list of installed apps:
   - Windows 11: **Settings › Apps › Installed apps**, then **⋯** next to Vestigium, then **Uninstall**.
   - Windows 10: **Settings › Apps › Apps & features**, then Vestigium, then **Uninstall**.
2. Confirm. Vestigium's uninstaller opens.
3. Tick **Delete the application data** to remove the history too, or leave it unticked to keep it. Then select **Uninstall**.

Windows asks for an administrator's permission, because the program sits in `C:\Program Files\Vestigium`. If the window is open, the uninstaller offers to close it.

## What the uninstaller removes

- The monitor: it closes it first, so the tray icon goes away.
- The program files, in `C:\Program Files\Vestigium` unless you chose another folder.
- The Start menu and desktop shortcuts.
- The **Start with Windows** entry.

With **Delete the application data** ticked, it also removes these folders of the account you are signed in to:

| Folder | What it holds |
|---|---|
| `%LOCALAPPDATA%\Vestigium\data` | The history and the settings |
| `%LOCALAPPDATA%\app.vestigium` | The update preferences and the window's own files |

## What stays

With the box unticked, both folders stay, and installing Vestigium again picks up the same history and settings. To remove them later, delete both folders in File Explorer.

In every case, these stay:

- The history and settings of every other account on the PC, each in its own profile.
- Windows settings changed in Optimizations, until you change them back.
- Restore points Windows made before a tweak.

[Privacy and data](./Privacy-and-data.md) describes every file Vestigium keeps.
