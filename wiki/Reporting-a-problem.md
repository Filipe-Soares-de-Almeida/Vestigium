How to share a report or a capture result, and where to ask for help.

Check the [FAQ](./FAQ.md) and [Troubleshooting](./Troubleshooting.md) first, and search the [existing issues](https://github.com/Filipe-Soares-de-Almeida/Vestigium/issues?q=is%3Aissue): someone may have asked already.

## Copy a report

1. Open the report of the freeze, stutter or shutdown, from **Activity › Today**, **All events** or **History**.
2. Select **Copy report**. Vestigium says **Report copied**.
3. Paste it where you write the issue.

The report goes to the clipboard as Markdown, so it keeps its headings and lists on GitHub. It holds the date and time, the kind and severity, each suspect with its signs and measured values, what happened in order, the Windows events of that moment, and the steps suggested. It is written in the window's language.

## Copy a capture result

In **Captures**, open the capture and select **Copy result**. Vestigium says **Result copied**. It holds the conclusion, the drivers by longest call, the apps with slow frames, the timeline and the steps.

## Read it before you post

Issues are public. A pasted report names the programs that were running, your disks and drivers, and the times. Read it and remove anything you want to keep private before you post.

Do not attach crash dump files to a public issue. A dump is a copy of memory and can hold personal data.

## Where to post

Open [New issue](https://github.com/Filipe-Soares-de-Almeida/Vestigium/issues/new/choose) and pick the form that fits:

| Form | Use it for |
|---|---|
| **Bug report** | Vestigium itself misbehaves: a crash, wrong data, a broken screen or a failed update |
| **Diagnosis help** | A freeze, stutter or crash, and a report that is wrong, empty or unclear about the cause. Paste the report here |
| **Feature request** | Something Vestigium should do, or do better |
| **Translation problem** | A wrong, unclear, missing or cut-off text in the window or the tray menu |
| **Accessibility barrier** | Something hard or impossible to use with your keyboard, screen reader, display settings or other assistive technology |

The forms ask for your Vestigium version, shown in **Settings › Updates › Installed version**, and your Windows version and build, shown by running `winver` from the Start menu.

- **A question** about using Vestigium: ask in [Discussions, Q&A](https://github.com/Filipe-Soares-de-Almeida/Vestigium/discussions/categories/q-a).
- **A security vulnerability**: report it privately from the [Security tab](https://github.com/Filipe-Soares-de-Almeida/Vestigium/security/advisories/new), never in a public issue.

To fix something yourself, read the [contributing guide](https://github.com/Filipe-Soares-de-Almeida/Vestigium/blob/dev/.github/CONTRIBUTING.md).
