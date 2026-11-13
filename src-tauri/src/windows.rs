//! The output windows (sanctuary and/or stream).
//!
//! They no longer open on their own: the operator opens each one from the
//! control window (top bar → Outputs), either as an ordinary window or full
//! screen on a monitor, by default the "second monitor" (the first screen that
//! is not showing the control window). `displays.open_at_start` in node.json
//! brings back the old behaviour of opening them at start-up.

use episco_sync::{NodeConfig, OutputKind};
use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};

type CmdResult<T> = Result<T, String>;

/// Emitted to every window when an output opens, closes or changes mode.
pub const EVENT: &str = "episco://outputs";

fn title(output: OutputKind) -> &'static str {
    match output {
        OutputKind::Sanctuary => "Sanctuary",
        OutputKind::Stream => "Live Stream",
    }
}

fn same_monitor(a: &Monitor, b: &Monitor) -> bool {
    a.position() == b.position() && a.size() == b.size()
}

/// Index of the monitor the control window is on (0 if unknown).
fn control_monitor(app: &AppHandle, monitors: &[Monitor]) -> usize {
    app.get_webview_window("control")
        .and_then(|w| w.current_monitor().ok().flatten())
        .and_then(|m| monitors.iter().position(|x| same_monitor(x, &m)))
        .unwrap_or(0)
}

/// The monitor an output goes on by default: the one set in node.json if it
/// exists; otherwise the first screen that is not the control screen (the
/// "second monitor"). On a standalone machine the stream takes the next one
/// after the sanctuary's, when there is one.
fn default_monitor(app: &AppHandle, config: &NodeConfig, output: OutputKind, monitors: &[Monitor]) -> usize {
    let explicit = match output {
        OutputKind::Sanctuary => config.displays.sanctuary_monitor,
        OutputKind::Stream => config.displays.stream_monitor,
    };
    if let Some(i) = explicit.filter(|i| *i < monitors.len()) {
        return i;
    }
    let control = control_monitor(app, monitors);
    let others: Vec<usize> = (0..monitors.len()).filter(|i| *i != control).collect();
    let both = config.role.outputs().len() > 1;
    let pick = if both && output == OutputKind::Stream { others.get(1).or(others.first()) } else { others.first() };
    pick.copied().unwrap_or(control)
}

fn emit_changed(app: &AppHandle) {
    let _ = app.emit(EVENT, ());
}

/// The output's window, created (hidden) if needed.
fn window_for(app: &AppHandle, output: OutputKind) -> tauri::Result<WebviewWindow> {
    let label = output.window_label();
    if let Some(w) = app.get_webview_window(label) {
        return Ok(w);
    }
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(format!("output/{label}.html").into()))
        .title(title(output))
        .visible(false)
        .inner_size(960.0, 540.0);
    if output == OutputKind::Stream {
        // Keyed overlay: the page paints only the text zones.
        builder = builder.transparent(true);
    }
    let window = builder.build()?;
    let handle = app.clone();
    window.on_window_event(move |e| {
        if matches!(e, WindowEvent::Destroyed | WindowEvent::Resized(_)) {
            emit_changed(&handle);
        }
    });
    Ok(window)
}

/// A monitor's rectangle in the coordinates windows are placed in: macOS
/// places windows in points (logical), Windows and Linux in pixels.
///
/// Tauri reports a monitor's position as its point position times *that
/// monitor's* scale, but on macOS turns a window position back into points
/// with *the window's* scale. With a Retina built-in display (2×) and an
/// external monitor (1×), "the external monitor's left edge" in pixels thus
/// lands half-way across the built-in display, and full screen happens
/// there. Placing windows in points avoids the mix-up.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Area {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    /// `x`… are points (macOS) rather than pixels.
    logical: bool,
}

impl Area {
    fn of(m: &Monitor) -> Area {
        Area::from_pixels(
            m.position().x,
            m.position().y,
            m.size().width,
            m.size().height,
            m.scale_factor(),
            cfg!(target_os = "macos"),
        )
    }

    fn from_pixels(x: i32, y: i32, w: u32, h: u32, scale: f64, logical: bool) -> Area {
        let k = if logical && scale > 0.0 { scale } else { 1.0 };
        Area { x: f64::from(x) / k, y: f64::from(y) / k, w: f64::from(w) / k, h: f64::from(h) / k, logical }
    }

    /// A window `frac` of the area's size, centred in it.
    fn centred(self, frac: f64) -> Area {
        let (w, h) = (self.w * frac, self.h * frac);
        Area { x: self.x + (self.w - w) / 2.0, y: self.y + (self.h - h) / 2.0, w, h, logical: self.logical }
    }

