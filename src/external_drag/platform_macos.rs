//! macOS outgoing file-drag platform implementation.

use super::normalize_path;
use std::{
    ffi::{CStr, c_char, c_void},
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tracing::{info, warn};

type Id = *mut c_void;
type Sel = *mut c_void;
type ObjcBool = i8;

const YES: ObjcBool = 1;
const NO: ObjcBool = 0;
const NS_DRAG_OPERATION_COPY: usize = 1;
const NS_LEFT_MOUSE_DOWN: usize = 1;
const NS_LEFT_MOUSE_DRAGGED: usize = 6;
const NS_UTF8_STRING_ENCODING: usize = 4;

#[repr(C)]
#[derive(Clone, Copy)]
struct NSPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct NSSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct NSRect {
    origin: NSPoint,
    size: NSSize,
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {}

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_allocateClassPair(superclass: Id, name: *const c_char, extra_bytes: usize) -> Id;
    fn objc_getClass(name: *const c_char) -> Id;
    fn objc_msgSend();
    fn objc_registerClassPair(class: Id);
    fn class_addMethod(class: Id, name: Sel, imp: *const c_void, types: *const c_char) -> ObjcBool;
    fn sel_registerName(name: *const c_char) -> Sel;
}

pub(super) fn start_file_drag(paths: &[PathBuf]) -> Result<(), String> {
    info!(
        path_count = paths.len(),
        first_path = %paths
            .first()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        "external drag: starting macOS file drag"
    );
    let _pool = AutoreleasePool::new()?;
    let absolute = paths
        .iter()
        .map(|path| normalize_path(path.as_path()))
        .collect::<Vec<_>>();
    let app = unsafe { shared_application()? };
    let (window, view) = unsafe { key_window_and_content_view(app)? };
    let event = unsafe { external_drag_event(app, window)? };
    let items = unsafe { dragging_items(&absolute)? };
    let source = unsafe { dragging_source()? };
    let session = unsafe {
        msg_id_id_id_id(
            view,
            selector(c"beginDraggingSessionWithItems:event:source:"),
            items,
            event,
            source,
        )
    };
    if session.is_null() {
        warn!("external drag: NSView beginDraggingSessionWithItems returned nil");
        return Err(String::from(
            "Could not start external drag: native macOS view anchor is unavailable",
        ));
    }
    info!("external drag: macOS file drag session started");
    Ok(())
}

struct AutoreleasePool {
    pool: Id,
}

impl AutoreleasePool {
    fn new() -> Result<Self, String> {
        let pool = unsafe {
            let class = class(c"NSAutoreleasePool")?;
            msg_id(class, selector(c"new"))
        };
        if pool.is_null() {
            Err(String::from("Failed to create NSAutoreleasePool"))
        } else {
            Ok(Self { pool })
        }
    }
}

impl Drop for AutoreleasePool {
    fn drop(&mut self) {
        unsafe {
            msg_void(self.pool, selector(c"drain"));
        }
    }
}

unsafe fn shared_application() -> Result<Id, String> {
    let app = unsafe {
        let class = class(c"NSApplication")?;
        msg_id(class, selector(c"sharedApplication"))
    };
    if app.is_null() {
        Err(String::from("NSApplication sharedApplication returned nil"))
    } else {
        Ok(app)
    }
}

unsafe fn external_drag_event(app: Id, window: Id) -> Result<Id, String> {
    let event = unsafe { current_event(app) };
    if !event.is_null() && unsafe { is_drag_start_event(event) } {
        Ok(event)
    } else {
        unsafe { synthetic_left_drag_event(window) }
    }
}

unsafe fn current_event(app: Id) -> Id {
    unsafe { msg_id(app, selector(c"currentEvent")) }
}

unsafe fn is_drag_start_event(event: Id) -> bool {
    matches!(
        unsafe { msg_usize(event, selector(c"type")) },
        NS_LEFT_MOUSE_DOWN | NS_LEFT_MOUSE_DRAGGED
    )
}

unsafe fn synthetic_left_drag_event(window: Id) -> Result<Id, String> {
    let point = unsafe { msg_point(window, selector(c"mouseLocationOutsideOfEventStream")) };
    let window_number = unsafe { msg_isize(window, selector(c"windowNumber")) };
    let event_class = unsafe { class(c"NSEvent")? };
    let event = unsafe {
        msg_id_usize_point_usize_f64_isize_id_isize_isize_f64(
            event_class,
            selector(
                c"mouseEventWithType:location:modifierFlags:timestamp:windowNumber:context:eventNumber:clickCount:pressure:",
            ),
            NS_LEFT_MOUSE_DRAGGED,
            point,
            0,
            0.0,
            window_number,
            std::ptr::null_mut(),
            0,
            1,
            1.0,
        )
    };
    if event.is_null() {
        Err(String::from(
            "Failed to synthesize NSEvent for external drag",
        ))
    } else {
        Ok(event)
    }
}

unsafe fn key_window_and_content_view(app: Id) -> Result<(Id, Id), String> {
    let mut window = unsafe { msg_id(app, selector(c"keyWindow")) };
    if window.is_null() {
        window = unsafe { msg_id(app, selector(c"mainWindow")) };
    }
    if window.is_null() {
        return Err(String::from(
            "Could not start external drag: NSApplication has no key or main window",
        ));
    }
    let view = unsafe { msg_id(window, selector(c"contentView")) };
    if view.is_null() {
        Err(String::from(
            "Could not start external drag: NSWindow contentView returned nil",
        ))
    } else {
        Ok((window, view))
    }
}

unsafe fn dragging_items(paths: &[PathBuf]) -> Result<Id, String> {
    let items = unsafe { ns_mutable_array(paths.len())? };
    for path in paths {
        let url = unsafe { file_url_for_path(path)? };
        let item = unsafe { ns_dragging_item(url)? };
        let contents = unsafe { file_icon_for_path(path)? };
        unsafe {
            msg_void_rect_id(
                item,
                selector(c"setDraggingFrame:contents:"),
                dragging_frame(),
                contents,
            );
            msg_void_id(items, selector(c"addObject:"), item);
        }
    }
    Ok(items)
}

unsafe fn ns_mutable_array(capacity: usize) -> Result<Id, String> {
    let array = unsafe {
        let class = class(c"NSMutableArray")?;
        msg_id_usize(class, selector(c"arrayWithCapacity:"), capacity)
    };
    if array.is_null() {
        Err(String::from("Failed to create NSMutableArray"))
    } else {
        Ok(array)
    }
}

unsafe fn file_url_for_path(path: &Path) -> Result<Id, String> {
    let path = path_to_string(path)?;
    let ns_path = unsafe { ns_string(&path)? };
    let url = unsafe {
        let class = class(c"NSURL")?;
        msg_id_id(class, selector(c"fileURLWithPath:"), ns_path)
    };
    if url.is_null() {
        Err(format!("Failed to create file URL for {path}"))
    } else {
        Ok(url)
    }
}

unsafe fn ns_dragging_item(url: Id) -> Result<Id, String> {
    let allocated = unsafe {
        let class = class(c"NSDraggingItem")?;
        msg_id(class, selector(c"alloc"))
    };
    if allocated.is_null() {
        return Err(String::from("Failed to allocate NSDraggingItem"));
    }
    let item = unsafe { msg_id_id(allocated, selector(c"initWithPasteboardWriter:"), url) };
    if item.is_null() {
        Err(String::from("Failed to create NSDraggingItem"))
    } else {
        Ok(unsafe { msg_id(item, selector(c"autorelease")) })
    }
}

unsafe fn file_icon_for_path(path: &Path) -> Result<Id, String> {
    let path = path_to_string(path)?;
    let ns_path = unsafe { ns_string(&path)? };
    let workspace = unsafe {
        let class = class(c"NSWorkspace")?;
        msg_id(class, selector(c"sharedWorkspace"))
    };
    if workspace.is_null() {
        return Err(String::from("NSWorkspace sharedWorkspace returned nil"));
    }
    let icon = unsafe { msg_id_id(workspace, selector(c"iconForFile:"), ns_path) };
    if icon.is_null() {
        Err(format!("NSWorkspace iconForFile returned nil for {path}"))
    } else {
        Ok(icon)
    }
}

fn dragging_frame() -> NSRect {
    NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize {
            width: 48.0,
            height: 48.0,
        },
    }
}

