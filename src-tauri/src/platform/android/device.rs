//! JNI transport for Android ceremony operations that do not require Tauri.

use crate::error::{AppError, Result};
use jni::objects::{GlobalRef, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};
use serde::{Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

struct Device {
    jvm: JavaVM,
    host: GlobalRef,
    data_dir: PathBuf,
    language: String,
}

static DEVICE: OnceLock<Device> = OnceLock::new();

fn connect(
    env: &mut JNIEnv<'_>,
    host: &JObject<'_>,
    directory: &JString<'_>,
    language: &JString<'_>,
) -> jni::errors::Result<()> {
    if DEVICE.get().is_none() {
        let directory: String = env.get_string(directory)?.into();
        let language: String = env.get_string(language)?.into();
        let _ = DEVICE.set(Device {
            jvm: env.get_java_vm()?,
            host: env.new_global_ref(host)?,
            data_dir: PathBuf::from(directory),
            language,
        });
    }
    Ok(())
}

#[unsafe(no_mangle)]
extern "system" fn Java_ua_pp_khvylyna_MinuteOfSilence_CeremonyDevice_connectNative(
    mut env: JNIEnv<'_>,
    host: JObject<'_>,
    directory: JString<'_>,
    language: JString<'_>,
) {
    if let Err(error) = connect(&mut env, &host, &directory, &language) {
        let _ = env.throw_new("java/lang/IllegalStateException", error.to_string());
    }
}

pub(super) fn data_dir() -> Option<&'static Path> {
    DEVICE.get().map(|device| device.data_dir.as_path())
}

pub(super) fn language() -> Option<&'static str> {
    DEVICE.get().map(|device| device.language.as_str())
}

/// Invokes a native operation on the application-scoped Android host.
pub fn call<T: DeserializeOwned>(command: &str, payload: impl Serialize) -> Result<T> {
    let device = DEVICE
        .get()
        .ok_or_else(|| AppError::Platform("Android ceremony device is not connected".into()))?;
    let failure =
        |error| AppError::Platform(format!("Android command `{command}` failed: {error}"));
    let payload = serde_json::to_string(&payload)?;
    let mut env = device.jvm.attach_current_thread().map_err(failure)?;
    let response = env.with_local_frame(8, |env| -> jni::errors::Result<String> {
        let command = env.new_string(command)?;
        let payload = env.new_string(payload)?;
        let response = env
            .call_method(
                device.host.as_obj(),
                "callFromNative",
                "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
                &[JValue::Object(&command), JValue::Object(&payload)],
            )?
            .l()?;
        let response = JString::from(response);
        Ok(env.get_string(&response)?.into())
    });
    if matches!(response, Err(jni::errors::Error::JavaException)) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    Ok(serde_json::from_str(&response.map_err(failure)?)?)
}
