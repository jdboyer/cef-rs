# cef-rs Message Router Bugs

Two bugs in `cef/src/wrapper/message_router.rs` that break basic `cefQuery` usage.

---

## Bug 1: `value_bykey` returns `Some(undefined)` for missing object keys, breaking optional field handling

### Location

`RendererSideV8Handler::execute()`, around the `persistent` field validation.

### Symptom

Calling `window.cefQuery({ request: "...", onSuccess: fn, onFailure: fn })` without the optional `persistent` field causes the handler to silently return 0 (undefined). No exception is thrown, no error is reported. The query is never sent.

### Root Cause

The code assumes `value_bykey` returns `None` for keys that don't exist on the JS object:

```rust
let key = CefString::from(ObjectMember::PERSISTENT);
let persistent = if let Some(persistent) = arg.value_bykey(Some(&key)) {
    if persistent.is_bool() == 0 {
        return_exception!("...");  // <-- hits this path
    }
    Some(persistent)
} else {
    None  // <-- never reached for missing keys
};
```

But `value_bykey` wraps CEF's `GetValue` C API, which returns a non-null pointer to a V8 `undefined` value when a key doesn't exist on an object. The Rust binding only checks for null pointers:

```rust
// In bindings/x86_64_unknown_linux_gnu.rs, line 31718
fn value_bykey(&self, key: Option<&CefString>) -> Option<V8Value> {
    // ...
    let result = f(arg_self_, arg_key);
    if result.is_null() {
        None
    } else {
        Some(result.wrap_result())  // V8 undefined is non-null, so this returns Some
    }
}
```

So for a missing key: `value_bykey` returns `Some(v8_undefined)`, `is_bool()` returns 0 on the undefined value, and `return_exception!` fires, silently aborting the handler.

### Fix

Filter out V8 undefined values when checking optional fields:

```rust
let persistent = if let Some(persistent) = arg.value_bykey(Some(&key)).filter(|v| v.is_undefined() == 0) {
```

### Why this is correct

- CEF's `persistent` parameter is explicitly optional in the cefQuery API. The [CEF documentation](https://bitbucket.org/chromiumembedded/cef/src/master/include/wrapper/cef_message_router.h) defines the JS function signature as `cefQuery({request: '...', onSuccess: function(response) {}, onFailure: function(error_code, error_message) {}, persistent: false})` where `persistent` defaults to `false`.
- The CEF C++ reference implementation (`cef_message_router.cc`) checks `HasValue` before `GetValue` for optional fields, or checks `IsUndefined()` after retrieval.
- The `else { None }` branch in the Rust code clearly intends for missing keys to be treated as absent, but that branch is unreachable due to the binding behavior.
- This bug affects `onSuccess` and `onFailure` too if they were ever omitted (they're also optional per the CEF API), but in practice most callers always provide them.

---

## Bug 2: `execute_success_callback` always delivers responses as ArrayBuffer, even for string responses

### Location

`RendererSideRouter::execute_success_callback()`, lines 1329-1344.

### Symptom

The `onSuccess` JavaScript callback receives an `ArrayBuffer` containing the UTF-8 bytes of the response string, instead of receiving a string. This means:
- `typeof response` is `"object"`, not `"string"`
- `JSON.stringify(response)` produces `{}` (ArrayBuffer serializes to empty object)
- String operations on the response silently fail or produce garbage

### Root Cause

The function unconditionally converts all response payloads (including strings) into an ArrayBuffer:

```rust
fn execute_success_callback(&self, ..., response: mru::MessagePayload) {
    let data = match &response {
        mru::MessagePayload::Empty => &[],
        mru::MessagePayload::String(s) => s.as_slice().unwrap_or(&[]),  // gets raw bytes
        mru::MessagePayload::Binary(b) => b.data(),
    };

    // Always creates ArrayBuffer, even for String payloads:
    let value = v8_value_create_array_buffer(data.as_ptr() as *mut u8, data.len(), ...);

    success_callback.execute_function_with_context(..., Some(&[value]));
}
```

There is no branch for string payloads to call `v8_value_create_string` instead.

### Fix

Dispatch on the payload type:

```rust
let value = match &response {
    mru::MessagePayload::String(s) => {
        let cef_str = CefString::from(std::str::from_utf8(s.as_slice().unwrap_or(&[])).unwrap_or(""));
        v8_value_create_string(Some(&cef_str))
    }
    _ => {
        // Binary and Empty payloads remain as ArrayBuffer
        let data = match &response {
            mru::MessagePayload::Empty => &[],
            mru::MessagePayload::Binary(b) => b.data(),
            _ => unreachable!(),
        };
        v8_value_create_array_buffer(data.as_ptr() as *mut u8, data.len(), ...)
    }
};
```

### Why this is correct

- The `BrowserSideCallback` trait explicitly distinguishes `success_str(&self, response: &str)` from `success_binary(&self, data: &[u8])`. These two methods create `MessagePayload::String` and `MessagePayload::Binary` respectively. This distinction exists precisely so that the renderer side can deliver the response in the appropriate JS type.
- The `execute_failure_callback` in the same file correctly uses `v8_value_create_string` for its string parameter (the error message). The success callback should follow the same pattern for string payloads.
- The CEF C++ reference implementation delivers string responses as JS strings and binary responses as ArrayBuffers. The Rust port collapses this distinction.
- The `onSuccess` callback signature in the CEF API is `function(response)` where `response` is a string for string queries. Every existing example and consumer of the cefQuery API expects a string.
