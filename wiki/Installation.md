How to download Vestigium and install it for your Windows user.

## Requirements

- Windows 10 22H2 or Windows 11, 64-bit (x64).
- The Microsoft Edge WebView2 Runtime, which draws the window. Windows 11 includes it, and so does an up-to-date Windows 10. If it is missing, the installer downloads it, so keep the PC online while you install.
- No administrator rights: Vestigium installs for your user only.

The README lists the Windows versions Vestigium [has been tested on](https://github.com/Filipe-Soares-de-Almeida/Vestigium#tested-on).

## Download

1. Open the [latest release](https://github.com/Filipe-Soares-de-Almeida/Vestigium/releases/latest).
2. Under **Assets**, download `Vestigium_<version>_x64-setup.exe`.

The release also lists `SHA256SUMS.txt`, which lets you [check the download](./Windows-security-warnings.md#check-the-download). The `.sig` and `latest.json` files serve the app's own update check; you do not need them.

## Install

1. Run the file you downloaded.
2. If Windows shows **Windows protected your PC**, select **More info**, then **Run anyway**. [Windows security warnings](./Windows-security-warnings.md) explains why it appears.
3. Follow the installer's pages. It installs into `%LOCALAPPDATA%\Vestigium`, with no administrator prompt, and adds **Vestigium** to the Start menu.
4. The last page offers to start Vestigium and to create a desktop shortcut.

On Windows 11, a PC with Smart App Control turned on blocks the installer. See [Smart App Control](./Windows-security-warnings.md#smart-app-control).

## Next

[First run](./First-run.md) covers what to do once the window opens.
