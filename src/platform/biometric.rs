//! Biometric authentication (Face ID / Touch ID / fingerprint).
//!
//! * iOS: `LAContext` (LocalAuthentication) — fully implemented.
//! * Android: framework `BiometricPrompt` (API 28+). Its callback is an abstract Java
//!   class, which pure JNI cannot implement, so a tiny Java bridge
//!   (`android/bio/BioAuth.java`) is pre-compiled to `classes.dex`, embedded in the
//!   binary and loaded at runtime with `InMemoryDexClassLoader` (API 26+).
//! * Web: unavailable (WebAuthn could be added later).

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum BioResult {
    Success,
    Failed,
    Unavailable,
}

#[cfg(target_os = "ios")]
mod imp {
    use super::BioResult;
    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSError, NSString};
    use objc2_local_authentication::{LAContext, LAPolicy};
    use std::sync::Mutex;

    pub fn available() -> bool {
        unsafe {
            let ctx = LAContext::new();
            ctx.canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics).is_ok()
        }
    }

    pub async fn authenticate(reason: &str, _cancel: &str) -> BioResult {
        if !available() {
            return BioResult::Unavailable;
        }
        let (tx, rx) = futures_channel::oneshot::channel::<bool>();
        let tx = Mutex::new(Some(tx));
        let block = RcBlock::new(move |ok: Bool, _err: *mut NSError| {
            if let Some(tx) = tx.lock().ok().and_then(|mut g| g.take()) {
                let _ = tx.send(ok.as_bool());
            }
        });
        unsafe {
            let ctx = LAContext::new();
            let reason = NSString::from_str(reason);
            ctx.evaluatePolicy_localizedReason_reply(LAPolicy::DeviceOwnerAuthenticationWithBiometrics, &reason, &block);
        }
        match rx.await {
            Ok(true) => BioResult::Success,
            _ => BioResult::Failed,
        }
    }
}

#[cfg(target_os = "android")]
mod imp {
    use super::BioResult;
    use jni::objects::{GlobalRef, JClass, JObject, JValue};
    use jni::{JNIEnv, JavaVM};
    use std::sync::OnceLock;

    static DEX: &[u8] = include_bytes!("../../android/bio/classes.dex");
    const CLASS: &str = "com.photoexchanger.bio.BioAuth";

    fn vm_and_activity() -> Option<(JavaVM, JObject<'static>)> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
        let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
        Some((vm, activity))
    }

    /// Runs `f`, clearing any pending Java exception on failure.
    fn guarded<T>(env: &mut JNIEnv, f: impl FnOnce(&mut JNIEnv) -> jni::errors::Result<T>) -> Option<T> {
        let r = f(env);
        if r.is_err() && env.exception_check().unwrap_or(false) {
            let _ = env.exception_describe();
            let _ = env.exception_clear();
        }
        r.ok()
    }

    /// Loads (once) the embedded BioAuth class and returns a global reference to it.
    fn bio_class(env: &mut JNIEnv, activity: &JObject) -> Option<&'static GlobalRef> {
        static CLS: OnceLock<GlobalRef> = OnceLock::new();
        if let Some(c) = CLS.get() {
            return Some(c);
        }
        let cls = guarded(env, |env| {
            let sdk = env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?.i()?;
            if sdk < 28 {
                return Err(jni::errors::Error::JavaException);
            }
            // The dex loader only reads the buffer; DEX lives for the whole program.
            let buf = unsafe { env.new_direct_byte_buffer(DEX.as_ptr() as *mut u8, DEX.len())? };
            let parent = env.call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?.l()?;
            let loader = env.new_object(
                "dalvik/system/InMemoryDexClassLoader",
                "(Ljava/nio/ByteBuffer;Ljava/lang/ClassLoader;)V",
                &[JValue::Object(&buf), JValue::Object(&parent)],
            )?;
            let name = env.new_string(CLASS)?;
            let cls = env
                .call_method(&loader, "loadClass", "(Ljava/lang/String;)Ljava/lang/Class;", &[JValue::Object(&name)])?
                .l()?;
            env.new_global_ref(cls)
        })?;
        Some(CLS.get_or_init(|| cls))
    }

    pub fn available() -> bool {
        let Some((vm, activity)) = vm_and_activity() else { return false };
        let Ok(mut env) = vm.attach_current_thread() else { return false };
        let Some(cls) = bio_class(&mut env, &activity) else { return false };
        guarded(&mut env, |env| {
            let cls = JClass::from(env.new_local_ref(cls.as_obj())?);
            env.call_static_method(&cls, "available", "(Landroid/content/Context;)Z", &[JValue::Object(&activity)])?.z()
        })
        .unwrap_or(false)
    }

    pub async fn authenticate(reason: &str, cancel: &str) -> BioResult {
        if !available() {
            return BioResult::Unavailable;
        }
        // Start the prompt (shown on the UI thread by the Java side).
        let prompt: GlobalRef = {
            let Some((vm, activity)) = vm_and_activity() else { return BioResult::Unavailable };
            let Ok(mut env) = vm.attach_current_thread() else { return BioResult::Unavailable };
            let Some(cls) = bio_class(&mut env, &activity) else { return BioResult::Unavailable };
            let started = guarded(&mut env, |env| {
                let cls = JClass::from(env.new_local_ref(cls.as_obj())?);
                let title = env.new_string(reason)?;
                let neg = env.new_string(cancel)?;
                let obj = env
                    .call_static_method(
                        &cls,
                        "start",
                        "(Landroid/app/Activity;Ljava/lang/String;Ljava/lang/String;)Lcom/photoexchanger/bio/BioAuth;",
                        &[JValue::Object(&activity), JValue::Object(&title), JValue::Object(&neg)],
                    )?
                    .l()?;
                env.new_global_ref(obj)
            });
            match started {
                Some(p) => p,
                None => return BioResult::Unavailable,
            }
        };

        // Poll the volatile `result` field (0 = pending) for up to 2 minutes.
        for i in 0..1200 {
            crate::platform::sleep_ms(100).await;
            let Some((vm, _)) = vm_and_activity() else { return BioResult::Failed };
            let Ok(mut env) = vm.attach_current_thread() else { return BioResult::Failed };
            if i == 1199 {
                let _ = guarded(&mut env, |env| env.call_method(prompt.as_obj(), "cancel", "()V", &[]).map(|_| ()));
                return BioResult::Failed;
            }
            match guarded(&mut env, |env| env.get_field(prompt.as_obj(), "result", "I")?.i()) {
                Some(0) => continue,
                Some(1) => return BioResult::Success,
                Some(3) | None => return BioResult::Unavailable,
                Some(_) => return BioResult::Failed,
            }
        }
        BioResult::Failed
    }
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod imp {
    use super::BioResult;
    pub fn available() -> bool {
        false
    }
    pub async fn authenticate(_reason: &str, _cancel: &str) -> BioResult {
        BioResult::Unavailable
    }
}

pub use imp::*;
