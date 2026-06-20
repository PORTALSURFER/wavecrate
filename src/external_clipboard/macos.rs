use std::{
    ffi::{CStr, c_char, c_void},
    path::{Path, PathBuf},
};

type Id = *mut c_void;
type Sel = *mut c_void;
type ObjcBool = i8;

const NO: ObjcBool = 0;
const NS_UTF8_STRING_ENCODING: usize = 4;
const NS_FILENAMES_PBOARD_TYPE: &str = "NSFilenamesPboardType";
const NS_URL_PBOARD_TYPE: &str = "NSURLPboardType";
const NSPASTEBOARD_TYPE_FILE_URL: &str = "public.file-url";
const NSPASTEBOARD_TYPE_STRING: &str = "public.utf8-plain-text";

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {}

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_msgSend();
}

pub(super) fn copy_file_paths(paths: &[PathBuf]) -> Result<(), String> {
    let _pool = AutoreleasePool::new()?;
    let urls = unsafe { ns_mutable_array(paths.len())? };
    let filenames = unsafe { ns_mutable_array(paths.len())? };
    let mut first_file_url_string = None;
    for path in paths {
        unsafe {
            let path_string = path_to_pasteboard_string(path)?;
            let url = file_url_for_path(path)?;
            msg_void_id(urls, selector(c"addObject:"), url);
            msg_void_id(filenames, selector(c"addObject:"), ns_string(&path_string)?);
            if first_file_url_string.is_none() {
                first_file_url_string = Some(msg_id(url, selector(c"absoluteString")));
            }
        }
    }
    let pasteboard = unsafe { general_pasteboard()? };
    unsafe {
        let _ = msg_isize(pasteboard, selector(c"clearContents"));
        let ok = msg_bool_id(pasteboard, selector(c"writeObjects:"), urls);
        if ok == NO {
            return Err(String::from("NSPasteboard writeObjects failed"));
        }
        set_pasteboard_property_list(pasteboard, filenames, NS_FILENAMES_PBOARD_TYPE)?;
        if let Some(file_url_string) = first_file_url_string {
            set_pasteboard_string(pasteboard, file_url_string, NSPASTEBOARD_TYPE_FILE_URL)?;
            set_pasteboard_string(pasteboard, file_url_string, NS_URL_PBOARD_TYPE)?;
        }
    }
    Ok(())
}

pub(super) fn copy_text(text: &str) -> Result<(), String> {
    let _pool = AutoreleasePool::new()?;
    let pasteboard = unsafe { general_pasteboard()? };
    let ns_text = unsafe { ns_string(text)? };
    let text_type = unsafe { ns_string(NSPASTEBOARD_TYPE_STRING)? };
    unsafe {
        let _ = msg_isize(pasteboard, selector(c"clearContents"));
        set_pasteboard_string_id(pasteboard, ns_text, text_type)?;
    }
    Ok(())
}

pub(super) fn read_file_paths() -> Result<Vec<PathBuf>, String> {
    let _pool = AutoreleasePool::new()?;
    let pasteboard = unsafe { general_pasteboard()? };
    unsafe { read_file_paths_from_pasteboard(pasteboard) }
}

