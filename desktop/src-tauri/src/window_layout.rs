#[cfg(windows)]
pub fn fit_work_area(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()? else {
        return Ok(());
    };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let inner = window.inner_size()?.to_logical::<f64>(scale);
    let outer = window.outer_size()?.to_logical::<f64>(scale);
    let work = area.size.to_logical::<f64>(scale);
    let frame = (outer.width - inner.width, outer.height - inner.height);
    let (width, height) = fitted_size(
        (inner.width, inner.height),
        (work.width, work.height),
        frame,
    );
    window.set_min_size(Some(tauri::LogicalSize::new(
        width.min(880.0),
        height.min(580.0),
    )))?;
    window.set_size(tauri::LogicalSize::new(width, height))?;
    window.set_position(tauri::PhysicalPosition::new(
        area.position.x + ((work.width - width - frame.0) * scale / 2.0).round() as i32,
        area.position.y + ((work.height - height - frame.1) * scale / 2.0).round() as i32,
    ))
}

fn fitted_size(desired: (f64, f64), work: (f64, f64), frame: (f64, f64)) -> (f64, f64) {
    // Keep the whole window inside the usable desktop, including its title bar.
    (
        desired.0.min((work.0 - frame.0 - 32.0).max(1.0)),
        desired.1.min((work.1 - frame.1 - 32.0).max(1.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::fitted_size;

    #[test]
    fn fits_1080p_work_area_at_common_windows_scales() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let work = (1920.0 / scale, 1032.0 / scale);
            let (width, height) = fitted_size((1160.0, 720.0), work, (16.0, 39.0));
            assert!(width + 16.0 + 32.0 <= work.0);
            assert!(height + 39.0 + 32.0 <= work.1);
            assert!(width <= 1160.0 && height <= 720.0);
        }
        assert_eq!(
            fitted_size((1160.0, 720.0), (1920.0, 1032.0), (16.0, 39.0)),
            (1160.0, 720.0)
        );
    }
}
