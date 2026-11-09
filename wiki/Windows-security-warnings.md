Why Windows warns about Vestigium, what each warning means, and how to check that your download is the published file. Nothing on this page asks you to turn off a security feature.

## Why the warnings appear

Vestigium's 0.x releases ship without a code signature. Windows judges a new program by its signature and by how many people already run it, so a new, unsigned program gets warnings that a well-known signed one does not.

## Windows protected your PC

SmartScreen shows this blue window when you run the installer.

1. Select **More info**. The window now shows the file name and an unknown publisher.
2. Check that the file is `Vestigium_<version>_x64-setup.exe` and that you downloaded it from the [releases page](https://github.com/Filipe-Soares-de-Almeida/Vestigium/releases).
3. Select **Run anyway**.

Your browser may also say that the file is not commonly downloaded. Its download list offers to keep the file anyway; do so only for the file from the releases page.

## The administrator prompt

A few tasks ask for administrator rights, and their prompt shows **Publisher: Unknown** for the same reason. [Administrator prompts](./Administrator-prompts.md) lists those tasks.

## Smart App Control

On Windows 11, Smart App Control blocks programs that have no code signature, and it offers no way to run them anyway. While Vestigium's releases are unsigned, Vestigium cannot be installed on a PC where Smart App Control is on. Leave Smart App Control as it is: Vestigium never asks you to turn it off.

## Antivirus warnings

An antivirus can flag a new, unsigned program on suspicion alone. If yours flags Vestigium:

1. [Check the download](#check-the-download). If the hash does not match, delete the file and do not run it.
2. If it matches, report the detection to your antivirus maker as a false positive; most have a form for it.
3. Open a **Bug report** from [New issue](https://github.com/Filipe-Soares-de-Almeida/Vestigium/issues/new/choose), with the antivirus name, its version and the name of the detection.

Keep the antivirus on while you wait for its maker's answer.

## Check the download

Each release lists `SHA256SUMS.txt`, with the SHA-256 hash of every file. A file with the same hash is exactly the published one.

1. On the release page, open `SHA256SUMS.txt` and find the line for `Vestigium_<version>_x64-setup.exe`.
2. Open PowerShell from the Start menu and run this, changing the folder if you saved the file elsewhere:

   ```powershell
   Get-FileHash "$env:USERPROFILE\Downloads\Vestigium_*_x64-setup.exe" | Format-List
   ```

3. Compare the **Hash** it prints with the one in `SHA256SUMS.txt`. Upper and lower case do not matter.

If they differ, delete the file and download it again from the release page. If they still differ, do not run it, and open a **Bug report**.
