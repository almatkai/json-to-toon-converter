# JSON to TOON Converter — compiled Rust Native V2

Offline Rust parser/encoder plus a concrete production adapter for Tezbar's compiled-in Native V2 system. The manifest alone cannot execute this code. There is no external Rust plugin loader, JavaScript runtime, formatter subprocess, network conversion, persistent source storage, history or migration.

Original: meshal, Raycast `6734d85992290066fd31a7f5bee3cca3d3545a1a`. Own source: `f72d105582202089268d31ab0d0314e5f1eef057`. Own `src/convert-json.ts` SHA256: `47770b2fde11cf2508fefd8267f404e421b865c585ae42891975386505e2e9dd`. Own lock SHA256: `dc218c9b390c2203ecd5e00391b0bdc8825edd2faaf6e27f0682c2ff7d797d15`. Actual locked library: **@toon-format/toon 1.3.0**, independently fetched tarball SHA256 `87f3755ab4cda51ca220a37f3ed2d37ce9f65bc78096f61e765955aabd5ed760`.

The exact original icon is 159,623 bytes, SHA256 `a0d60a4ee7f9074dc52ad478a89e11a4b13e244eff9863d45cf2c69081a1a384`. It is `assets/extension-icon.png`; copy it unchanged to **`src/renderer/public/extensions/icons/json-to-toon-converter.png`**. Its manifest URL is `/extensions/icons/json-to-toon-converter.png`. Source and license evidence, including the missing extension-local license, are in [NOTICES.md](NOTICES.md).

## Behavior and bounded differences

The command has no input arguments. Preferences come from `payload.preferences`, with metadata defaults `defaultAction=copy`, `inputSource=auto`. Present unexpected/invalid typed values retain the original strict comparison branches: non-`auto` reads clipboard only; non-`copy` delivers through paste. Registry tests exercise the actual beta default and legacy preference routing, not an invented preference store.

With `auto`, selected text is read once. Selected-read errors fall back to clipboard; empty/ECMAScript-trim-whitespace selection falls back once. Nonempty invalid selected JSON fails without reading clipboard. Clipboard failures fail safely. Malformed non-string service replies are protocol errors, not strings to coerce. Empty sources report the original no-JSON message. Conversion errors are useful sanitized classifications; unlike the V1 toast/log path, they never embed source JSON or raw service error text.

Launch reads/converts and immediately performs exactly one copy or paste according to preferences, without another Enter/click. The manifest declares `no-view`. Strictly acknowledged success returns the SDK's actual `no-view` message with the original copied/pasted wording; empty, invalid and read-error outcomes also return sanitized `no-view` messages. A successful launch registers no interactive session. The beta host has no SDK HUD/toast/popToRoot helper and `window:hide` returns null; no HUD is claimed or emulated. Only an unconfirmed delivery returns a static Detail recovery view with an explicit duplicate-effect warning and retry action. No source or output is rendered as Markdown: only fixed status/error strings are shown. Consequently malicious TOON content cannot create trusted markup/URLs or expose the host Markdown code-copy control.

Copy calls SDK `copy` → `clipboard:write[text]` once. Paste calls `core:platform:paste-text[text]` once; that route already writes clipboard, hides/restores app context, waits and pastes. The adapter makes no second write/hide/paste call and no additional dismissal request. Success requires an object with `ok:true`, `error` absent/null, and `success` absent/true. The native message uses the source copied/pasted wording only after this acknowledgement; it cannot prove actual target-app focus or receipt.

Before every effect await, including the automatic launch, the adapter checkpoints intent and allocates a fresh token. Transport errors and null/boolean/malformed/negative/contradictory replies leave a visible unconfirmed state. Only its explicit retry action may repeat the effect, using the **same converted bytes**, with a duplicate-effect warning. Refresh/search/load-more do not read, convert, allocate IDs or deliver. IDs combine a process nonce and a monotonically increasing allocation, so discarded clones cannot revive a token. NativeSessions gates session IDs; the adapter rejects missing/stale/forged/cross-session action IDs. Caller content/formValues/argumentValues are never delivery data.

