// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The app's own screens on a phone, stepped as a person with a keyboard
//! steps them: plan section 29 row 2 through Show, Add (by the camera's
//! Scan or by the field) and the trust step, in the app's normal process
//! rather than a case's (`app_lifecycle.rs`'s shape).
//!
//! The host cannot see the screen: Slint draws no accessibility tree on
//! Android, and Show is a secured window, so a screencap of it is black.
//! What it reads instead is the debug build's trace (ui-slint's
//! `src/trace.rs`, the `ui-trace` feature): a logcat line per change of
//! page, focus and the Peers steps' outcomes, in kinds only. A key
//! goes in by `adb shell input keyevent`; [`Screens::tab_to`] presses TAB
//! until the trace names the control, and a step waits for the outcome
//! line it expects. Nothing is pressed on a count of TABs: a TAB that
//! lands elsewhere is seen, and the step fails naming where focus was.
//!
//! Each step answers `{case, result, detail}`, as the cases do, and never
//! panics: the driver decides what a failed step means. A step done is
//! the screen's answer, never the request: a Trust reports done on
//! `block:<h>=trusted`, which the view draws only once the runtime has
//! answered the set, after writing it (`LOCAL-IPC.md`'s trust overlay), so
//! a stop after a done step loses nothing.
//!
//! What this does NOT prove: what the screen looks like or says (the
//! trace names kinds, not text or layout); what a screen reader reads
//! (none reads this tree on Android); that a person can reach a control
//! by touch; and, for the field route, Show on the other phone, whose
//! code text comes from the `own_addresses` case.

use std::process::{Command, Output};
use std::time::{Duration, Instant};

use interweave_android_e2e_cases::keys;
use serde_json::{Map, Value, json};

use crate::adb::PACKAGE;

/// What every trace line starts with (ui-slint's `trace::PREFIX`).
pub const TRACE: &str = "ui-trace ";

/// How many TABs [`Screens::tab_to`] presses before it gives up: more
/// than any page here has controls, so a control never reached is not
/// on the page.
const TABS: usize = 40;

/// How long a TAB's focus line may take to come.
const TAB_PATIENCE: Duration = Duration::from_secs(3);

/// How long the trace must be quiet for focus to count as settled.
const QUIET: Duration = Duration::from_millis(700);

/// How often the log is read while waiting.
const POLL: Duration = Duration::from_millis(250);

/// One step's answer: `{case, result, detail}`.
pub type Step = Map<String, Value>;

/// The app on one phone, by its `adb -s` serial.
pub struct Screens {
    serial: String,
    /// The trace lines read so far since the launch cleared the log.
    seen: Vec<String>,
    /// How many of `seen` a wait has already consumed.
    read: usize,
    /// The focus key the trace last named.
    focus: String,
}

impl Screens {
    /// The app on the phone with `serial`; nothing is sent yet.
    #[must_use]
    pub fn new(serial: &str) -> Self {
        Self {
            serial: serial.to_owned(),
            seen: Vec::new(),
            read: 0,
            focus: String::new(),
        }
    }

    /// Start the app from nothing: force-stop it (focus survives an
    /// `am start` of a running activity, so a step never starts from
    /// where the last left focus), clear the log, launch, and wait for
    /// the conversation list.
    pub fn launch(&mut self, patience: Duration) -> Step {
        let name = "launch";
        let _ = self.adb(&["shell", "am", "force-stop", PACKAGE]);
        if let Err(why) = self.adb_ok(&["logcat", "-c"]) {
            return step(name, false, why);
        }
        (self.seen, self.read, self.focus) = (Vec::new(), 0, String::new());
        let launched = self
            .adb_ok(&[
                "shell",
                "am",
                "start",
                "-W",
                "-n",
                &format!("{PACKAGE}/.MainActivity"),
            ])
            .and_then(|_| self.wait_all(0, &["page=conversations"], patience));
        self.answer(name, launched)
    }

