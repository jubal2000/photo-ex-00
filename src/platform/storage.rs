//! Persistent storage.
//!
//! * `get`/`set`/`remove` — plain preferences (language, flags).
//! * `secure_get`/`secure_set`/`secure_remove` — the Firebase refresh token used for
//!   auto-login. iOS: Keychain. Android: app-private internal storage (sandboxed to the
//!   app). Web: localStorage (browsers offer no stronger primitive).

const SERVICE: &str = "com.photoexchanger.app";

// ---------------- web ----------------
#[cfg(target_arch = "wasm32")]
mod imp {
    fn ls() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }
    pub fn get(key: &str) -> Option<String> {
        ls()?.get_item(key).ok()?
    }
    pub fn set(key: &str, val: &str) {
        if let Some(s) = ls() {
            let _ = s.set_item(key, val);
        }
    }
    pub fn remove(key: &str) {
        if let Some(s) = ls() {
            let _ = s.remove_item(key);
        }
    }
    pub fn secure_get(key: &str) -> Option<String> {
        get(&format!("secure.{key}"))
    }
    pub fn secure_set(key: &str, val: &str) {
        set(&format!("secure.{key}"), val)
    }
    pub fn secure_remove(key: &str) {
        remove(&format!("secure.{key}"))
    }
}

// ---------------- native (file based prefs) ----------------
#[cfg(not(target_arch = "wasm32"))]
mod files {
    use std::path::PathBuf;

    #[cfg(target_os = "android")]
    fn base_dir() -> Option<PathBuf> {
        use jni::objects::{JObject, JString};
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let context = unsafe { JObject::from_raw(ctx.context().cast()) };
        let dir = env.call_method(&context, "getFilesDir", "()Ljava/io/File;", &[]).ok()?.l().ok()?;
        let path = env.call_method(&dir, "getAbsolutePath", "()Ljava/lang/String;", &[]).ok()?.l().ok()?;
        let path: String = env.get_string(&JString::from(path)).ok()?.into();
        Some(PathBuf::from(path))
    }

    #[cfg(not(target_os = "android"))]
    fn base_dir() -> Option<PathBuf> {
        // iOS: <app sandbox>/Library/Application Support ; desktop: ~/.photo_exchanger
        let home = std::env::var("HOME").ok()?;
        let p = if cfg!(target_os = "ios") {
            PathBuf::from(home).join("Library").join("Application Support")
        } else {
            PathBuf::from(home).join(".photo_exchanger")
        };
        Some(p)
    }

    fn path(key: &str) -> Option<PathBuf> {
        let dir = base_dir()?.join("prefs");
        std::fs::create_dir_all(&dir).ok()?;
        let safe: String = key.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' { c } else { '_' }).collect();
        Some(dir.join(safe))
    }

    pub fn get(key: &str) -> Option<String> {
        // An empty file means an interrupted/incomplete write: treat as unset.
        std::fs::read_to_string(path(key)?).ok().filter(|s| !s.is_empty())
    }
    /// Atomic write: temp file + fsync + rename, so a crash never leaves a truncated value.
    pub fn set(key: &str, val: &str) {
        use std::io::Write;
        let Some(p) = path(key) else { return };
        let tmp = p.with_extension("tmp");
        let ok = std::fs::File::create(&tmp)
            .and_then(|mut f| {
                f.write_all(val.as_bytes())?;
                f.sync_all()
            })
            .is_ok();
        if ok {
            let _ = std::fs::rename(&tmp, &p);
        }
    }
    pub fn remove(key: &str) {
        if let Some(p) = path(key) {
            let _ = std::fs::remove_file(p);
        }
    }
}

#[cfg(target_os = "ios")]
mod imp {
    pub use super::files::{get, remove, set};
    use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};

    pub fn secure_get(key: &str) -> Option<String> {
        get_generic_password(super::SERVICE, key).ok().and_then(|b| String::from_utf8(b).ok())
    }
    pub fn secure_set(key: &str, val: &str) {
        let _ = set_generic_password(super::SERVICE, key, val.as_bytes());
    }
    pub fn secure_remove(key: &str) {
        let _ = delete_generic_password(super::SERVICE, key);
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "ios")))]
mod imp {
    pub use super::files::{get, remove, set};
    pub fn secure_get(key: &str) -> Option<String> {
        get(&format!("secure.{}.{key}", super::SERVICE))
    }
    pub fn secure_set(key: &str, val: &str) {
        set(&format!("secure.{}.{key}", super::SERVICE), val)
    }
    pub fn secure_remove(key: &str) {
        remove(&format!("secure.{}.{key}", super::SERVICE))
    }
}

#[allow(unused)]
const _: &str = SERVICE;

pub use imp::*;