    fn apply(self, window: &WebviewWindow) {
        if self.logical {
            let _ = window.set_size(tauri::LogicalSize::new(self.w, self.h));
            let _ = window.set_position(tauri::LogicalPosition::new(self.x, self.y));
        } else {
            let _ = window.set_size(PhysicalSize::new(self.w.round() as u32, self.h.round() as u32));
            let _ = window.set_position(PhysicalPosition::new(self.x.round() as i32, self.y.round() as i32));
        }
    }
}

/// Whether `window` is on the monitor `target` (by the system's own account).
fn on_monitor(window: &WebviewWindow, target: &Monitor) -> bool {
    window.current_monitor().ok().flatten().is_some_and(|m| same_monitor(&m, target))
}

/// Shows an output: full screen on `monitor` (default: its default monitor),
/// or as an ordinary window on that monitor.
///
/// Full screen happens on whichever screen the window is on, and a hidden
/// window cannot be moved reliably (macOS keeps it on the main screen). So
/// the window is shown, moved onto the chosen monitor (in points on macOS,
/// see [`Area`]), checked to be there, and only then made full screen. If
/// the system still will not move it, it is made to fill the chosen
/// monitor without a title bar instead, so it is never full screen on the
/// wrong one.
pub fn show_output(
    app: &AppHandle,
    config: &NodeConfig,
    output: OutputKind,
    fullscreen: bool,
    monitor: Option<usize>,
) -> tauri::Result<()> {
    let monitors = app.available_monitors().unwrap_or_default();
    let index =
        monitor.filter(|i| *i < monitors.len()).unwrap_or_else(|| default_monitor(app, config, output, &monitors));
    let window = window_for(app, output)?;
    let target = monitors.get(index).cloned();
    let was_fullscreen = window.is_fullscreen().unwrap_or(false);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let pause = |ms| tokio::time::sleep(std::time::Duration::from_millis(ms));
        if was_fullscreen {
            let _ = window.set_fullscreen(false);
            // macOS animates out of full screen; moving during it is ignored.
            pause(900).await;
        }
        // Back to an ordinary window (after a "fill the monitor" fallback).
        let _ = window.set_decorations(true);
        let _ = window.set_always_on_top(false);
        let _ = window.show();
        let mut placed = true;
        if let Some(m) = &target {
            let area = Area::of(m);
            area.centred(0.5).apply(&window);
            pause(250).await;
            // Not there yet (the system was slow): once more, near the corner.
            for attempt in 0..2 {
                if on_monitor(&window, m) {
                    break;
                }
                let near = Area { x: area.x + 60.0, y: area.y + 60.0, w: area.w / 2.0, h: area.h / 2.0, ..area };
                near.apply(&window);
                pause(300 + 200 * attempt).await;
            }
            placed = on_monitor(&window, m);
            if !placed {
                tracing::warn!("{} output: could not move it onto monitor {index}", output.window_label());
            }
        }
        if fullscreen {
            match (&target, placed) {
                (Some(m), false) => {
                    // Never full screen on the wrong monitor: fill the chosen one.
                    let _ = window.set_decorations(false);
                    Area::of(m).apply(&window);
                    let _ = window.set_always_on_top(true);
                }
                _ => {
                    let _ = window.set_fullscreen(true);
                }
            }
            let _ = window.set_focus();
        }
        tracing::info!(
            "{} output {} on monitor {index}",
            output.window_label(),
            if fullscreen { "full screen" } else { "in a window" }
        );
        pause(300).await;
        emit_changed(&handle);
    });
    Ok(())
}

/// Closes every output window (when the main window closes).
pub fn close_outputs(app: &AppHandle) {
    for output in [OutputKind::Sanctuary, OutputKind::Stream] {
        if let Some(w) = app.get_webview_window(output.window_label()) {
            let _ = w.destroy();
        }
    }
}

/// Opens the outputs at start-up if node.json asks for it (`open_at_start`).
pub fn open_at_start(app: &AppHandle, config: &NodeConfig) -> tauri::Result<()> {
    if !config.displays.open_at_start {
        return Ok(());
    }
    for &output in config.role.outputs() {
        show_output(app, config, output, !config.displays.windowed, None)?;
    }
    Ok(())
}

// ---- commands ----------------------------------------------------------------

#[derive(Serialize)]
pub struct MonitorInfo {
    index: usize,
    name: String,
    width: u32,
    height: u32,
    /// The control window is on this monitor.
    control: bool,
}