    /// Conversations -> the app's menu -> Peers, with both lists read.
    pub fn open_peers(&mut self, patience: Duration) -> Step {
        // The menu is the window's own: no render draws it, so the trace
        // has no line for it opening; its first entry taking focus is the
        // sign (`MenuItem`'s first takes focus when the menu opens).
        let opened = self
            .press("app-options", &["focus=app-trust"], patience)
            .and_then(|()| self.press("app-peers", &["page=peers"], patience));
        self.answer("open_peers", opened)
    }

    /// Show my address, its code drawn (`show=shown`), then Back to
    /// Peers. The code's text is not read here: the screen is secured
    /// and the trace carries none of it, so the other phone's field gets
    /// it from the `own_addresses` case.
    pub fn show(&mut self, patience: Duration) -> Step {
        let shown = self
            .press("show-my-address", &["show=shown"], patience)
            .and_then(|()| self.back(&["show=closed"], patience));
        self.answer("show", shown)
    }

    /// Show my address held open for the other phone's camera: the step
    /// ends with the code drawn; [`Self::close_show`] closes it.
    pub fn hold_show(&mut self, patience: Duration) -> Step {
        let shown = self.press("show-my-address", &["show=shown"], patience);
        self.answer("hold_show", shown)
    }

    /// Back from a Show [`Self::hold_show`] left open.
    pub fn close_show(&mut self, patience: Duration) -> Step {
        let closed = self.back(&["show=closed"], patience);
        self.answer("close_show", closed)
    }

    /// Peers -> the add page, its first step.
    pub fn open_add(&mut self, patience: Duration) -> Step {
        let opened = self.press("add-address", &["page=peers-add"], patience);
        self.answer("open_add", opened)
    }

    /// Back from the add page to Peers, nothing added.
    pub fn leave_add(&mut self, patience: Duration) -> Step {
        let left = self.back(&["page=peers"], patience);
        self.answer("leave_add", left)
    }

    /// The field route: `line` typed into the address field (one line:
    /// `input text` types no newline, and the field splits on newlines
    /// alone), Add pressed, the add answered `added` and its trust step
    /// drawn.
    pub fn add_by_field(&mut self, line: &str, patience: Duration) -> Step {
        let name = "add_by_field";
        if !typable(line) {
            return step(
                name,
                false,
                format!("{line:?} holds a character `input text` would mangle"),
            );
        }
        let added = self
            .tab_to("address-field")
            .and_then(|()| self.adb_ok(&["shell", "input", "text", line]).map(drop))
            .and_then(|()| {
                self.press(
                    "add-action",
                    &["add=added lines=1", "page=peers-add-trust"],
                    patience,
                )
            });
        self.answer(name, added)
    }

    /// The camera route: Scan pressed, the viewfinder open, and the add
    /// answered `added` with its trust step drawn once the camera read
    /// the other phone's code -- which needs the phones facing each
    /// other, and the camera granted.
    pub fn add_by_scan(&mut self, patience: Duration) -> Step {
        let scanned = self.tab_to("scan-code").and_then(|()| {
            let from = self.seen.len();
            self.key("KEYCODE_ENTER")?;
            self.wait_all(from, &["page=peers-scan"], patience)?;
            self.wait_all(from, &["page=peers-add-trust"], patience)?;
            // A scan adds every line the code carries: how many is the
            // other phone's, so only the kind is asked.
            self.wait_prefix(from, "add=added lines=", patience)
        });
        self.answer("add_by_scan", scanned)
    }

    /// The trust step's one block: Trust pressed, and the block trusted.
    pub fn trust(&mut self, patience: Duration) -> Step {
        let trusted = self.tab_to("block-trust:").and_then(|()| {
            let handle = self
                .focus
                .strip_prefix("block-trust:")
                .map(str::to_owned)
                .ok_or_else(|| format!("focus on {:?}, not a block's Trust", self.focus))?;
            let from = self.seen.len();
            self.key("KEYCODE_ENTER")?;
            self.wait_all(from, &[&format!("block:{handle}=trusted")], patience)
        });
        self.answer("trust", trusted)
    }

