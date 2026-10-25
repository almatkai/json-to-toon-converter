// Generated from immutable beta fixtures by tools/prepare_harness.py.
#![allow(dead_code)] // Host modules expose APIs unrelated to this one-command harness.
pub mod native_core {
    #[path = "../../generated/src-tauri/src/native_core/common.rs"]
    pub mod common;
    pub mod extensions {
        #[path = "../../../generated/src-tauri/src/native_core/extensions/native.rs"]
        mod native;
        #[path = "../../../generated/src-tauri/src/native_core/extensions/registry.rs"]
        mod registry;
        #[path = "../../../generated/src-tauri/src/native_core/extensions/v2_catalog.rs"]
        mod v2_catalog;
        #[path = "../../../generated/src-tauri/native-extensions/json-to-toon-converter/adapter.rs"]
        mod v2_json_to_toon;
        #[path = "../../../generated/src-tauri/src/native_core/extensions/v2_runtime.rs"]
        mod v2_runtime;
        #[path = "../../../generated/src-tauri/src/native_core/extensions/v2_sdk.rs"]
        mod v2_sdk;
        #[cfg(test)]
        mod perplexity {}
        mod v2_amphetamine {
            use crate::native_core::common::{CoreContext, CoreResult};
            use serde_json::Value;
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &CoreContext,
                _: &str,
                _: &str,
                _: &Value,
                _: &mut Value,
            ) -> CoreResult {
                Err("Unrelated adapter is outside this harness".into())
            }
        }
        mod v2_audio_device {
            use crate::native_core::common::{CoreContext, CoreResult};
            use serde_json::Value;
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &CoreContext,
                _: &str,
                _: &str,
                _: &Value,
                _: &mut Value,
            ) -> CoreResult {
                Err("Unrelated adapter is outside this harness".into())
            }
        }
        mod v2_cleanshotx {
            use crate::native_core::common::{CoreContext, CoreResult};
            use serde_json::Value;
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &CoreContext,
                _: &str,
                _: &str,
                _: &Value,
                _: &mut Value,
            ) -> CoreResult {
                Err("Unrelated adapter is outside this harness".into())
            }
        }
        mod v2_color_picker {
            use crate::native_core::common::{CoreContext, CoreResult};
            use serde_json::Value;
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &CoreContext,
                _: &str,
                _: &str,
                _: &Value,
                _: &mut Value,
            ) -> CoreResult {
                Err("Unrelated adapter is outside this harness".into())
            }
        }
        mod v2_gif_search {
            use crate::native_core::common::{CoreContext, CoreResult};
            use serde_json::Value;
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &CoreContext,
                _: &str,
                _: &str,
                _: &Value,
                _: &mut Value,
            ) -> CoreResult {
                Err("Unrelated adapter is outside this harness".into())
            }
        }
        mod v2_google_calendar {
            use crate::native_core::common::{CoreContext, CoreResult};
            use serde_json::Value;
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &CoreContext,
                _: &str,
                _: &str,
                _: &Value,
                _: &mut Value,
            ) -> CoreResult {
                Err("Unrelated adapter is outside this harness".into())
            }
        }
        mod v2_google_search {
            use crate::native_core::common::{CoreContext, CoreResult};
            use serde_json::Value;
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &CoreContext,
                _: &str,
                _: &str,
                _: &Value,
                _: &mut Value,
            ) -> CoreResult {
                Err("Unrelated adapter is outside this harness".into())
            }
        }
        mod v2_port_manager {
            use crate::native_core::common::{CoreContext, CoreResult};
            use serde_json::Value;
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &CoreContext,
                _: &str,
                _: &str,
                _: &Value,
                _: &mut Value,
            ) -> CoreResult {
                Err("Unrelated adapter is outside this harness".into())
            }
        }
        mod v2_speedtest {
            use crate::native_core::common::{CoreContext, CoreResult};
            use serde_json::Value;
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &CoreContext,
                _: &str,
                _: &str,
                _: &Value,
                _: &mut Value,
            ) -> CoreResult {
                Err("Unrelated adapter is outside this harness".into())
            }
        }
        mod v2_timers {
            use crate::native_core::common::{CoreContext, CoreResult};
            use serde_json::Value;
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &CoreContext,
                _: &str,
                _: &str,
                _: &Value,
                _: &mut Value,
            ) -> CoreResult {
                Err("Unrelated adapter is outside this harness".into())
            }
        }
        mod v2_perplexity {
            use crate::native_core::common::{CoreContext, CoreResult};
            use serde_json::Value;
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &CoreContext,
                _: &str,
                _: &str,
                _: &Value,
                _: &mut Value,
            ) -> CoreResult {
                Err("Unrelated adapter is outside this harness".into())
            }
        }
        #[cfg(test)]
        #[path = "../../../tests.rs"]
        mod tests;
    }
}