pub(super) fn read_text() -> Result<String, String> {
    let _pool = AutoreleasePool::new()?;
    let pasteboard = unsafe { general_pasteboard()? };
    unsafe {
        let text_type = ns_string(NSPASTEBOARD_TYPE_STRING)?;
        let text = msg_id_id(pasteboard, selector(c"stringForType:"), text_type);
        if text.is_null() {
            return Ok(String::new());
        }
        ns_string_to_rust(text)
    }
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

unsafe fn general_pasteboard() -> Result<Id, String> {
    let pasteboard = unsafe {
        let class = class(c"NSPasteboard")?;
        msg_id(class, selector(c"generalPasteboard"))
    };
    if pasteboard.is_null() {
        Err(String::from("NSPasteboard generalPasteboard returned nil"))
    } else {
        Ok(pasteboard)
    }
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
    let path = path_to_pasteboard_string(path)?;
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

unsafe fn set_pasteboard_property_list(
    pasteboard: Id,
    property_list: Id,
    pasteboard_type: &str,
) -> Result<(), String> {
    let pasteboard_type = unsafe { ns_string(pasteboard_type)? };
    let ok = unsafe {
        msg_bool_id_id(
            pasteboard,
            selector(c"setPropertyList:forType:"),
            property_list,
            pasteboard_type,
        )
    };
    if ok == NO {
        Err(String::from("NSPasteboard setPropertyList failed"))
    } else {
        Ok(())
    }
}

unsafe fn set_pasteboard_string(
    pasteboard: Id,
    string: Id,
    pasteboard_type: &str,
) -> Result<(), String> {
    let pasteboard_type = unsafe { ns_string(pasteboard_type)? };
    unsafe { set_pasteboard_string_id(pasteboard, string, pasteboard_type) }
}

unsafe fn set_pasteboard_string_id(
    pasteboard: Id,
    string: Id,
    pasteboard_type: Id,
) -> Result<(), String> {
    let ok = unsafe {
        msg_bool_id_id(
            pasteboard,
            selector(c"setString:forType:"),
            string,
            pasteboard_type,
        )
    };
    if ok == NO {
        Err(String::from("NSPasteboard setString failed"))
    } else {
        Ok(())
    }
}

unsafe fn read_file_paths_from_pasteboard(pasteboard: Id) -> Result<Vec<PathBuf>, String> {
    let filenames_type = unsafe { ns_string(NS_FILENAMES_PBOARD_TYPE)? };
    let filenames = unsafe {
        msg_id_id(
            pasteboard,
            selector(c"propertyListForType:"),
            filenames_type,
        )
    };
    if !filenames.is_null() {
        return unsafe { ns_string_array_to_paths(filenames) };
    }

    let file_url_type = unsafe { ns_string(NSPASTEBOARD_TYPE_FILE_URL)? };
    let file_url = unsafe { msg_id_id(pasteboard, selector(c"stringForType:"), file_url_type) };
    if !file_url.is_null() {
        let url = unsafe { ns_string_to_rust(file_url)? };
        return file_url_string_to_path(&url).map(|path| vec![path]);
    }

    let legacy_url_type = unsafe { ns_string(NS_URL_PBOARD_TYPE)? };
    let legacy_url = unsafe { msg_id_id(pasteboard, selector(c"stringForType:"), legacy_url_type) };
    if !legacy_url.is_null() {
        let url = unsafe { ns_string_to_rust(legacy_url)? };
        return file_url_string_to_path(&url).map(|path| vec![path]);
    }

    Ok(Vec::new())
}

unsafe fn ns_string_array_to_paths(array: Id) -> Result<Vec<PathBuf>, String> {
    let count = unsafe { msg_usize(array, selector(c"count")) };
    let mut paths = Vec::with_capacity(count);
    for index in 0..count {
        let value = unsafe { msg_id_usize(array, selector(c"objectAtIndex:"), index) };
        if value.is_null() {
            continue;
        }
        let path = unsafe { ns_string_to_rust(value)? };
        if !path.is_empty() {
            paths.push(PathBuf::from(path));
        }
    }
    Ok(paths)
}

unsafe fn ns_string_to_rust(string: Id) -> Result<String, String> {
    let bytes = unsafe { msg_ptr(string, selector(c"UTF8String")) };
    if bytes.is_null() {
        return Ok(String::new());
    }
    unsafe { CStr::from_ptr(bytes.cast()) }
        .to_str()
        .map(ToOwned::to_owned)
        .map_err(|err| format!("Clipboard text is not valid UTF-8: {err}"))
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

unsafe fn msg_id_usize(receiver: Id, selector: Sel, arg: usize) -> Id {
    let msg: unsafe extern "C" fn(Id, Sel, usize) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, arg) }
}

unsafe fn msg_usize(receiver: Id, selector: Sel) -> usize {
    let msg: unsafe extern "C" fn(Id, Sel) -> usize =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}

unsafe fn msg_ptr(receiver: Id, selector: Sel) -> *const c_char {
    let msg: unsafe extern "C" fn(Id, Sel) -> *const c_char =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
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

unsafe fn msg_isize(receiver: Id, selector: Sel) -> isize {
    let msg: unsafe extern "C" fn(Id, Sel) -> isize =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector) }
}

unsafe fn msg_bool_id(receiver: Id, selector: Sel, arg: Id) -> ObjcBool {
    let msg: unsafe extern "C" fn(Id, Sel, Id) -> ObjcBool =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, arg) }
}

unsafe fn msg_bool_id_id(receiver: Id, selector: Sel, first: Id, second: Id) -> ObjcBool {
    let msg: unsafe extern "C" fn(Id, Sel, Id, Id) -> ObjcBool =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { msg(receiver, selector, first, second) }
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

fn path_to_pasteboard_string(path: &Path) -> Result<String, String> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        format!(
            "Cannot copy non-UTF-8 path to clipboard: {}",
            path.display()
        )
    })
}

fn file_url_string_to_path(url: &str) -> Result<PathBuf, String> {
    let Some(rest) = url.strip_prefix("file://") else {
        return Err(format!("Clipboard URL is not a file URL: {url}"));
    };
    let path = rest
        .strip_prefix("localhost/")
        .map(|path| format!("/{path}"))
        .or_else(|| rest.strip_prefix('/').map(|path| format!("/{path}")))
        .ok_or_else(|| format!("Clipboard file URL has an unsupported host: {url}"))?;
    let path = path.split(['?', '#']).next().unwrap_or(path.as_str());
    percent_decode_utf8(path).map(PathBuf::from)
}

fn percent_decode_utf8(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(format!("Clipboard URL has incomplete escape: {value}"));
            }
            let high = hex_value(bytes[index + 1])
                .ok_or_else(|| format!("Clipboard URL has invalid escape: {value}"))?;
            let low = hex_value(bytes[index + 2])
                .ok_or_else(|| format!("Clipboard URL has invalid escape: {value}"))?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|err| format!("Clipboard URL path is not UTF-8: {err}"))
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{file_url_string_to_path, path_to_pasteboard_string};
    use std::path::Path;

    #[test]
    fn path_to_pasteboard_string_keeps_absolute_path_text() {
        assert_eq!(
            path_to_pasteboard_string(Path::new("/tmp/wavecrate clip.wav")).unwrap(),
            "/tmp/wavecrate clip.wav"
        );
    }

    #[test]
    fn file_url_string_to_path_decodes_file_url_paths() {
        assert_eq!(
            file_url_string_to_path("file:///tmp/wavecrate%20clip.wav").unwrap(),
            Path::new("/tmp/wavecrate clip.wav")
        );
    }

    #[test]
    fn file_url_string_to_path_rejects_non_file_urls() {
        assert!(
            file_url_string_to_path("https://example.test/kick.wav")
                .unwrap_err()
                .contains("not a file URL")
        );
    }
}