    /// Every trace line read so far, for a run file.
    #[must_use]
    pub fn trace(&self) -> &[String] {
        &self.seen
    }

    fn answer(&self, name: &str, done: Result<(), String>) -> Step {
        match done {
            Ok(()) => step(name, true, self.where_now()),
            Err(why) => step(name, false, why),
        }
    }

    /// Focus moved to `key`, ENTER pressed, and every one of `then` seen
    /// after the press, in any order: one render writes its page line
    /// before its outcome lines.
    fn press(&mut self, key: &str, then: &[&str], patience: Duration) -> Result<(), String> {
        self.tab_to(key)?;
        let from = self.seen.len();
        self.key("KEYCODE_ENTER")?;
        self.wait_all(from, then, patience)
    }

    /// Back pressed, and every one of `then` seen after it.
    ///
    /// The soft keyboard is hidden first: while it is up the platform
    /// spends a Back on it and the app never sees one, and a page whose
    /// opening focuses a text field (the add page's address field) has
    /// it up.
    fn back(&mut self, then: &[&str], patience: Duration) -> Result<(), String> {
        self.settle()?;
        self.hide_keyboard()?;
        let from = self.seen.len();
        self.key("KEYCODE_BACK")?;
        self.wait_all(from, then, patience)
    }

    /// Back spent on the soft keyboard if it is up, and its going waited
    /// for; nothing pressed when it is down, since that Back would reach
    /// the app.
    fn hide_keyboard(&mut self) -> Result<(), String> {
        if !self.keyboard_up()? {
            return Ok(());
        }
        self.key("KEYCODE_BACK")?;
        let deadline = Instant::now() + TAB_PATIENCE;
        loop {
            if Instant::now() >= deadline {
                return Err(format!(
                    "the soft keyboard still up {TAB_PATIENCE:?} after a Back"
                ));
            }
            if !self.keyboard_up()? {
                return Ok(());
            }
            std::thread::sleep(POLL);
        }
    }

    /// Whether the input method shows its window, by the platform's own
    /// `mInputShown`, which API 30 and API 36 both print, at different
    /// places on a line ([`keyboard_shown`]).
    fn keyboard_up(&self) -> Result<bool, String> {
        let out = self.adb_ok(&["shell", "dumpsys", "input_method"])?;
        keyboard_shown(&out).ok_or_else(|| "dumpsys input_method printed no mInputShown".to_owned())
    }

    /// TAB until the trace names `key` (a prefix ending in `:` matches
    /// any of its handles), or fail naming where focus went.
    ///
    /// Judged on the SETTLED focus, never on the first focus line after
    /// a TAB: a page moves focus on its own as it opens (Peers to its
    /// title), and that line, read as a TAB's answer, pressed ENTER one
    /// control late (the 2026-10-25 Pixel run: Add opened for Show).
    fn tab_to(&mut self, key: &str) -> Result<(), String> {
        let at = |focus: &str| {
            if key.ends_with(':') {
                focus.starts_with(key)
            } else {
                focus == key
            }
        };
        self.settle()?;
        if at(&self.focus) {
            return Ok(());
        }
        let mut visited = Vec::new();
        for _ in 0..TABS {
            self.key("KEYCODE_TAB")?;
            let moved = self.settle()?;
            visited.push(if moved {
                self.focus.clone()
            } else {
                "(no focus line)".to_owned()
            });
            if at(&self.focus) {
                return Ok(());
            }
        }
        Err(format!(
            "{TABS} TABs never reached {key:?}; focus went {visited:?} on {}",
            self.where_now()
        ))
    }

    /// Read the trace until it has been quiet for [`QUIET`] (at most
    /// [`TAB_PATIENCE`]), following focus through what it read; whether
    /// focus moved.
    fn settle(&mut self) -> Result<bool, String> {
        let deadline = Instant::now() + TAB_PATIENCE;
        let mut quiet_since = Instant::now();
        let mut moved = false;
        loop {
            self.poll()?;
            let mut read_any = false;
            while self.next_focus().is_some() {
                moved = true;
                read_any = true;
            }
            let now = Instant::now();
            if read_any {
                quiet_since = now;
            }
            if now.duration_since(quiet_since) >= QUIET || now >= deadline {
                return Ok(moved);
            }
            std::thread::sleep(POLL);
        }
    }

