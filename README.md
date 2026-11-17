# Social Workspace · Rust preview

Your favorite websites in one desktop workspace.

![Social Workspace design preview](screenshot.png)

## Run on Windows

1. Install [Rust](https://rustup.rs/) and [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with **Desktop development with C++**.
2. Click **Code → Download ZIP**, then extract it.
3. Double-click **START-WINDOWS.bat**. The first build can take a few minutes.

Windows needs [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/). It is usually already installed.

You can also open a terminal in this folder and run:

```bat
cargo run --release
```

## What it does

- Opens websites in workspace tabs and separate windows.
- Adds platforms with a name, a website link or QR image, and an optional picture.
- Changes or removes any platform, including built-in ones.
- Saves platforms, pictures, settings, and browser data locally.
- Customizes workspace colors and text size.

## Preview status

This is the approved Rust design preview, replacing the Python version. The picture shows the interface with example open tabs, not signed-in accounts.

The Rust source passes a Windows-target compile check and 3 core tests on Linux. The interface preview passed navigation, search, tabs, text-size, and removal checks. **The Windows desktop app has not been run here.** Website-focus shortcuts, popup sign-in, downloads, notifications, and installer packaging still need verification or completion. Some sites may restrict embedded browsers.

The previous Python version remains in the repository history. See **LICENSES.txt** for notices.
