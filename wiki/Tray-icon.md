The Vestigium icon in the notification area: its menu, and what the badge in its corner means.

The icon belongs to the monitor, so it is there whenever Vestigium is monitoring or paused, with the window open or not. Windows may tuck it into the overflow behind the **^** arrow; [Troubleshooting](./Troubleshooting.md#the-tray-icon-is-missing) shows how to keep it in view.

## The menu

Click or right-click the icon.

| Item | What it does |
|---|---|
| **Open Vestigium** | Opens the window |
| **Pause monitoring** | Stops measuring until you select **Resume monitoring**, or until Vestigium starts again. For a pause that ends by itself, use **Settings › Monitoring** |
| **Sensitive mode (30 min)** | Checks the PC far more often for 30 minutes, to catch short stutters in games, audio and video. A tick shows while it is on |
| **Start with Windows** | Starts monitoring at sign-in. A tick shows while it is on |
| **Exit** | Stops monitoring and removes the icon. **Start Vestigium** in the window starts it again, and so does the next sign-in when **Start with Windows** is on |

## The badge

A small shape in the icon's bottom-right corner tells the state. Only one shows at a time, in this order: paused, then sensitive mode, then the last 24 hours.

| Badge | Meaning |
|---|---|
| None | Monitoring, and nothing to point out in the last 24 hours |
| Two bars, on a faded icon | Monitoring is paused |
| Dashed ring with a dot | Sensitive mode is on |
| Solid dot | A severe freeze, longer than 5 seconds, in the last 24 hours |
| Ring | Slowdowns in the last 24 hours, such as freezes or overload periods, or a critical alert open in Health |

## The tooltip

Point at the icon to read the same state in words:

- **Vestigium — monitoring**
- **Vestigium — paused**
- **Vestigium — sensitive mode on**
- **Vestigium — slowdowns in the last 24 h:** and how many
- **Vestigium — severe freezes in the last 24 h:** and how many
- **Vestigium — critical alerts in Health:** and how many

After stutters in sensitive mode, a second line adds **Stutters detected. Open Vestigium to investigate.** A [capture](./Captures.md) can name the cause.