Cancellation while acquisition is still pending has not dispatched delivery. Cancellation after effect intent/router dispatch may already have copied or pasted, even without an acknowledgement. NativeSessions cleans up cancelled launch state; no retry view/session was committed and there may be no recoverable checkpoint. Reopening is a deliberate new invocation that reads and converts again, not recovery of the cancelled delivery. A cancelled retry action on an already committed recovery session retains intent; refresh exposes the rotated retry token. Disposing/clearing the session clears its private output and does not undo a possible external effect. After disposal, cancellation by uninstall/shutdown, renderer loss or process exit, that in-memory checkpoint is intentionally unavailable: an external delivery may have happened, and reopening is a new conversion, never automatic recovery. There is no global session map or source persistence.

The pure core deliberately implements JSON.parse's f64 Number rounding, negative zero normalization, exponent/overflow behavior (nonfinite numbers become null in the locked encoder), duplicate-key replacement without moving insertion order, canonical integer-key ordering, Unicode/escapes, and the locked encoder's `__proto__` normalization/prototype effects. It preserves the library's default comma delimiters, two-space indentation, key/field order, tables, mixed/nested arrays, empty objects and the empty-key header quirks. Sparse arrays are invalid JSON input; JSON `null` array entries are supported. Optional encoder settings, non-JSON JS values and newer TOON dialect behavior are outside this command's contract.

**Limits:** input 1 MiB UTF-8, output 4 MiB UTF-8, root depth 0 through 64, 50,000 parsed value nodes, 8,000,000 deterministic encoder work units (input bytes + charged output/header/key/table traversal). Whole input is rejected on any exceeded budget; no truncated result is delivered. Parsing is separately bounded by input/depth/node caps; temporary encoder allocations are bounded by those caps. Unpaired UTF-16 surrogates are accepted by JS JSON.parse but cannot cross Rust/host UTF-8 string boundaries faithfully: reject them explicitly, with no replacement character or lossy normalization. Valid surrogate pairs pass. This is finite tested parity, not a universal proof.

## Integration artifacts — host remains unchanged

Production source layout:

- Vendor `adapter.rs`, `src/lib.rs`, notices/licenses and manifest under **`src-tauri/native-extensions/json-to-toon-converter/`**.
- Register `#[path="../../../native-extensions/json-to-toon-converter/adapter.rs"] mod v2_json_to_toon;` in `src-tauri/src/native_core/extensions/mod.rs`. The adapter imports sibling `super::v2_sdk`.
- Add exact host dependency `ryu-js = "=1.0.2"`, then update the host Cargo.lock in the integration checkout; the core/harness locks here are not replacements for the host lock.
- Apply the matching registration and number-dependency patches in `integration/`. Separate patches target beta `fd300e2` and committed current `fe71ca8`. They append one catalog entry, add slug/command/handle dispatch and adjust the fixture catalog count 11→12. Every pre-existing entry is retained in order; scripts check this. If earlier sequential ports have already been integrated, rebase these small additions and increment the **actual** count, preserving all earlier ports. Never replace the whole catalog with the baseline fixture.
- Install canonical `com.tezbar.native.json-to-toon-converter` after rebuilding. Both canonical and `raycast.json-to-toon-converter` dispatch through the native installed command; without compiled/installed coverage the canonical ID fails. The V1 package remains for reversible coexistence. Actual registry logic reads legacy preferences only when native preferences are absent and does not write/migrate them. The own V1 manifest's `com.tezbar.json-to-toon-converter` ID is not an added runtime alias.

Two renderer prerequisites are supplied **separately**, not applied to a real host:

1. `current-icon-prerequisite.patch`: fe71ca8's `extensionImageSource` converts bundled `/extensions/...` paths into file URLs. Preserve exactly this extension's URL before that conversion. Other filesystem/image policy remains unchanged, including other extensions' own prerequisites.
2. `{beta,current}-detail-button-prerequisite.patch`: the actual Detail Enter button forwards a MouseEvent into Surface's optional action-ID parameter. Wrap only the Detail callback so it invokes the committed primary action without the event. This prerequisite is used only by the uncertain-delivery retry surface. Actual pinned renderer tests reproduce this defect and verify the patch. Keyboard Enter and the action overlay also dispatch the same opaque ID. No form/parser redesign is proposed.

