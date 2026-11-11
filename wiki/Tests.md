Tests of the memory and of each disk, none of which changes your data. Each result says what it covered, and what was not tested shows as such.

**Tests** opens on an overview of the devices, the last test and the history. Pick **Memory** or a disk in the pane on the left. A running test shows its progress and can be stopped with **Cancel**; the PC stays usable meanwhile.

## Memory

| Test | What it does | Button |
|---|---|---|
| **Windows Memory Diagnostic** | Windows' own test, outside Windows at the next restart, for about 40 minutes. The most thorough and the slowest. Windows asks for administrator rights and then when to restart; the result shows here afterwards. | **Schedule the diagnostic** |
| **Quick memory test** | Tests the memory that is free right now, inside Windows, for 1, 5 or 15 minutes. It cannot reach memory in use or name the faulty module. | **Start test** |

An error in either test opens a critical alert in [Health](./Health.md). For a full test of every module, MemTest86 runs from a bootable USB stick for hours; Vestigium downloads nothing, and it is available at memtest86.com.

## Disks

Each disk has four tests.

| Test | What it does | Button |
|---|---|---|
| **SMART** | Reads the disk's own health counters, or the NVMe health log of an NVMe SSD. Reading an idle HDD spins it up for a few seconds, so it runs only when asked. | **Read the SMART** |
| **Surface scan** | Reads the whole disk without writing, finds unreadable sectors and can show each region's response time. It runs at low priority, can take a few hours, and goes on with the window closed. Asks for administrator rights. | **Scan the whole disk** |
| **File system** | Checks each NTFS volume with `chkdsk /scan`, in a few minutes. When it finds problems it marks the volume, and Windows repairs it at the next restart; a repair can remove files it cannot recover, so back up what matters first. The card says while a volume is marked. Asks for administrator rights. | **Check now** |
| **Speed test** | Measures read and write speed with a temporary file of about 1 GB, deleted at the end, in about 40 seconds. Another program using the disk lowers the values, and the result says so. | **Measure** |

A stopped surface scan can resume with **Continue from …** or begin again with **Start over**. A disk's earlier scans open from **History**, each with its map of the disk; **Show the response time** colours each region by how fast it read.

On an SSD, slow reads do not mean wear: what matters is unreadable sectors.

## Results

Each result says how much it covered, such as the share of memory tested or the part of the disk read. Problems found open alerts in [Health](./Health.md), and a later clean result closes them. The [Glossary](./Glossary.md) explains SMART, the NVMe health log, the surface scan and chkdsk.