#[derive(Serialize)]
pub struct OutputInfo {
    output: OutputKind,
    title: &'static str,
    open: bool,
    fullscreen: bool,
    /// The monitor it is on, when open.
    #[serde(skip_serializing_if = "Option::is_none")]
    monitor: Option<usize>,
    /// Where "Full screen" puts it by default.
    default_monitor: usize,
}

#[derive(Serialize)]
pub struct OutputsStatus {
    monitors: Vec<MonitorInfo>,
    outputs: Vec<OutputInfo>,
}

/// This node's outputs, whether each is open and where, and the monitors.
#[tauri::command]
pub fn outputs_status(app: AppHandle, config: State<'_, NodeConfig>) -> OutputsStatus {
    let monitors = app.available_monitors().unwrap_or_default();
    let control = control_monitor(&app, &monitors);
    let outputs = config
        .role
        .outputs()
        .iter()
        .map(|&output| {
            let window = app.get_webview_window(output.window_label());
            let open = window.as_ref().is_some_and(|w| w.is_visible().unwrap_or(false));
            let monitor = window
                .as_ref()
                .filter(|_| open)
                .and_then(|w| w.current_monitor().ok().flatten())
                .and_then(|m| monitors.iter().position(|x| same_monitor(x, &m)));
            OutputInfo {
                output,
                title: title(output),
                open,
                fullscreen: open && window.as_ref().is_some_and(|w| w.is_fullscreen().unwrap_or(false)),
                monitor,
                default_monitor: default_monitor(&app, &config, output, &monitors),
            }
        })
        .collect();
    let monitors = monitors
        .iter()
        .enumerate()
        .map(|(index, m)| MonitorInfo {
            index,
            name: m.name().cloned().unwrap_or_else(|| format!("Monitor {}", index + 1)),
            width: m.size().width,
            height: m.size().height,
            control: index == control,
        })
        .collect();
    OutputsStatus { monitors, outputs }
}

/// Opens (or moves) an output: full screen or in a window, on `monitor`.
#[tauri::command]
pub fn output_show(
    app: AppHandle,
    config: State<'_, NodeConfig>,
    output: OutputKind,
    fullscreen: bool,
    monitor: Option<usize>,
) -> CmdResult<()> {
    if !config.role.outputs().contains(&output) {
        return Err(format!("this node has no {} output", title(output)));
    }
    show_output(&app, &config, output, fullscreen, monitor).map_err(|e| e.to_string())
}

/// Closes an output window.
#[tauri::command]
pub fn output_close(app: AppHandle, output: OutputKind) -> CmdResult<()> {
    if let Some(w) = app.get_webview_window(output.window_label()) {
        w.destroy().map_err(|e| e.to_string())?;
    }
    emit_changed(&app);
    Ok(())
}

/// From an output window itself: leave full screen (Esc) or toggle it
/// (double-click), staying on the same monitor.
#[tauri::command]
pub fn output_set_fullscreen(
    app: AppHandle,
    config: State<'_, NodeConfig>,
    output: OutputKind,
    fullscreen: Option<bool>,
) -> CmdResult<()> {
    let Some(window) = app.get_webview_window(output.window_label()) else { return Ok(()) };
    let now = window.is_fullscreen().unwrap_or(false);
    let want = fullscreen.unwrap_or(!now);
    if want == now {
        return Ok(());
    }
    let monitors = app.available_monitors().unwrap_or_default();
    let current =
        window.current_monitor().ok().flatten().and_then(|m| monitors.iter().position(|x| same_monitor(x, &m)));
    show_output(&app, &config, output, want, current).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_retina_laptop_with_an_external_monitor() {
        // macOS: built-in 1512×982 points at 2× (3024×1964 pixels) at the
        // origin; external 1920×1080 points at 1× to its right, at x = 1512
        // points. Tauri reports the external monitor at x = 1512 "pixels".
        let external = Area::from_pixels(1512, 0, 1920, 1080, 1.0, true);
        let window = external.centred(0.5);
        assert_eq!((window.x, window.y, window.w, window.h), (1992.0, 270.0, 960.0, 540.0));
        assert!(window.logical, "placed in points, which macOS uses as they are");
        // A 2× external monitor: pixels are halved back to points.
        let retina_external = Area::from_pixels(3024, 0, 5120, 2880, 2.0, true);
        assert_eq!((retina_external.x, retina_external.w), (1512.0, 2560.0));
        // Windows and Linux place windows in pixels, as reported.
        let pixels = Area::from_pixels(1920, 0, 1920, 1080, 1.5, false);
        assert_eq!((pixels.x, pixels.w, pixels.logical), (1920.0, 1920.0, false));
    }
}
