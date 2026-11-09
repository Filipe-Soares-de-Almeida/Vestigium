Common problems, each with the symptom, the cause and the fix. If yours is not here, see [Reporting a problem](./Reporting-a-problem.md).

## The tray icon is missing

**Symptom:** no Vestigium icon beside the clock.

**Cause:** Windows hid it in the overflow, or the monitor is not running.

**Fix:**

- Select the **^** arrow beside the clock. If the icon is there, drag it onto the taskbar to keep it in view.
- To show it for good: on Windows 11, **Settings › Personalization › Taskbar › Other system tray icons**, and turn Vestigium on; on Windows 10, **Settings › Personalization › Taskbar › Select which icons appear on the taskbar**.
- If it is not in the overflow either, open **Vestigium** from the Start menu. If the window says **Vestigium is not running.**, select **Start Vestigium**.

## Monitoring is paused

**Symptom:** the window says **Monitoring paused.**, the icon is faded with two bars, and its tooltip says **Vestigium — paused**.

**Cause:** a pause from the tray menu or **Settings › Monitoring**. Nothing is measured during a pause, so a freeze in that time has no report.

**Fix:** select **Resume** in the window, **Resume monitoring** in the tray menu, or **Resume now** in **Settings › Monitoring**. A pause with a length ends by itself.

## Monitoring does not start after a restart

**Symptom:** after a restart or a new sign-in there is no tray icon, and the window says **Vestigium is not running.**

**Cause:** **Start with Windows** is off, which is how Vestigium is installed, or Task Manager disabled its startup entry.

**Fix:** select **Start Vestigium**, then turn on **Start with Windows** in the tray menu or in **Settings › Monitoring**. Turning it on in Vestigium also clears a **Disabled** mark left in Task Manager.

## No freezes are listed

**Symptom:** the PC froze, but **Activity › Today** shows nothing at that time.

**Cause:** one of these:

- The monitor was not running, or was paused. Point at that time on the timeline: it says whether monitoring was paused or stopped.
- The stall was shorter than a quarter of a second, or a burst of short hitches. Those count only in sensitive mode.
- Only one program stopped responding. That appears as **Program not responding** in **All events**.
- It happened on another day, or longer ago than **Settings › Data retention** keeps.

**Fix:** keep **Start with Windows** on. Before a game or a call where stutters happen, turn on **Sensitive mode (30 min)** from the tray. Look for older days in **History**.

## A capture failed

**Symptom:** **Capture not started**, **Capture interrupted**, or a result that says **Drivers not measured** or **Frames not measured on this PC**.

**Cause:** one of these:

- **No** was selected at the administrator prompt, so nothing was measured.
- Monitoring was off; a capture needs it on.
- Another tool tracing the system, such as LatencyMon, kept the capture from measuring drivers. An on-screen frame counter or another tool tracing frames can keep it from measuring frames.
- The PC restarted, or the helper stopped, before the capture saved its result.

**Fix:** close the other tracing tools, make sure monitoring is on, and start the capture again, selecting **Yes** at the prompt. A capture that was interrupted changed nothing on the PC.

## The update check failed

**Symptom:** **Settings › Updates** says **Could not connect to GitHub. Check the connection and try again.**, or an update stops with **Download did not complete. Check the connection and try again.**

**Cause:** no internet connection, or a firewall or proxy blocks GitHub. A failed automatic check stays quiet and tries again the next time the window opens.

**Fix:** check the connection, then select **Try again**. If it keeps failing, download the installer from the [latest release](https://github.com/Filipe-Soares-de-Almeida/Vestigium/releases/latest) and run it; see [Updating](./Updating.md#updating-by-hand).

## The window opens blank

**Symptom:** the window opens but stays empty.

**Cause:** the Microsoft Edge WebView2 Runtime, which draws the window, is missing or damaged. The monitor does not use it and keeps running.

**Fix:** install the Evergreen WebView2 Runtime from [Microsoft's WebView2 page](https://developer.microsoft.com/microsoft-edge/webview2/), or repair **Microsoft Edge WebView2 Runtime** from the list of installed apps with **Modify**. Then open Vestigium again.
