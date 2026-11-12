//! Cross-platform image picker: native UIImagePickerController on iOS,
//! hidden file input + FileReader on Web and other platforms.

/// Opens the system/browser photo picker and returns the image as a Base64 data URL.
/// e.g. "data:image/jpeg;base64,..."
pub async fn pick_image() -> Option<String> {
    #[cfg(target_os = "ios")]
    {
        use std::ffi::CStr;
        use std::os::raw::{c_char, c_void};
        use futures_channel::oneshot;

        type Callback = unsafe extern "C" fn(*const c_char, *mut c_void);

        unsafe extern "C" fn on_picked(c_str: *const c_char, ctx: *mut c_void) {
            let tx = unsafe { Box::from_raw(ctx as *mut oneshot::Sender<Option<String>>) };
            let res = if c_str.is_null() {
                None
            } else {
                let s = unsafe { CStr::from_ptr(c_str) }.to_string_lossy().into_owned();
                Some(s)
            };
            let _ = tx.send(res);
        }

        unsafe extern "C" {
            fn pe_pick_image_ios(ctx: *mut c_void, cb: Callback);
        }

        let (tx, rx) = oneshot::channel::<Option<String>>();
        let ctx = Box::into_raw(Box::new(tx)) as *mut c_void;
        unsafe {
            pe_pick_image_ios(ctx, on_picked);
        }
        rx.await.unwrap_or(None)
    }

    #[cfg(not(target_os = "ios"))]
    {
        let js = r#"
            (async () => {
                return new Promise((resolve) => {
                    const input = document.createElement('input');
                    input.type = 'file';
                    input.accept = 'image/*';
                    input.style.display = 'none';
                    document.body.appendChild(input);
                    input.onchange = () => {
                        const file = input.files && input.files[0];
                        if (!file) {
                            try { document.body.removeChild(input); } catch (_) {}
                            dioxus.send("");
                            return;
                        }
                        const reader = new FileReader();
                        reader.onload = (e) => {
                            try { document.body.removeChild(input); } catch (_) {}
                            dioxus.send(e.target.result || "");
                        };
                        reader.onerror = () => {
                            try { document.body.removeChild(input); } catch (_) {}
                            dioxus.send("");
                        };
                        reader.readAsDataURL(file);
                    };
                    input.click();
                });
            })();
        "#;
        let mut eval = dioxus::document::eval(js);
        if let Ok(val) = eval.recv::<String>().await {
            if val.starts_with("data:image/") {
                return Some(val);
            }
        }
        None
    }
}
