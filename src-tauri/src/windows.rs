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

/// Shows an output: full screen on `monitor` (default: its default monitor),
/// or as an ordinary window on that monitor.
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
    // Leave full screen before moving between monitors.
    if window.is_fullscreen().unwrap_or(false) {
        window.set_fullscreen(false)?;
    }
    if let Some(m) = monitors.get(index) {
        let pos = m.position();
        let size = m.size();
        if fullscreen {
            window.set_position(PhysicalPosition::new(pos.x, pos.y))?;
        } else {
            // A half-size window in the middle of the chosen screen.
            let (w, h) = (size.width / 2, size.height / 2);
            window.set_size(PhysicalSize::new(w, h))?;
            window.set_position(PhysicalPosition::new(
                pos.x + (size.width - w) as i32 / 2,
                pos.y + (size.height - h) as i32 / 2,
            ))?;
        }
    }
    window.show()?;
    if fullscreen {
        window.set_fullscreen(true)?;
    }
    tracing::info!(
        "{} output {} on monitor {index}",
        output.window_label(),
        if fullscreen { "full screen" } else { "in a window" }
    );
    emit_changed(app);
    Ok(())
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
