# Linux

Planned for v0.3 (X11 plus the parts that work on any session) and v0.5 (Wayland window control).
Nothing here is implemented yet. Evidence and sources: [research/platform-windows-linux.md](../research/platform-windows-linux.md).

GNOME 50 and Plasma 6.8 are Wayland-only, so Wayland is the main target long term. On X11 window
control is easy and capture detection impossible; on Wayland it is the other way round.

## Permissions and setup

- **Tabs** need the accessibility bus: `gsettings set org.gnome.desktop.interface toolkit-accessibility true`,
  then restart the browser. The app shows the command and runs it only after you agree.
- **Hotkey on Wayland** goes through the GlobalShortcuts portal; you approve it in a system dialog.
  Where there is no portal (Sway), bind `closecall panic` in your compositor.
- **v0.5**: the KWin script (loaded over D-Bus, no root) or our GNOME Shell extension from extensions.gnome.org.

## Capabilities

| Capability | X11 | KDE Plasma 6 Wayland | GNOME Wayland | wlroots, Hyprland, niri |
|---|---|---|---|---|
| Windows and titles | Full (EWMH) | Full (KWin script, v0.5) | Partial (our extension, v0.5; AT-SPI frame names before that) | Full (wlr-foreign-toplevel, v0.5) |
| Close, minimize | Full | Full (v0.5) | Partial (extension, v0.5) | Full (v0.5) |
| Tab titles | Partial (AT-SPI after enabling accessibility) | Partial | Partial | Partial |
| Tab URLs | Active tab only; all tabs with the v0.4 extension | same | same | same |
| File behind a document | Partial (`/proc` cmdline and fds, LibreOffice lock files) | same | same | same |
| Unsaved state | Title markers only, otherwise Unknown (treated as unsaved) | same | same | same |

Under a Wayland session v0.3 sees XWayland windows only. The capability report says exactly that.

## Triggers

| Trigger | Mechanism | Confidence |
|---|---|---|
| Screen share | PipeWire registry: a compositor or portal `Video/Source` node linked to a consumer stream. The consumer app is protected. Wayland only. | Medium-High; node names per desktop version unverified |
| Screen share on X11 | **Not detectable.** Any X client can read the screen silently. Use the hotkey. | – |
| Projector | RandR, `wl_output`, Mutter `DisplayConfig` | High |
| Call likely | PipeWire audio input stream from a meeting app | Medium |
| Sleep | logind `PrepareForSleep` with a delay inhibitor | Full |
| Lid | UPower `LidIsClosed` | Full |
| Lock | logind `LockedHint`, `org.gnome.ScreenSaver` / `org.freedesktop.ScreenSaver` `ActiveChanged` | Full on GNOME and KDE; depends on the locker elsewhere |
| Idle | Mutter IdleMonitor, ext-idle-notify v2, XScreenSaver | Full |
| Panic | X11 `XGrabKey`; Wayland portal; `closecall panic` everywhere | Full / Partial |

## Limits

- No input injection on Wayland, so no keyboard shortcuts as a fallback.
- No Flatpak in v0.x: the sandbox filters D-Bus, hides host processes and blocks Wayland window control.
- Packages: `.deb`, `.rpm`, AppImage, AUR.
