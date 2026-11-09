A latency capture names the driver or the app behind stutters, audio crackle and lag when a report alone cannot.

For a few minutes, a capture measures how long each driver holds the CPU (its DPC/ISR latency) and the frame time of every app on screen, while the monitor notes each stall of the PC. Then it lines the three up. No data leaves the PC.

## Start a capture

Open **Captures**, or select **Investigate stutters** in a report.

1. Pick a **Duration**: **2 min** for frequent stutters, **5 min** (recommended), **10 min** for rare ones, or **Custom**, from 1 minute to 24 hours. A long capture keeps only its summary.
2. Select **Start capture**. Windows asks for administrator rights once, for this capture only.
3. Keep doing what causes the stutters: the game, the video, the call.

Monitoring must be on. The capture goes on with the window closed, and its result shows in **Captures** when Vestigium opens again.

## The result

- **Conclusion**: the suspect driver or app, or **Nothing stood out**.
- **What to do**, such as updating the driver named.
- **Drivers**, ordered by how far each one's longest call went past the recommended limit of 0.1 ms. A driver that serves other drivers is marked, because the cause may be the device behind it.
- **Apps** that drew frames, with their slow frames.
- **Timeline**: PC stalls, long driver calls and slow frames on one time line, so you can see which came together.
- **Reports from these minutes**, when a freeze or stutter happened during the capture.

When nothing stood out, the PC may have returned to normal. **Capture for 10 minutes** or **Capture at the next stutter** gives it another chance.

## Past captures

**Past captures** lists every capture with its result. Run another one after updating a driver to see whether it helped.

**Copy result** puts a result on the clipboard as Markdown; see [Reporting a problem](./Reporting-a-problem.md).

## When drivers or frames are missing

Another tracing tool, such as LatencyMon, can keep the capture from measuring drivers, and an on-screen frame counter can keep it from measuring frames. The result says which part is missing. Close the other tool and capture again; see [Troubleshooting](./Troubleshooting.md#a-capture-failed).
