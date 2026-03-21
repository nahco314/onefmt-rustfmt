# foro-rustfmt

This is the rustfmt foro plugin.

**This plugin cannot be used as a WASM plugin.**

rustfmt depends on the rustc library, and can only be built as a target that can build rustc itself. WASM does not fall into this category, so **it can only be used as a native DLL.**

This plugin provides native plug-ins for most platforms, so it usually works fine. for now, x86_64-unknown-linux-gnu, x86_64-apple-darwin, aarch64-apple-darwin and x86_64-pc-windows-msvc are supported.

## Known Issues

### Linux: `cannot allocate memory in static TLS block` (versions ≥ 1.9.0)

**Affected:** Linux (glibc), `foro-rustfmt` ≥ 1.9.0

**Symptom:**

```
Error: cannot allocate memory in static TLS block
```

**Cause:** `librustc_driver.so` uses the `initial-exec` thread-local storage (TLS) model. When loaded dynamically via `dlopen` at runtime, glibc must fit the library's TLS into a fixed-size static block reserved at process startup. Starting from rustc 1.95.0-nightly (bundled in foro-rustfmt 1.9.0), the TLS requirements of `librustc_driver` exceed this block size on typical Linux systems.

**Workaround:** Use `foro-rustfmt` 0.4.7 or earlier (based on rustc 1.83.0-nightly), which does not hit this limit. The default foro configuration pins to 0.4.7 for this reason.

**Long-term fix:** Under investigation. The root fix requires either the rustc project to change its TLS model, or foro to load plugins in an isolated worker process.
