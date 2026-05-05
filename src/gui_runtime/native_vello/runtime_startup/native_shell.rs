use super::super::*;

#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Reveal a minimal native shell frame before Vello startup work completes.
    ///
    /// On Windows this paints a lightweight GDI placeholder into the newly
    /// created HWND so cold-start surface and renderer initialization no longer
    /// leave the app entirely absent from the desktop. Other platforms keep the
    /// existing first-present reveal behavior.
    pub(in crate::gui_runtime::native_vello) fn maybe_reveal_startup_window_before_renderer_ready(
        &mut self,
    ) {
        #[cfg(target_os = "windows")]
        {
            if self.startup_window_visible {
                return;
            }
            self.reveal_startup_window();
            self.paint_native_startup_shell_frame();
        }
    }

    #[cfg(target_os = "windows")]
    fn paint_native_startup_shell_frame(&self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Ok(handle) = window.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        let title = if self.options.title.trim().is_empty() {
            crate::gui_runtime::DEFAULT_NATIVE_WINDOW_TITLE
        } else {
            self.options.title.as_str()
        };
        paint_windows_startup_shell_frame(handle.hwnd.get(), title, self.clear_color);
    }
}

#[cfg(target_os = "windows")]
fn paint_windows_startup_shell_frame(hwnd: isize, title: &str, clear_color: Rgba8) {
    use windows_sys::Win32::{
        Foundation::RECT,
        Graphics::Gdi::{
            CreateSolidBrush, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW,
            FillRect, GetDC, HGDIOBJ, ReleaseDC, SetBkMode, SetTextColor, TRANSPARENT,
        },
        UI::WindowsAndMessaging::GetClientRect,
    };

    unsafe {
        let hdc = GetDC(hwnd as _);
        if hdc.is_null() {
            return;
        }
        let mut rect = RECT::default();
        if GetClientRect(hwnd as _, &mut rect) == 0 {
            let _ = ReleaseDC(hwnd as _, hdc);
            return;
        }

        let background = CreateSolidBrush(rgb_to_colorref(clear_color));
        if !background.is_null() {
            let _ = FillRect(hdc, &rect, background);
        }

        let mut title_rect = RECT {
            left: rect.left,
            top: rect.top + ((rect.bottom - rect.top) / 2) - 26,
            right: rect.right,
            bottom: rect.top + ((rect.bottom - rect.top) / 2) + 2,
        };
        let mut subtitle_rect = RECT {
            left: rect.left,
            top: title_rect.bottom,
            right: rect.right,
            bottom: title_rect.bottom + 34,
        };
        let title_wide: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        let subtitle_wide: Vec<u16> = "Starting interface..."
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let _ = SetBkMode(hdc, TRANSPARENT as i32);
        let _ = SetTextColor(
            hdc,
            rgb_to_colorref(Rgba8 {
                r: 238,
                g: 243,
                b: 240,
                a: 255,
            }),
        );
        let _ = DrawTextW(
            hdc,
            title_wide.as_ptr(),
            -1,
            &mut title_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        let _ = SetTextColor(
            hdc,
            rgb_to_colorref(Rgba8 {
                r: 160,
                g: 174,
                b: 166,
                a: 255,
            }),
        );
        let _ = DrawTextW(
            hdc,
            subtitle_wide.as_ptr(),
            -1,
            &mut subtitle_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );

        if !background.is_null() {
            let _ = DeleteObject(background as HGDIOBJ);
        }
        let _ = ReleaseDC(hwnd as _, hdc);
    }
}

#[cfg(target_os = "windows")]
fn rgb_to_colorref(color: Rgba8) -> u32 {
    u32::from(color.r) | (u32::from(color.g) << 8) | (u32::from(color.b) << 16)
}
