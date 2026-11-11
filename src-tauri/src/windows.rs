//! Output window placement on physical monitors.

use episco_sync::{NodeConfig, OutputKind};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

/// Opens this node's output windows (sanctuary and/or stream) on their
/// configured monitors. A missing monitor falls back to a normal window on
/// the primary screen so the operator can still see the output.
pub fn open_outputs(app: &AppHandle, config: &NodeConfig) -> tauri::Result<()> {
    let monitors = app.available_monitors().unwrap_or_default();
    tracing::info!("{} monitor(s) detected", monitors.len());

    for &output in config.role.outputs() {
        let label = output.window_label();
        if app.get_webview_window(label).is_some() {
            continue;
        }
        let index = config.monitor_for(output);
        let monitor = monitors.get(index);
        let title = match output {
            OutputKind::Sanctuary => "Sanctuary Output",
            OutputKind::Stream => "Live Stream Overlay",
        };

        let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(format!("output/{label}.html").into()))
            .title(title)
            .decorations(config.displays.windowed || monitor.is_none())
            .visible(false)
            .inner_size(960.0, 540.0);
        if output == OutputKind::Stream {
            // Keyed overlay: the page paints only the text zones.
            builder = builder.transparent(true);
        }
        let window = builder.build()?;

        match monitor {
            Some(m) => {
                let pos = m.position();
                window.set_position(PhysicalPosition::new(pos.x, pos.y))?;
                if config.displays.windowed {
                    let size = m.size();
                    window.set_size(PhysicalSize::new(size.width / 2, size.height / 2))?;
                } else {
                    window.set_fullscreen(true)?;
                }
                tracing::info!("{label} output on monitor {index} ({:?})", m.name());
            }
            None => tracing::warn!(
                "{label} output: monitor {index} not found ({} available); opening as a window",
                monitors.len()
            ),
        }
        window.show()?;
    }
    Ok(())
}