    /// The next unread focus line's key, consuming the lines up to it.
    /// A wait never depends on what was consumed: it reads the lines
    /// from the index its key press was sent at.
    fn next_focus(&mut self) -> Option<String> {
        while self.read < self.seen.len() {
            let line = &self.seen[self.read];
            self.read += 1;
            if let Some(focus) = line
                .strip_prefix(TRACE)
                .and_then(|l| l.strip_prefix("focus="))
            {
                self.focus = focus.to_owned();
                return Some(self.focus.clone());
            }
        }
        None
    }

    /// Wait until every line of `wants` is among the trace's lines from
    /// index `from` on, following focus through what is read.
    fn wait_all(&mut self, from: usize, wants: &[&str], patience: Duration) -> Result<(), String> {
        let lines: Vec<String> = wants.iter().map(|w| format!("{TRACE}{w}")).collect();
        self.wait_until(patience, |seen| {
            let missing: Vec<&str> = wants
                .iter()
                .zip(&lines)
                .filter(|(_, l)| !seen[from..].contains(l))
                .map(|(w, _)| *w)
                .collect();
            if missing.is_empty() {
                None
            } else {
                Some(format!("{missing:?}"))
            }
        })
    }

    /// Wait until a line starting with `prefix` is among the trace's
    /// lines from index `from` on.
    fn wait_prefix(&mut self, from: usize, prefix: &str, patience: Duration) -> Result<(), String> {
        let start = format!("{TRACE}{prefix}");
        self.wait_until(patience, |seen| {
            (!seen[from..].iter().any(|l| l.starts_with(&start))).then(|| format!("{prefix:?}..."))
        })
    }

    /// Poll until `missing` says nothing is missing, or fail naming what
    /// it says was.
    fn wait_until(
        &mut self,
        patience: Duration,
        missing: impl Fn(&[String]) -> Option<String>,
    ) -> Result<(), String> {
        let deadline = Instant::now() + patience;
        loop {
            self.poll()?;
            while self.next_focus().is_some() {}
            let Some(what) = missing(&self.seen) else {
                return Ok(());
            };
            if Instant::now() >= deadline {
                return Err(format!(
                    "no {what} within {patience:?}; {}",
                    self.where_now()
                ));
            }
            std::thread::sleep(POLL);
        }
    }

    /// The trace lines logged since the launch, read again whole: the
    /// app's lines go to the `interweave` tag (apps/human-android's
    /// jni.rs), and the log was cleared at the launch.
    fn poll(&mut self) -> Result<(), String> {
        let log = self.adb_ok(&["logcat", "-d", "-v", "raw", "-s", "interweave:I"])?;
        let lines = trace_lines(&log);
        if lines.len() < self.seen.len() || lines[..self.seen.len()] != self.seen[..] {
            return Err(format!(
                "the trace went back on itself (the log cleared or cut under the run): {} lines seen, {} now",
                self.seen.len(),
                lines.len()
            ));
        }
        self.seen = lines;
        Ok(())
    }

    /// Where the trace says the app is: its last page, the focus, and
    /// the last lines, for a step's detail.
    fn where_now(&self) -> String {
        let page = self
            .seen
            .iter()
            .rev()
            .find_map(|l| l.strip_prefix(TRACE).and_then(|l| l.strip_prefix("page=")))
            .unwrap_or("(none)");
        let tail = &self.seen[self.seen.len().saturating_sub(8)..];
        format!(
            "page {page}, focus {:?}; last trace lines {tail:?}",
            self.focus
        )
    }

    fn key(&self, code: &str) -> Result<(), String> {
        self.adb_ok(&["shell", "input", "keyevent", code]).map(drop)
    }

