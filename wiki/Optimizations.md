A fixed set of Windows settings that can help with freezes and stutters, each recommended only when this PC shows a sign of it, and each undone in one click.

## The lists

| List | What it holds |
|---|---|
| **Recommended** | Tweaks with a sign on this PC that they would help |
| **No sign on this PC** | Tweaks that exist but have no sign here; applying one is optional |
| **No change** | Nothing to change: Windows already has the value, the setting is not available on this PC, or it belongs to a security product |
| **Applied** | Tweaks you applied, each with **Undo** |
| **All tweaks** | Everything above in one list |

The set covers fast startup, hibernation, the power plan, PCI Express power saving, USB selective suspend, program preloading, search indexing, Game Bar background recording, hardware-accelerated GPU scheduling, multiplane overlay, the apps that open at sign-in, and the dumps Windows writes for a blue screen or an app crash.

## A tweak

Each tweak shows:

- **What it does** and **What changes**: its **Current** value and the value **After**;
- **Why** it is recommended, with the sign found on this PC, or **Why it is not recommended**;
- **Takes effect**: immediately, at the next shutdown, at the next sign-in or after a restart;
- **Permission**: **No administrator rights needed** or **Requires administrator**;
- **Downside**: what you give up.

## Apply and undo

1. Select **Apply**, and confirm.
2. For a tweak that requires administrator rights, Windows asks for permission once and, if System Restore is on, makes a restore point before anything changes. See [Administrator prompts](./Administrator-prompts.md).
3. The previous value is saved, so **Undo** puts it back.

After a change, the tweak counts the same sign on each side of it, such as graphics driver restarts or stutters, and says **Fewer since the change**, **More since the change** or **No significant difference**. **History of this tweak** lists each change and undo.

## Never offered

Vestigium never changes security software, Microsoft Defender, the firewall, Windows Update, User Account Control, SmartScreen, memory integrity or the page file. A security product's startup entry appears only as **Security product: never suggested**.
