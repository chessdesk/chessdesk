//! Saves what the window shows as a PNG, for scripts. GPUI can't read back what it drew, so
//! this asks the system: Windows paints the window into a bitmap, however it's covered, while
//! macOS and Linux take the window's part of the screen.

use std::path::Path;
use std::process::Command;

use gpui::Window;

/// Windows PowerShell, which every Windows has, given the window and the file in the
/// environment, with .NET's `System.Drawing` to write the PNG.
/// `PrintWindow` with `PW_RENDERFULLCONTENT` (2) also gets what DirectX drew. The process
/// declares itself DPI-aware first, or a scaled display would report a shrunken window.
#[cfg(windows)]
const PRINT_WINDOW: &str = r#"
Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
public static class Shot {
    [StructLayout(LayoutKind.Sequential)] struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] static extern bool SetProcessDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);
    public static void Save(long handle, string path) {
        SetProcessDpiAwarenessContext(new IntPtr(-4));
        IntPtr window = new IntPtr(handle);
        Rect rect;
        if (!GetWindowRect(window, out rect)) throw new Exception("no such window");
        using (Bitmap bitmap = new Bitmap(rect.Right - rect.Left, rect.Bottom - rect.Top)) {
            using (Graphics graphics = Graphics.FromImage(bitmap)) {
                IntPtr dc = graphics.GetHdc();
                bool painted = PrintWindow(window, dc, 2);
                graphics.ReleaseHdc(dc);
                if (!painted) throw new Exception("the window would not paint");
            }
            bitmap.Save(path, ImageFormat.Png);
        }
    }
}
'@
[Shot]::Save([long]$env:CHESSDESK_SHOT_WINDOW, $env:CHESSDESK_SHOT_PATH)
"#;

/// What takes the screenshot, found on the window's thread. Run it with [`take`] on another:
/// on Windows, the window paints itself into the picture, which its thread must be free for.
pub fn capture(window: &Window, path: &Path) -> Result<Command, String> {
    if let Some(folder) = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
    {
        std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    }
    command(window, path)
}

/// Takes the screenshot [`capture`] prepared.
pub fn take(mut command: Command) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("could not take the screenshot: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "could not take the screenshot: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(windows)]
fn command(window: &Window, path: &Path) -> Result<Command, String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window).map_err(|error| error.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("not a Win32 window".to_owned());
    };
    let mut command = Command::new("powershell");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", PRINT_WINDOW])
        // Arguments after `-Command` would become part of the script.
        .env("CHESSDESK_SHOT_WINDOW", handle.hwnd.get().to_string())
        .env("CHESSDESK_SHOT_PATH", path);
    Ok(command)
}

/// `screencapture` takes the rectangle in points, as GPUI gives it.
#[cfg(target_os = "macos")]
fn command(window: &Window, path: &Path) -> Result<Command, String> {
    let bounds = window.bounds();
    let mut command = Command::new("screencapture");
    command
        .arg("-x")
        .arg(format!(
            "-R{},{},{},{}",
            f32::from(bounds.origin.x),
            f32::from(bounds.origin.y),
            f32::from(bounds.size.width),
            f32::from(bounds.size.height)
        ))
        .arg(path);
    Ok(command)
}

/// ImageMagick's `import`, on X11 only: Wayland lets no program read the screen this way.
#[cfg(all(unix, not(target_os = "macos")))]
fn command(window: &Window, path: &Path) -> Result<Command, String> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return Err("screenshots need X11; Wayland doesn't allow them".to_owned());
    }
    let bounds = window.bounds();
    let scale = window.scale_factor();
    let pixels = |value: gpui::Pixels| (f32::from(value) * scale).round() as i32;
    let mut command = Command::new("import");
    command
        .args(["-window", "root", "-crop"])
        .arg(format!(
            "{}x{}+{}+{}",
            pixels(bounds.size.width),
            pixels(bounds.size.height),
            pixels(bounds.origin.x),
            pixels(bounds.origin.y)
        ))
        .arg(path);
    Ok(command)
}