unsafe fn dragging_source() -> Result<Id, String> {
    static SOURCE: OnceLock<usize> = OnceLock::new();
    let source = *SOURCE.get_or_init(|| unsafe {
        match create_dragging_source() {
            Ok(source) => source as usize,
            Err(_) => 0,
        }
    }) as Id;
    if source.is_null() {
        Err(String::from("Failed to create NSDraggingSource"))
    } else {
        Ok(source)
    }
}

unsafe fn create_dragging_source() -> Result<Id, String> {
    let superclass = unsafe { class(c"NSObject")? };
    let class_name = c"WavecrateExternalFileDraggingSource";
    let mut source_class = unsafe { objc_getClass(class_name.as_ptr()) };
    if source_class.is_null() {
        source_class = unsafe { objc_allocateClassPair(superclass, class_name.as_ptr(), 0) };
        if source_class.is_null() {
            return Err(String::from("objc_allocateClassPair failed"));
        }
        unsafe {
            add_method(
                source_class,
                c"draggingSession:sourceOperationMaskForDraggingContext:",
                dragging_source_operation_mask as *const c_void,
                c"Q@:@@q",
            )?;
            add_method(
                source_class,
                c"ignoreModifierKeysForDraggingSession:",
                dragging_source_ignores_modifier_keys as *const c_void,
                c"c@:@",
            )?;
            objc_registerClassPair(source_class);
        }
    }
    let source = unsafe { msg_id(source_class, selector(c"new")) };
    if source.is_null() {
        Err(String::from("Failed to instantiate NSDraggingSource"))
    } else {
        Ok(source)
    }
}