    fn adb(&self, args: &[&str]) -> std::io::Result<Output> {
        Command::new("adb")
            .args(["-s", &self.serial])
            .args(args)
            .output()
    }

    fn adb_ok(&self, args: &[&str]) -> Result<String, String> {
        match self.adb(args) {
            Err(e) => Err(format!("adb {}: {e}", args.join(" "))),
            Ok(out) if !out.status.success() => Err(format!(
                "adb {} ended {}: {}",
                args.join(" "),
                out.status,
                String::from_utf8_lossy(&out.stderr).trim_end()
            )),
            Ok(out) => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        }
    }
}

/// The trace's lines in a `raw` logcat, in order.
#[must_use]
pub fn trace_lines(log: &str) -> Vec<String> {
    log.lines()
        .map(str::trim_end)
        .filter(|l| l.starts_with(TRACE))
        .map(str::to_owned)
        .collect()
}

/// Whether `input text` types `line` as it is: it reads `%s` as a space
/// and the device shell would read its metacharacters.
fn typable(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | ':' | '-' | '_'))
}

/// `mInputShown` out of `dumpsys input_method`, the first one printed:
/// the service's own, above any client's state it lists after. A token
/// anywhere on its line: API 36 (the Pixel) prints it alone, API 30 (the
/// A40) after `mShowRequested=` and its kin on one line.
fn keyboard_shown(dump: &str) -> Option<bool> {
    dump.split_whitespace()
        .find_map(|t| t.strip_prefix("mInputShown="))
        .and_then(|v| match v {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        })
}

fn step(case: &str, passed: bool, detail: String) -> Step {
    let mut out = Map::new();
    out.insert(keys::CASE.to_owned(), json!(case));
    out.insert(
        keys::RESULT.to_owned(),
        json!(if passed { keys::PASS } else { keys::FAIL }),
    );
    out.insert(keys::DETAIL.to_owned(), Value::String(detail));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trace_is_read_out_of_the_apps_other_lines() {
        let log = "the runtime started\nui-trace page=conversations\r\nui-trace focus=app-options\n  ui-trace not at the start\nui-trace show=shown\n";
        assert_eq!(
            trace_lines(log),
            [
                "ui-trace page=conversations",
                "ui-trace focus=app-options",
                "ui-trace show=shown"
            ]
        );
    }

    #[test]
    fn the_keyboard_is_read_from_the_first_input_shown_line() {
        let up = "Input method service state:\n  mImeWindowVis=3\n  mInputShown=true\n  mInputShown=false\n";
        assert_eq!(keyboard_shown(up), Some(true));
        let down = "  mImeWindowVis=0\r\n      mInputShown=false\r\n";
        assert_eq!(keyboard_shown(down), Some(false));
        let api30 = "  mShowRequested=true mShowExplicitlyRequested=false mShowForced=false mInputShown=true\n  mInputShown=false\n";
        assert_eq!(keyboard_shown(api30), Some(true));
        assert_eq!(keyboard_shown("mImeWindowVis=0\n"), None);
        assert_eq!(keyboard_shown("mInputShown=maybe\n"), None);
    }

    #[test]
    fn only_an_address_input_text_types_as_it_is_is_typed() {
        assert!(typable("/ip4/192.168.100.6/tcp/4001/p2p/12D3KooWExample"));
        assert!(typable("/ip6/fe80::1/udp/4001/quic-v1"));
        for line in ["", "a b", "a%sb", "a;b", "a'b", "a\nb", "a$b", "a&b"] {
            assert!(!typable(line), "{line:?}");
        }
    }

    #[test]
    fn a_step_is_the_cases_shape() {
        let s = step("trust", false, "why".to_owned());
        assert_eq!(s.get(keys::CASE), Some(&json!("trust")));
        assert_eq!(s.get(keys::RESULT), Some(&json!(keys::FAIL)));
        assert_eq!(s.get(keys::DETAIL), Some(&json!("why")));
    }
}
