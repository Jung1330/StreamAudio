use tauri::{PhysicalPosition, WebviewWindow};
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

fn get_primary_work_area() -> RECT {
    let mut rect = RECT::default();
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut RECT as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    rect
}

/// Positions the main mixer window right above the taskbar at the bottom-right corner.
pub fn position_above_taskbar(window: &WebviewWindow) {
    let work = get_primary_work_area();
    if work.right <= work.left || work.bottom <= work.top {
        return;
    }
    let size = match window.outer_size() {
        Ok(s) => s,
        Err(_) => return,
    };
    let margin = 12i32;
    let x = (work.right - size.width as i32 - margin).max(work.left + margin);
    let y = (work.bottom - size.height as i32 - margin).max(work.top + margin);
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// Positions the OSD toast near the bottom-right corner above the taskbar (or top-right if preferred).
pub fn position_toast(window: &WebviewWindow) {
    let work = get_primary_work_area();
    if work.right <= work.left || work.bottom <= work.top {
        return;
    }
    let size = match window.outer_size() {
        Ok(s) => s,
        Err(_) => return,
    };
    let margin = 18i32;
    let x = (work.right - size.width as i32 - margin).max(work.left + margin);
    let y = (work.bottom - size.height as i32 - margin).max(work.top + margin);
    let _ = window.set_position(PhysicalPosition::new(x, y));
}
