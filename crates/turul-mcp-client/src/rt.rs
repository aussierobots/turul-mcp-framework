//! Runtime primitives: the one place the client spawns, sleeps, times out and
//! reads the clock. Native re-exports tokio and `std::time`;
//! wasm32 uses wasm-bindgen's executor, JavaScript timers and `web_time`.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::*;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub(crate) use std::time::Instant;
    pub(crate) use tokio::spawn;
    pub(crate) use tokio::task::JoinHandle;
    /// Deadlines on the 2026-07-28 streaming paths, the only users.
    #[cfg(any(feature = "client-bilingual", feature = "client-2026-07-28-only"))]
    pub(crate) use tokio::time::Instant as TimerInstant;
    pub(crate) use tokio::time::{sleep, timeout};
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::future::Future;
    use std::time::Duration;

    use futures::future::{AbortHandle, Abortable, Either, select};

    pub(crate) use web_time::Instant;
    /// Deadlines use the same clock as everything else on wasm32.
    pub(crate) use web_time::Instant as TimerInstant;

    /// A task spawned on the wasm-bindgen executor; `abort` stops it at its next poll.
    pub(crate) struct JoinHandle<T> {
        abort: AbortHandle,
        _output: std::marker::PhantomData<T>,
    }

    impl<T> JoinHandle<T> {
        pub(crate) fn abort(&self) {
            self.abort.abort();
        }
    }

    /// Runs `future` on the current thread's executor. The output is discarded:
    /// no caller awaits a handle.
    pub(crate) fn spawn<F>(future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        let (abort, registration) = AbortHandle::new_pair();
        wasm_bindgen_futures::spawn_local(async move {
            let _ = Abortable::new(future, registration).await;
        });
        JoinHandle {
            abort,
            _output: std::marker::PhantomData,
        }
    }

    /// Waits for `duration` on a JavaScript timer.
    pub(crate) async fn sleep(duration: Duration) {
        gloo_timers::future::sleep(duration).await;
    }

    /// `future` did not finish within the timeout.
    #[derive(Debug)]
    pub(crate) struct Elapsed;

    /// Resolves to `future`'s output, or to `Elapsed` once `duration` passes first.
    pub(crate) async fn timeout<F: Future>(
        duration: Duration,
        future: F,
    ) -> Result<F::Output, Elapsed> {
        let future = std::pin::pin!(future);
        let timer = std::pin::pin!(sleep(duration));
        match select(future, timer).await {
            Either::Left((output, _)) => Ok(output),
            Either::Right(((), _)) => Err(Elapsed),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use regex::Regex;

    /// Files compiled only on native, exempt from the clock scan.
    const NATIVE_ONLY: [&str; 2] = ["transport/sse.rs", "oauth_client_credentials.rs"];

    fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// Code before the file's inline `#[cfg(test)] mod name {`, with `//`
    /// comment lines blanked so line numbers survive. Tests run on native only.
    fn non_test_code(source: &str) -> String {
        let test_module = Regex::new(r"#\[cfg\(test\)\]\s*mod \w+\s*\{").unwrap();
        let code = test_module
            .find(source)
            .map_or(source, |m| &source[..m.start()]);
        code.lines()
            .map(|line| {
                if line.trim_start().starts_with("//") {
                    ""
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Every use of `std::time` other than `Duration`, as `(byte offset, text)`.
    ///
    /// `std::time::Instant` and `SystemTime` compile on wasm32 and panic when
    /// called, so the compiler cannot catch them. The rule is inverted: only
    /// `Duration` is allowed, so a new import form cannot slip through.
    fn clock_offences(code: &str) -> Vec<(usize, String)> {
        let path_item = Regex::new(r"std::time::(\w+|\*)").unwrap();
        let brace_list = Regex::new(r"std::time::\{([^}]*)\}").unwrap();
        let whole_module = Regex::new(r"std::time(\s*[;,}]|\s+as\b)").unwrap();
        let nested_in_std = Regex::new(r"use std::\{[^;]*\btime\b").unwrap();

        let mut found = Vec::new();
        for m in path_item.captures_iter(code) {
            if &m[1] != "Duration" {
                let whole = m.get(0).unwrap();
                found.push((whole.start(), whole.as_str().to_string()));
            }
        }
        for m in brace_list.captures_iter(code) {
            if m[1]
                .split(',')
                .map(str::trim)
                .any(|item| !item.is_empty() && item != "Duration")
            {
                let whole = m.get(0).unwrap();
                found.push((whole.start(), whole.as_str().to_string()));
            }
        }
        for pattern in [&whole_module, &nested_in_std] {
            for m in pattern.find_iter(code) {
                found.push((m.start(), m.as_str().to_string()));
            }
        }
        found
    }

    /// Every clock read compiled on wasm32 goes through `rt::Instant`.
    #[test]
    fn wasm_compiled_code_reads_the_clock_only_through_rt() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&src, &mut files);

        let mut scanned = Vec::new();
        let mut offenders = Vec::new();
        for path in files {
            let relative = path
                .strip_prefix(&src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if relative == "rt.rs" || NATIVE_ONLY.contains(&relative.as_str()) {
                continue;
            }
            let code = non_test_code(&std::fs::read_to_string(&path).unwrap());
            for (at, text) in clock_offences(&code) {
                let line = code[..at].matches('\n').count() + 1;
                offenders.push(format!("src/{relative}:{line}: {text}"));
            }
            scanned.push(relative);
        }
        for required in ["client.rs", "session.rs", "transport/http.rs"] {
            assert!(
                scanned.iter().any(|f| f == required),
                "{required} was not scanned: {scanned:?}"
            );
        }
        assert!(
            offenders.is_empty(),
            "std clock in wasm-compiled code; use crate::rt::Instant:\n{}",
            offenders.join("\n")
        );
    }

    /// The scan flags every import form except `Duration`.
    #[test]
    fn clock_scan_flags_every_form_but_duration() {
        for hit in [
            "use std::time::Instant;",
            "use std::time::{Duration, Instant};",
            "use std::time::{\n    Duration,\n    Instant,\n};",
            "let t = std::time::Instant::now();",
            "use std::time::{SystemTime, UNIX_EPOCH};",
            "let e = std::time::UNIX_EPOCH;",
            "use std::time::*;",
            "use std::time::{self, Duration};",
            "use std::time;",
            "use std::time as t;",
            "use std::{sync::Arc, time::Instant};",
            "use std::{time::{Duration, Instant}};",
        ] {
            assert!(!clock_offences(hit).is_empty(), "should flag: {hit}");
        }
        for miss in [
            "use std::time::Duration;",
            "pub use std::time::Duration;",
            "use std::time::{Duration};",
            "let d = std::time::Duration::from_secs(1);",
            "use crate::rt::Instant;",
        ] {
            assert!(clock_offences(miss).is_empty(), "should not flag: {miss}");
        }
    }

    /// Only an inline test module ends the scanned code, and comments are skipped.
    #[test]
    fn non_test_code_cuts_at_the_inline_test_module_only() {
        let source = "#[cfg(test)]\nmod helpers;\nuse std::time::Instant;\n// std::time::Instant in a comment\n#[cfg(test)]\nmod tests {\n    use std::time::SystemTime;\n}\n";
        let code = non_test_code(source);
        assert!(code.contains("use std::time::Instant;"));
        assert!(!code.contains("in a comment"));
        assert!(!code.contains("SystemTime"));
        assert_eq!(clock_offences(&code).len(), 1);
    }
}
