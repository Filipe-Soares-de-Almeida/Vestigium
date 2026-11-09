**Note:** No release has been published yet.

How Vestigium checks for a newer version, installs it, and tells you what changed.

## The automatic check

The first time the window opens, it asks **Check for updates automatically?** With **Check automatically**, Vestigium asks GitHub for the newest version when the window opens, at most once a day. No PC data is sent, and nothing installs without your confirmation.

Change your answer in **Settings › Updates › Check for updates automatically**.

## Checking by hand

In **Settings › Updates**, select **Check now**. The answer is **Latest version installed.**, or the version on offer. If GitHub cannot be reached, see [The update check failed](./Troubleshooting.md#the-update-check-failed).

**Installed version** in the same place shows the version you have.

## Installing an update

When a newer version exists, a dialog says **Version … is available** and lists what changed in it.

1. Select **Update and restart**. The dialog shows the download as it goes; **Cancel** stops it.
2. The installer runs on its own, with a progress bar and no questions. It needs no administrator prompt.
3. It closes the window and the monitor, replaces the program, starts the monitor again if it was running, and reopens the window. Monitoring stops only while the files are replaced.

Your history and settings stay. Vestigium installs an update only if it carries the project's update signature, so a modified download is refused.

While a test or a capture is running, the button waits: **Wait for the test or capture in progress to finish.** **Not now** closes the dialog until the next check.

## After an update

The next time the window opens, it shows **What's new in …** once, with the notes of every version since the one you had. **Settings › Updates › What's new** lists the notes of every version. With the window in Portuguese, the notes appear in Portuguese where a translation exists.

## Updating by hand

Download the newer installer from the [latest release](https://github.com/Filipe-Soares-de-Almeida/Vestigium/releases/latest) and run it. It replaces the program and keeps your history and settings. The same [Windows security warnings](./Windows-security-warnings.md) apply.
