use super::{native::NativeSessions, registry, v2_catalog, v2_json_to_toon as adapter, v2_runtime};
use crate::native_core::common::{write_json_atomic, CoreContext, CorePaths};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

const ID: &str = "com.tezbar.native.json-to-toon-converter";
const ALIAS: &str = "raycast.json-to-toon-converter";
const COMMAND: &str = "convert-json";
#[derive(Clone)]
enum Reply {
    Value(Value),
    Error,
    Wait(Arc<Notify>, Arc<Notify>, Value),
}
type CallLog = Arc<Mutex<Vec<(String, Vec<Value>)>>>;
struct Fixture {
    _dir: tempfile::TempDir,
    ctx: CoreContext,
    sessions: NativeSessions,
    files: Arc<Mutex<()>>,
    replies: Arc<Mutex<VecDeque<Reply>>>,
    calls: CallLog,
}
impl Fixture {
    fn new(prefs: Value) -> Self {
        assert!(
            std::env::var_os("SUPERCMD_EXTENSION_PATHS").is_none(),
            "Unset external extension roots for isolated headless tests"
        );
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let ctx = CoreContext::headless(CorePaths {
            data_dir: root.join("data"),
            config_dir: root.join("config"),
            resource_dir: root.join("resources"),
            temp_dir: root.join("tmp"),
            home_dir: root.join("home"),
        });
        let replies = Arc::new(Mutex::new(VecDeque::<Reply>::new()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (queue, log) = (replies.clone(), calls.clone());
        ctx.set_router(Arc::new(move |channel, args| {
            log.lock().unwrap().push((channel, args));
            let reply = queue
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected OS request");
            Box::pin(async move {
                match reply {
                    Reply::Value(v) => Ok(v),
                    Reply::Error => Err("private clipboard secret".into()),
                    Reply::Wait(entered, release, v) => {
                        entered.notify_one();
                        release.notified().await;
                        Ok(v)
                    }
                }
            })
        }))
        .unwrap();
        v2_catalog::install(&ctx, ID).unwrap().unwrap();
        write_json_atomic(
            &ctx.paths
                .data_dir
                .join("extensions/packages")
                .join(ID)
                .join("preferences.json"),
            &prefs,
        )
        .unwrap();
        Self {
            _dir: dir,
            ctx,
            sessions: NativeSessions::default(),
            files: Arc::new(Mutex::new(())),
            replies,
            calls,
        }
    }
    fn add(&self, v: Value) {
        self.replies.lock().unwrap().push_back(Reply::Value(v));
    }
    fn error(&self) {
        self.replies.lock().unwrap().push_back(Reply::Error);
    }
    async fn ipc(&self, channel: &str, body: Value) -> Value {
        self.sessions
            .handle(&self.ctx, &self.files, channel, &[body])
            .await
            .expect("native dispatch")
            .unwrap()
    }
    async fn launch(&self, id: &str) -> Value {
        self.ipc(
            "extension:run-command",
            json!({"extensionId":id,"commandName":COMMAND,"argumentValues":{"json":"DO NOT READ"}}),
        )
        .await
    }
    async fn invoke(&self, view: &Value) -> Value {
        self.ipc("extension:invoke-action", json!({"sessionId":view["sessionId"],"actionId":view["actions"][0]["id"],"formValues":{"value":"FORGED OUTPUT"},"content":"FORGED OUTPUT"})).await
    }
    async fn refresh(&self, view: &Value) -> Value {
        self.ipc(
            "extension:refresh-session",
            json!({"sessionId":view["sessionId"]}),
        )
        .await
    }
    fn channels(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.0.clone())
            .collect()
    }
}

#[tokio::test]
async fn actual_registry_legacy_preferences_coexistence_and_uninstalled_canonical() {
    let f = Fixture::new(json!({}));
    let own = f
        .ctx
        .paths
        .data_dir
        .join("extensions/packages")
        .join(ID)
        .join("preferences.json");
    // No deletion: use a second fresh temporary context where native prefs are absent.
    assert!(own.is_file());
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    let ctx = CoreContext::headless(CorePaths {
        data_dir: p.join("data"),
        config_dir: p.join("config"),
        resource_dir: p.join("resources"),
        temp_dir: p.join("tmp"),
        home_dir: p.join("home"),
    });
    let legacy = ctx.paths.data_dir.join("extensions/json-to-toon-converter");
    write_json_atomic(&legacy.join("package.json"),&json!({"name":"json-to-toon-converter","commands":[{"name":COMMAND},{"name":"legacy-only"}]})).unwrap();
    write_json_atomic(
        &legacy.join("preferences.json"),
        &json!({"inputSource":"clipboard","unknown":"preserved"}),
    )
    .unwrap();
    assert!(registry::installed_command(&ctx, ID, COMMAND).is_none());
    v2_catalog::install(&ctx, ID).unwrap().unwrap();
    let installed = registry::installed_command(&ctx, ALIAS, COMMAND).unwrap();
    let prefs = registry::preferences(&installed, Some(COMMAND)).unwrap();
    assert_eq!(prefs["inputSource"], "clipboard");
    assert_eq!(prefs["unknown"], "preserved");
    assert_eq!(prefs["defaultAction"], "copy");
    assert!(registry::installed_command(&ctx, ALIAS, "legacy-only").is_some());
    assert!(registry::installed_command(&ctx, ID, "legacy-only").is_none());
    let before = std::fs::read(legacy.join("preferences.json")).unwrap();
    let _ = registry::preferences(&installed, None).unwrap();
    assert_eq!(
        std::fs::read(legacy.join("preferences.json")).unwrap(),
        before
    );
}

fn no_view(v: &Value, message: &str) {
    assert_eq!(v, &json!({"ok":true,"mode":"no-view","message":message}));
}
fn uncertain(v: &Value) {
    assert_eq!(v["mode"], "view");
    assert_eq!(v["root"]["type"], "Detail");
    assert!(v["message"].as_str().unwrap().contains("unconfirmed"));
    assert!(v["message"].as_str().unwrap().contains("duplicate"));
    assert_eq!(v["actions"].as_array().unwrap().len(), 1);
    assert!(!v.to_string().contains("private"));
}
#[tokio::test]
async fn real_registry_catalog_alias_defaults_and_automatic_copy_without_session() {
    let f = Fixture::new(json!({}));
    assert_eq!(v2_catalog::manifests().len(), 12);
    assert!(v2_runtime::supports(ID, COMMAND));
    assert!(v2_runtime::supports(ALIAS, COMMAND));
    assert!(!v2_runtime::supports(ID, "other"));
    assert_eq!(
        v2_catalog::manifest("json-to-toon-converter").unwrap()["commands"][0]["mode"],
        "no-view"
    );
    let prefs = registry::preferences(
        &registry::installed_command(&f.ctx, ALIAS, COMMAND).unwrap(),
        Some(COMMAND),
    )
    .unwrap();
    assert_eq!(prefs["defaultAction"], "copy");
    assert_eq!(prefs["inputSource"], "auto");
    // More than NativeSessions' 128-session cap: successful launches register none.
    for i in 0..130 {
        f.add(json!("{\"b\":1,\"2\":2,\"a\":3}"));
        f.add(json!({"ok":true}));
        let done = f.launch(if i % 2 == 0 { ID } else { ALIAS }).await;
        no_view(&done, "Converted! TOON format copied to clipboard");
    }
    let calls = f.calls.lock().unwrap();
    for pair in calls.chunks_exact(2) {
        assert_eq!(pair[0], ("core:platform:selected-text".into(), vec![]));
        assert_eq!(
            pair[1],
            (
                "clipboard:write".into(),
                vec![json!("\"2\": 2\nb: 1\na: 3")]
            )
        );
    }
}
#[tokio::test]
async fn fallback_invalid_and_empty_keep_source_order_without_confirmation() {
    for selected in [None, Some("\u{feff}\u{00a0}\n")] {
        let f = Fixture::new(json!({}));
        if let Some(s) = selected {
            f.add(json!(s));
        } else {
            f.error();
        }
        f.add(json!("[1,2]"));
        f.add(json!({"ok":true}));
        no_view(
            &f.launch(ID).await,
            "Converted! TOON format copied to clipboard",
        );
        assert_eq!(
            f.channels(),
            [
                "core:platform:selected-text",
                "clipboard:read",
                "clipboard:write"
            ]
        );
        assert_eq!(f.calls.lock().unwrap()[2].1, vec![json!("[2]: 1,2")]);
    }
    for selected in ["bad private source", "\u{0085}"] {
        let f = Fixture::new(json!({}));
        f.add(json!(selected));
        let result = f.launch(ID).await;
        assert_eq!(result["mode"], "no-view");
        assert!(result["message"].as_str().unwrap().contains("Invalid JSON"));
        assert!(!result.to_string().contains(selected));
        assert_eq!(f.channels(), ["core:platform:selected-text"]);
    }
    let f = Fixture::new(json!({}));
    f.add(json!(""));
    f.add(json!("\t\n"));
    no_view(
        &f.launch(ID).await,
        "No JSON found - selection and clipboard are empty",
    );
    assert_eq!(
        f.channels(),
        ["core:platform:selected-text", "clipboard:read"]
    );
}
#[tokio::test]
async fn strict_typed_preferences_and_automatic_paste_have_one_effect() {
    for bad in [
        json!(false),
        json!(null),
        json!(17),
        json!({}),
        json!([]),
        json!("unexpected"),
        json!("paste"),
    ] {
        let f = Fixture::new(json!({"defaultAction":bad,"inputSource":bad}));
        f.add(json!("{}"));
        f.add(json!({"ok":true,"success":true,"error":null}));
        no_view(
            &f.launch(ALIAS).await,
            "Converted! TOON format pasted to active app",
        );
        assert_eq!(f.channels(), ["clipboard:read", "core:platform:paste-text"]);
        assert_eq!(f.calls.lock().unwrap()[1].1, vec![json!("")]);
    }
}
#[tokio::test]
async fn strict_ack_matrix_commits_recovery_and_retries_identical_bytes_without_reads() {
    for action in ["copy", "paste"] {
        for reply in [
            Some(json!(null)),
            Some(json!(true)),
            Some(json!(false)),
            Some(json!({})),
            Some(json!({"ok":false})),
            Some(json!({"ok":true,"error":"private secret"})),
            Some(json!({"ok":true,"success":false})),
            Some(json!({"ok":true,"success":null})),
            Some(json!({"ok":true,"error":false})),
            Some(json!({"success":true})),
            None,
        ] {
            let f = Fixture::new(json!({"inputSource":"clipboard","defaultAction":action}));
            f.add(json!("\"<https://evil.invalid>```\""));
            if let Some(v) = reply {
                f.add(v);
            } else {
                f.error();
            }
            let retry = f.launch(ID).await;
            uncertain(&retry);
            assert!(retry["sessionId"]
                .as_str()
                .unwrap()
                .starts_with("native-extension:"));
            assert!(!retry.to_string().contains("evil.invalid"));
            for op in [
                "extension:refresh-session",
                "extension:search-text-changed",
                "extension:load-more",
            ] {
                let view = f
                    .ipc(
                        op,
                        json!({"sessionId":retry["sessionId"],"searchText":"FORGED"}),
                    )
                    .await;
                assert_eq!(view["actions"], retry["actions"]);
            }
            f.add(json!({"ok":true}));
            let done = f.invoke(&retry).await;
            no_view(
                &done,
                if action == "copy" {
                    "Converted! TOON format copied to clipboard"
                } else {
                    "Converted! TOON format pasted to active app"
                },
            );
            assert_eq!(f.invoke(&retry).await["ok"], false);
            let route = if action == "copy" {
                "clipboard:write"
            } else {
                "core:platform:paste-text"
            };
            assert_eq!(f.channels(), ["clipboard:read", route, route]);
            let calls = f.calls.lock().unwrap();
            assert_eq!(calls[1].1, calls[2].1);
        }
    }
}
#[tokio::test]
async fn positive_noncontradictory_ack_variants_return_no_view() {
    for ack in [
        json!({"ok":true}),
        json!({"ok":true,"error":null}),
        json!({"ok":true,"success":true}),
        json!({"ok":true,"success":true,"error":null}),
    ] {
        let f = Fixture::new(json!({}));
        f.add(json!("1"));
        f.add(ack);
        no_view(
            &f.launch(ID).await,
            "Converted! TOON format copied to clipboard",
        );
        assert_eq!(
            f.channels(),
            ["core:platform:selected-text", "clipboard:write"]
        );
    }
}
#[tokio::test]
async fn recovery_rejects_stale_forged_cross_session_ids_and_disposal_never_replays() {
    let f = Fixture::new(json!({}));
    f.add(json!("1"));
    f.add(json!(null));
    let a = f.launch(ID).await;
    f.add(json!("2"));
    f.add(json!(null));
    let b = f.launch(ID).await;
    for (session, action) in [
        (&b["sessionId"], a["actions"][0]["id"].clone()),
        (&a["sessionId"], json!("forged")),
        (&a["sessionId"], Value::Null),
    ] {
        assert_eq!(
            f.ipc(
                "extension:invoke-action",
                json!({"sessionId":session,"actionId":action})
            )
            .await["ok"],
            false
        );
    }
    f.add(json!(null));
    let rotated = f.invoke(&a).await;
    uncertain(&rotated);
    assert_ne!(rotated["actions"], a["actions"]);
    assert_eq!(f.invoke(&a).await["ok"], false);
    assert_eq!(
        f.ipc(
            "extension:dispose-session",
            json!({"sessionId":a["sessionId"]})
        )
        .await,
        true
    );
    assert_eq!(f.invoke(&rotated).await["ok"], false);
    f.sessions.clear().await;
    assert_eq!(f.invoke(&b).await["ok"], false);
    assert_eq!(f.channels().len(), 5);
}
fn spawn_launch(f: &Fixture) -> tokio::task::JoinHandle<crate::native_core::common::HandlerResult> {
    let (sessions, ctx, files) = (f.sessions.clone(), f.ctx.clone(), f.files.clone());
    tokio::spawn(async move {
        sessions
            .handle(
                &ctx,
                &files,
                "extension:run-command",
                &[json!({"extensionId":ID,"commandName":COMMAND})],
            )
            .await
    })
}
#[tokio::test]
async fn cancelled_launch_before_acquisition_completes_has_no_delivery_or_replay() {
    let f = Fixture::new(json!({}));
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    f.replies
        .lock()
        .unwrap()
        .push_back(Reply::Wait(entered.clone(), release, json!("2")));
    let task = spawn_launch(&f);
    entered.notified().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
    assert_eq!(f.channels(), ["core:platform:selected-text"]);
    assert!(f.replies.lock().unwrap().is_empty());
    f.add(json!("3"));
    f.add(json!({"ok":true}));
    no_view(
        &f.launch(ID).await,
        "Converted! TOON format copied to clipboard",
    );
    assert_eq!(
        f.channels(),
        [
            "core:platform:selected-text",
            "core:platform:selected-text",
            "clipboard:write"
        ]
    );
    assert_eq!(f.calls.lock().unwrap()[2].1, vec![json!("3")]);
}
#[tokio::test]
async fn cancelled_launch_after_effect_dispatch_may_deliver_and_has_no_recoverable_session() {
    for action in ["copy", "paste"] {
        let f = Fixture::new(json!({"defaultAction":action}));
        f.add(json!("2"));
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        f.replies.lock().unwrap().push_back(Reply::Wait(
            entered.clone(),
            release,
            json!({"ok":true}),
        ));
        let task = spawn_launch(&f);
        entered.notified().await;
        // Router dispatch has crossed the effect boundary even without an ack.
        assert_eq!(f.calls.lock().unwrap()[1].1, vec![json!("2")]);
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        assert_eq!(f.channels().len(), 2);
        // No view/ID was committed. A fabricated session cannot recover launch state.
        assert_eq!(
            f.ipc(
                "extension:refresh-session",
                json!({"sessionId":"native-extension:uncommitted"})
            )
            .await["ok"],
            false
        );
        f.sessions.clear().await;
        assert_eq!(f.channels().len(), 2);
        f.add(json!("3"));
        f.add(json!({"ok":true}));
        let done = f.launch(ID).await;
        assert_eq!(done["mode"], "no-view");
        let calls = f.calls.lock().unwrap();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[3].1, vec![json!("3")]);
    }
}
#[tokio::test]
async fn direct_launch_checkpoints_intent_before_effect_await() {
    let f = Fixture::new(json!({}));
    f.add(json!("2"));
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    f.replies
        .lock()
        .unwrap()
        .push_back(Reply::Wait(entered.clone(), release, json!({"ok":true})));
    let mut state = json!({});
    {
        let payload = json!({});
        let future = adapter::handle(
            &f.ctx,
            COMMAND,
            "extension:run-command",
            &payload,
            &mut state,
        );
        tokio::pin!(future);
        tokio::select! { _=entered.notified()=>{}, result=&mut future=>panic!("effect should wait: {result:?}") }
    }
    assert_eq!(state["phase"], "intent");
    assert_eq!(state["output"], "2");
    assert!(state["token"].as_str().is_some());
    assert_eq!(
        f.channels(),
        ["core:platform:selected-text", "clipboard:write"]
    );
}
#[tokio::test]
async fn cancelled_committed_retry_retains_checkpoint_and_never_rereads() {
    let f = Fixture::new(json!({}));
    f.add(json!("3"));
    f.add(json!(null));
    let view = f.launch(ID).await;
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    f.replies
        .lock()
        .unwrap()
        .push_back(Reply::Wait(entered.clone(), release, json!({"ok":true})));
    let (sessions, ctx, files, target) = (
        f.sessions.clone(),
        f.ctx.clone(),
        f.files.clone(),
        view.clone(),
    );
    let task = tokio::spawn(async move {
        sessions
            .handle(
                &ctx,
                &files,
                "extension:invoke-action",
                &[json!({"sessionId":target["sessionId"],"actionId":target["actions"][0]["id"]})],
            )
            .await
    });
    entered.notified().await;
    assert_eq!(f.refresh(&view).await["mode"], "unchanged");
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let retry = f.refresh(&view).await;
    uncertain(&retry);
    assert_ne!(retry["actions"], view["actions"]);
    assert_eq!(f.invoke(&view).await["ok"], false);
    f.add(json!({"ok":true}));
    no_view(
        &f.invoke(&retry).await,
        "Converted! TOON format copied to clipboard",
    );
    assert_eq!(
        f.channels(),
        [
            "core:platform:selected-text",
            "clipboard:write",
            "clipboard:write",
            "clipboard:write"
        ]
    );
    let calls = f.calls.lock().unwrap();
    assert_eq!(calls[1].1, calls[2].1);
    assert_eq!(calls[2].1, calls[3].1);
}
#[tokio::test]
async fn clear_inflight_launch_does_not_replay_or_offer_recovery() {
    let f = Fixture::new(json!({}));
    f.add(json!("1"));
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    f.replies
        .lock()
        .unwrap()
        .push_back(Reply::Wait(entered.clone(), release, json!({"ok":true})));
    let task = spawn_launch(&f);
    entered.notified().await;
    f.sessions.clear().await;
    assert_eq!(task.await.unwrap().unwrap().unwrap()["ok"], false);
    assert_eq!(f.channels().len(), 2);
}
#[tokio::test]
async fn clearing_inflight_committed_retry_discards_checkpoint_without_replaying() {
    let f = Fixture::new(json!({}));
    f.add(json!("1"));
    f.add(json!(null));
    let view = f.launch(ID).await;
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    f.replies
        .lock()
        .unwrap()
        .push_back(Reply::Wait(entered.clone(), release, json!({"ok":true})));
    let (sessions, ctx, files, target) = (
        f.sessions.clone(),
        f.ctx.clone(),
        f.files.clone(),
        view.clone(),
    );
    let task = tokio::spawn(async move {
        sessions
            .handle(
                &ctx,
                &files,
                "extension:invoke-action",
                &[json!({"sessionId":target["sessionId"],"actionId":target["actions"][0]["id"]})],
            )
            .await
    });
    entered.notified().await;
    f.sessions.clear().await;
    assert_eq!(task.await.unwrap().unwrap().unwrap()["ok"], false);
    assert_eq!(f.invoke(&view).await["ok"], false);
    assert_eq!(f.channels().len(), 3);
}
#[tokio::test]
async fn discarded_state_tokens_cannot_revive_and_refresh_is_readonly() {
    let f = Fixture::new(json!({}));
    let mut state = json!({});
    f.add(json!("1"));
    f.add(json!(null));
    let view = adapter::handle(
        &f.ctx,
        COMMAND,
        "extension:run-command",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    let saved = state.clone();
    adapter::handle(
        &f.ctx,
        COMMAND,
        "extension:refresh-session",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(state, saved);
    let mut staged = state.clone();
    f.add(json!(null));
    let discarded = adapter::handle(
        &f.ctx,
        COMMAND,
        "extension:invoke-action",
        &json!({"actionId":view["actions"][0]["id"]}),
        &mut staged,
    )
    .await
    .unwrap();
    f.add(json!(null));
    let next = adapter::handle(
        &f.ctx,
        COMMAND,
        "extension:invoke-action",
        &json!({"actionId":view["actions"][0]["id"]}),
        &mut state,
    )
    .await
    .unwrap();
    assert_ne!(discarded["actions"], next["actions"]);
    assert!(adapter::handle(
        &f.ctx,
        COMMAND,
        "extension:invoke-action",
        &json!({"actionId":discarded["actions"][0]["id"]}),
        &mut state
    )
    .await
    .is_err());
}
#[tokio::test]
async fn service_protocol_read_errors_and_resource_limits_never_deliver() {
    for response in [
        None,
        Some(json!(null)),
        Some(json!({"text":"private secret"})),
        Some(json!(false)),
    ] {
        let f = Fixture::new(json!({"inputSource":"clipboard"}));
        if let Some(v) = response {
            f.add(v)
        } else {
            f.error()
        }
        let result = f.launch(ID).await;
        assert_eq!(result["mode"], "no-view");
        assert!(!result.to_string().contains("private"));
        assert_eq!(f.channels(), ["clipboard:read"]);
    }
    for selected in [
        json!(null),
        json!(" ".repeat(1_048_577)),
        json!("[".repeat(65) + "0" + &"]".repeat(65)),
    ] {
        let f = Fixture::new(json!({}));
        f.add(selected);
        assert_eq!(f.launch(ID).await["mode"], "no-view");
        assert_eq!(f.channels(), ["core:platform:selected-text"]);
    }
}
#[tokio::test]
async fn native_launch_matches_all_executable_source_flow_route_order_and_effect_bytes() {
    let flows: Value = serde_json::from_str(include_str!("../../tests/source-flows.json")).unwrap();
    assert_eq!(flows.as_array().unwrap().len(), 16);
    for flow in flows.as_array().unwrap() {
        let input = &flow["input"];
        // Source fixture's API stub returns raw prefs, whereas real registry supplies
        // defaults. Materialize absent raw keys as null for strict source comparison.
        let prefs = if input["preferences"].is_object() {
            json!({"inputSource":input["preferences"]["inputSource"],"defaultAction":input["preferences"]["defaultAction"]})
        } else {
            json!({"inputSource":"auto","defaultAction":"copy"})
        };
        let f = Fixture::new(prefs);
        let mut expected = vec![];
        for event in flow["events"].as_array().unwrap() {
            let name = event[0].as_str().unwrap();
            let route = match name {
                "selection" => "core:platform:selected-text",
                "read" => "clipboard:read",
                "copy" => "clipboard:write",
                "paste" => "core:platform:paste-text",
                _ => continue,
            };
            let args = if matches!(name, "copy" | "paste") {
                vec![event[1].clone()]
            } else {
                vec![]
            };
            expected.push((route.to_owned(), args));
            if input["fail"] == name {
                f.error()
            } else {
                f.add(match name {
                    "selection" => input.get("selected").cloned().unwrap_or(json!("")),
                    "read" => input.get("clipboard").cloned().unwrap_or(json!("")),
                    _ => json!({"ok":true}),
                })
            }
        }
        let result = f.launch(ID).await;
        assert_eq!(*f.calls.lock().unwrap(), expected, "{input}");
        if matches!(input["fail"].as_str(), Some("copy" | "paste")) {
            uncertain(&result)
        } else {
            assert_eq!(result["mode"], "no-view");
            if let Some(hud) = flow["events"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e[0] == "hud")
            {
                assert_eq!(result["message"], hud[1]);
            }
        }
        assert!(f.replies.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn exported_actual_uncertain_recovery_and_no_view_success_for_renderer() {
    let f = Fixture::new(json!({}));
    let mut state = json!({});
    f.add(json!("\"<script>private</script>\""));
    f.add(json!(null));
    let mut retry = adapter::handle(
        &f.ctx,
        COMMAND,
        "extension:run-command",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    uncertain(&retry);
    f.add(json!({"ok":true}));
    let done = adapter::handle(
        &f.ctx,
        COMMAND,
        "extension:invoke-action",
        &json!({"actionId":retry["actions"][0]["id"]}),
        &mut state,
    )
    .await
    .unwrap();
    retry["actions"][0]["id"] = json!("fixture-retry-action");
    let bytes = serde_json::to_string_pretty(&vec![retry, done]).unwrap() + "\n";
    let expected =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/surfaces.json");
    if std::env::var_os("TOON_RECORD_SURFACES").is_some() && !expected.exists() {
        std::fs::write(&expected, &bytes).unwrap();
    }
    assert_eq!(std::fs::read_to_string(expected).unwrap(), bytes);
}
