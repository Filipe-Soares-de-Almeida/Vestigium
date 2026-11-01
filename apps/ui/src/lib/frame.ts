type UaData = { getHighEntropyValues(hints: string[]): Promise<{ platformVersion?: string }> };

/** Sets data-frame="win10" on the root when Windows 10 leaves the frameless window without a top edge. */
export async function markFrame(): Promise<void> {
  const ua = (navigator as Navigator & { userAgentData?: UaData }).userAgentData;
  if (!ua) return;
  try {
    const { platformVersion = "" } = await ua.getHighEntropyValues(["platformVersion"]);
    // Windows 11 reports platform version 13 and above; Windows 10 reports 1 to 10.
    const major = Number.parseInt(platformVersion, 10);
    if (major > 0 && major < 13) document.documentElement.dataset.frame = "win10";
  } catch {
    // Without the version the window keeps the edges Windows draws.
  }
}
