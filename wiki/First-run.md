What happens the first time Vestigium opens, and the three things worth setting.

Vestigium has two parts. The **monitor** runs in the background, with an icon in the notification area, and keeps the evidence. The **window** shows what the monitor found; you can close it at any time.

## Start monitoring

The installer's last page opens the window, and so does **Vestigium** in the Start menu. The first time the window opens, it starts the monitor by itself: the Vestigium icon appears in the notification area, and the window says **Monitoring started**.

**Activity › Today** then shows when monitoring started. Freezes appear there from that moment on, each with the data of the minute before.

## Answer the update question

The first time the window opens, it asks **Check for updates automatically?**

- **Check automatically**: when the window opens, at most once a day, Vestigium asks GitHub whether a newer version exists. No PC data is sent, and nothing installs until you confirm.
- **Not now**: no automatic check. You can turn it on later in **Settings › Updates**.

[Updating](./Updating.md) describes the rest.

## Turn on Start with Windows

Vestigium does not start with Windows until you ask it to. Until then, monitoring stops when you sign out or restart, and starts again only when you open Vestigium and select **Start Vestigium** in the banner at the top of the window.

Turn it on with **Start with Windows** in the tray menu, or in **Settings › Monitoring**. It needs no administrator rights.

## Choose a detail level

Every screen speaks at one of three levels. Set it in **Settings › Reports › Detail level**.

| Level | What it shows |
|---|---|
| **Simple** (default) | Summary and actions |
| **Detailed** | With measured values |
| **Technical** | Counters, events and IDs |

## The first day

- Health checks the PC a minute after the monitor starts, then every hour. It also reads the last 30 days of Windows' own logs, so an alert about earlier shutdowns or disk errors can appear on the first day.
- Activity fills in from the moment the monitor starts.
- Closing the window leaves monitoring running from the tray. **Exit** in the tray menu stops it.

Next: [The window](./The-window.md) and [Tray icon](./Tray-icon.md).