These patches are checked against exported Git objects. Host registration/rebuild/install and actual renderer rendering remain **NOT-RUN**. Manifest verification is **not-run on macOS and Windows**. Windows GNU check/clippy is compilation only. Real clipboard/selection/paste/focus, permissions, UI and keyboard QA remain NOT-RUN. No actual OS effect was executed during validation.

## Reproduction

From `v2/`, with cached Cargo crates and both Rust stable and 1.85.0 installed:

```sh
python3 tools/prepare_harness.py
bash tools/check-rust.sh
python3 tools/index.py --check
```

The nested harness manifests are templates; preparation materializes a self-contained copy under `.work/harness`. It compiles the real beta common/SDK/NativeSessions/registry/catalog and production adapter. Tauri and unrelated adapters are explicit accurate boundary stubs; all profiles are disposable headless directories. No desktop profile or real router is used. Core tests compare 4,558 deterministic JSON.parse + actual TOON 1.3.0 oracle cases, with exactly three explicit surrogate representation gaps; 16 executable own-command flow fixtures cover API ordering and failures. The beta harness runs 24 tests including pinned SDK/common/catalog/core tests and 17 adapter/session tests. It compares all 16 source flows against native launch route ordering and exact delivery bytes; checks 130 successful launches without registering sessions, automatic copy/paste, strict acknowledgements, sanitized failures, cancellation before acquisition completion and after effect dispatch, committed-retry cancellation, no replay, identical-byte retries without rereads, stale/forged/cross-session IDs and installed alias/preferences. Exported renderer fixtures contain only actual uncertain recovery and acknowledged no-view success.

Review round 2 also corrects a source-flow recorder defect: VM `JSON.parse` objects previously crossed into an encoder imported from another realm. TOON 1.3.0 checks object prototypes, so that artificial boundary recorded ordinary object output as `null`. The recorder now supplies matching JSON/Error intrinsics to the unmodified source command and imported encoder. Derived source-flow evidence was regenerated (ordinary object flows now encode correctly); the 4,558-case oracle, source/library/host exports and core remain unchanged.

To rerun the executable TypeScript oracle/renderer contracts without touching V1:

```sh
mkdir -p .work/v1
cp -R fixtures/own/. .work/v1/
npm ci --prefix .work/v1 --ignore-scripts --legacy-peer-deps --no-audit --no-fund
cargo +stable build --locked --offline
node tools/oracle.mjs
node tools/renderer-contract.mjs
npm --prefix .work/v1 test
npm --prefix .work/v1 run typecheck
```

The original install has a React types peer conflict; `--legacy-peer-deps` is needed only in this disposable copy. Smoke and typecheck pass with the unchanged original lock. `oracle.mjs` intercepts import resolution and records API/OS boundaries while transpiling/executing the exact own command. No actual Raycast or desktop clipboard call runs.

Full schema validation uses the saved Draft 2020-12 schema and 21 negative cases:

```sh
python3 -m venv .work/schema
.work/schema/bin/pip install -r tools/schema-requirements.txt
.work/schema/bin/python tools/validate.py
```

`tools/check-package.py` packages, audits every indexed artifact and extracts the crate into `.work/`, then runs the core and concrete beta harness offline on both Rust toolchains using cached registry crates. No absolute host/source path is needed. `INDEX.json` covers exact immutable evidence and integration fixtures; the parent must rehash every record after staging. All V1 tracked bytes remain untouched except the short root README discovery link. The clone and all work are intentionally left unstaged and uncommitted for parent review.

XERJ was attempted with canonical root-hash prefix `ax-4e14c96e30c3`. Indexing hit HTTP 429 / RSS circuit-breaker limits; only this task's indexing process was interrupted. The index is incomplete. A review-round-2 bounded refresh completed with junk (10 files/49 records) and still did not find `prepare`; no complete V2 coverage is claimed. Exact review-round-2 checks used Python searches/full current files and pinned Git object exports because `rg` is unavailable in this environment. No global service/schema/prefix change was made.