unsafe fn add_method(
    class: Id,
    name: &'static CStr,
    imp: *const c_void,
    types: &'static CStr,
) -> Result<(), String> {
    let added = unsafe { class_addMethod(class, selector(name), imp, types.as_ptr()) };
    if added == NO {
        Err(format!(
            "class_addMethod failed for {}",
            name.to_string_lossy()
        ))
    } else {
        Ok(())
    }
}

extern "C" fn dragging_source_operation_mask(_: Id, _: Sel, _: Id, _: isize) -> usize {
    NS_DRAG_OPERATION_COPY
}

extern "C" fn dragging_source_ignores_modifier_keys(_: Id, _: Sel, _: Id) -> ObjcBool {
    YES
}

unsafe fn ns_string(value: &str) -> Result<Id, String> {
    let allocated = unsafe {
        let class = class(c"NSString")?;
        msg_id(class, selector(c"alloc"))
    };
    if allocated.is_null() {
        return Err(String::from("Failed to allocate NSString"));
    }
    let string = unsafe {
        msg_id_ptr_usize_usize(
            allocated,
            selector(c"initWithBytes:length:encoding:"),
            value.as_ptr().cast(),
            value.len(),
            NS_UTF8_STRING_ENCODING,
        )
    };
    if string.is_null() {
        Err(String::from("Failed to create NSString"))
    } else {
        Ok(unsafe { msg_id(string, selector(c"autorelease")) })
    }
}

unsafe fn class(name: &'static CStr) -> Result<Id, String> {
    let class = unsafe { objc_getClass(name.as_ptr()) };
    if class.is_null() {
        Err(format!(
            "Objective-C class {} not found",
            name.to_string_lossy()
        ))
    } else {
        Ok(class)
    }
}

unsafe fn selector(name: &'static CStr) -> Sel {
    unsafe { sel_registerName(name.as_ptr()) }
}

unsafe fn msg_id(receiver: Id, selector: Sel) -> Id {
    let msg: unsafe extern "C" fn(Id, Sel) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}

unsafe fn msg_id_id(receiver: Id, selector: Sel, arg: Id) -> Id {
    let msg: unsafe extern "C" fn(Id, Sel, Id) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, arg) }
}

unsafe fn msg_id_id_id_id(receiver: Id, selector: Sel, first: Id, second: Id, third: Id) -> Id {
    let msg: unsafe extern "C" fn(Id, Sel, Id, Id, Id) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, first, second, third) }
}

unsafe fn msg_id_usize(receiver: Id, selector: Sel, arg: usize) -> Id {
    let msg: unsafe extern "C" fn(Id, Sel, usize) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, arg) }
}

unsafe fn msg_id_usize_point_usize_f64_isize_id_isize_isize_f64(
    receiver: Id,
    selector: Sel,
    event_type: usize,
    location: NSPoint,
    modifier_flags: usize,
    timestamp: f64,
    window_number: isize,
    context: Id,
    event_number: isize,
    click_count: isize,
    pressure: f64,
) -> Id {
    let msg: unsafe extern "C" fn(
        Id,
        Sel,
        usize,
        NSPoint,
        usize,
        f64,
        isize,
        Id,
        isize,
        isize,
        f64,
    ) -> Id = unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe {
        msg(
            receiver,
            selector,
            event_type,
            location,
            modifier_flags,
            timestamp,
            window_number,
            context,
            event_number,
            click_count,
            pressure,
        )
    }
}

unsafe fn msg_id_ptr_usize_usize(
    receiver: Id,
    selector: Sel,
    bytes: *const c_void,
    length: usize,
    encoding: usize,
) -> Id {
    let msg: unsafe extern "C" fn(Id, Sel, *const c_void, usize, usize) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, bytes, length, encoding) }
}

unsafe fn msg_void(receiver: Id, selector: Sel) {
    let msg: unsafe extern "C" fn(Id, Sel) =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}

unsafe fn msg_void_id(receiver: Id, selector: Sel, arg: Id) {
    let msg: unsafe extern "C" fn(Id, Sel, Id) =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, arg) }
}

unsafe fn msg_void_rect_id(receiver: Id, selector: Sel, rect: NSRect, arg: Id) {
    let msg: unsafe extern "C" fn(Id, Sel, NSRect, Id) =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, rect, arg) }
}

unsafe fn msg_isize(receiver: Id, selector: Sel) -> isize {
    let msg: unsafe extern "C" fn(Id, Sel) -> isize =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}

unsafe fn msg_point(receiver: Id, selector: Sel) -> NSPoint {
    let msg: unsafe extern "C" fn(Id, Sel) -> NSPoint =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}

unsafe fn msg_usize(receiver: Id, selector: Sel) -> usize {
    let msg: unsafe extern "C" fn(Id, Sel) -> usize =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        format!(
            "Cannot drag non-UTF-8 path to external application: {}",
            path.display()
        )
    })
}
