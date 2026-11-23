# Social Workspace · Rust preview

Your favorite websites in one desktop workspace.

![Social Workspace design preview](screenshot.png)

## Download for Windows

**[Download SocialWorkspace.exe](https://github.com/nge9/program/releases/download/windows-preview-1/SocialWorkspace.exe)**

Or get the **[ZIP with instructions and licenses](https://github.com/nge9/program/releases/download/windows-preview-1/SocialWorkspace-Windows.zip)**.

Download and double-click the executable. **You do not need Rust or Visual Studio.** This is an unsigned Windows x64 preview.

It requires [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/), which is usually already installed on Windows.

<details>
<summary>Build from source instead</summary>

Install [Rust](https://rustup.rs/) and Visual Studio Build Tools with **Desktop development with C++**. Download the source ZIP, extract it, and double-click **START-WINDOWS.bat**, or run:

```bat
cargo run --release --locked
```

</details>

## What it does

- **Built-in ad and tracker blocking**, always on in workspace tabs and app windows.
- Opens websites in workspace tabs and separate windows.
- Adds platforms with a name, a website link or QR image, and an optional picture.
- Changes or removes any platform, including built-in ones.
- Saves platforms, pictures, settings, and browser data locally.
- Customizes workspace colors and text size.

## Ad blocking — no extra installation

Ad blocking is **always on** in workspace tabs and app windows. No Brave, extension, subscription, or separate ad-blocker installation is needed.

The app uses **adblock-rust** with bundled **EasyList and EasyPrivacy** network rules. Filtering happens locally. The bundled rules are from **22 November 2026** and are refreshed with app updates.

It blocks many ads and trackers, but some sponsored posts and video ads may remain. This version blocks matching network requests; it does not remove every empty ad space or filter every type of browser traffic. Actual blocking still needs Windows runtime verification.

## Preview status

This is the approved Rust design preview, replacing the Python version. The picture shows the interface with example open tabs, not signed-in accounts.

The Windows x64 executable builds successfully and all 5 core tests pass on Windows. The interface preview passed navigation, search, tabs, text-size, and removal checks. **The Windows desktop app has not been run here.** Website-focus shortcuts, popup sign-in, downloads, notifications, and installer packaging still need verification or completion. Some sites may restrict embedded browsers.

The previous Python version remains in the repository history. See **LICENSES.txt** for notices.
