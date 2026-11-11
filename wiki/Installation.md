How to download Vestigium and install it on your PC.

## Requirements

- Windows 10 22H2 or Windows 11, 64-bit (x64).
- The Microsoft Edge WebView2 Runtime, which draws the window. Windows 11 includes it, and so does an up-to-date Windows 10. If it is missing, the installer downloads it, so keep the PC online while you install.
- An administrator's permission: Vestigium installs for every account on the PC, in `C:\Program Files\Vestigium`, so Windows asks for it. On an account that is not an administrator, Windows asks for an administrator's password. Only the install, an update and the uninstall ask; the monitor and the window run as your user.

The README lists the Windows versions Vestigium [has been tested on](https://github.com/Filipe-Soares-de-Almeida/Vestigium#tested-on).

If another account is signed in with Vestigium's monitor running, choose **Exit** on its tray icon in that session before you install, update or uninstall: the installer can close only the monitor of the session it runs in.

## Download

1. Open the [latest release](https://github.com/Filipe-Soares-de-Almeida/Vestigium/releases/latest).
2. Under **Assets**, download `Vestigium_<version>_x64-setup.exe`.

The release also lists `SHA256SUMS.txt`, which lets you [check the download](./Windows-security-warnings.md#check-the-download). The `.sig` and `latest.json` files serve the app's own update check; you do not need them.

## Install

1. Run the file you downloaded.
2. If Windows shows **Windows protected your PC**, select **More info**, then **Run anyway**. [Windows security warnings](./Windows-security-warnings.md) explains why it appears.
3. Windows asks for permission to install. Select **Yes**, or type an administrator's password. Then follow the installer's pages. It installs into `C:\Program Files\Vestigium` and adds **Vestigium** to the Start menu. A folder that any account can write is not a safe place for the program, so keep the default folder.
4. The last page offers to start Vestigium and to create a desktop shortcut. Vestigium starts as your user, not as the administrator who approved the install.

## If you used an earlier version

Version 0.1.0 installed for one user only, in `%LOCALAPPDATA%\Vestigium`. The new installer, or an update from that version, removes that copy: its programs, its Start menu and desktop shortcuts and its entry in the installed apps. Your history and settings stay in `%LOCALAPPDATA%\Vestigium\data`, and a **Start with Windows** entry carries over to the new copy. If the old copy's window is open, the installer closes it.

On Windows 11, a PC with Smart App Control turned on blocks the installer. See [Smart App Control](./Windows-security-warnings.md#smart-app-control).

## Next

[First run](./First-run.md) covers what to do once the window opens.
