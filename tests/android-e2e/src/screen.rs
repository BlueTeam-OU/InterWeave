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

/// How many TABs [`Screens::read_whole`] presses before it gives up: a
/// cut row's preview of autolinks holds some 20 link controls, which
/// follow its Read the whole message and come before the next row's.
const READ_TABS: usize = 80;

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
    /// The adb program: `adb` on the PATH, a stand-in in this module's
    /// tests.
    program: std::ffi::OsString,
    /// The trace lines read so far since the launch cleared the log.
    seen: Vec<String>,
    /// How many of `seen` a wait has already consumed.
    read: usize,
    /// The focus key the trace last named.
    focus: String,
    /// The app's other lines on its tag (the runtime's start among
    /// them), as last read: what a failed step cannot be judged without.
    app: Vec<String>,
}

impl Screens {
    /// The app on the phone with `serial`; nothing is sent yet.
    #[must_use]
    pub fn new(serial: &str) -> Self {
        Self {
            serial: serial.to_owned(),
            program: "adb".into(),
            seen: Vec::new(),
            read: 0,
            focus: String::new(),
            app: Vec::new(),
        }
    }

    /// Start the app from nothing: force-stop it (focus survives an
    /// `am start` of a running activity, so a step never starts from
    /// where the last left focus), clear the log, launch, and wait for
    /// the conversation list to be the page the app settles on: once the
    /// trace says the start no longer waits on Connect's reads
    /// (`connect=` not-owed, settled or open). Owed and deferred are
    /// waited past within `patience`: a deferred start decides again only
    /// in a fresh model, when the service's running state changes, so a
    /// read that keeps failing ends the step naming the last phase.
    /// A phone that owes onboarding's Connect (no peer, no trust entry)
    /// settles on Connect, and the step fails saying so: every later
    /// step starts from Chats, and leaving Connect writes the profile.
    pub fn launch(&mut self, patience: Duration) -> Step {
        let name = "launch";
        if let Err(why) = self.unlocked() {
            return step(name, false, why);
        }
        let _ = self.adb(&["shell", "am", "force-stop", PACKAGE]);
        if let Err(why) = self.adb_ok(&["logcat", "-c"]) {
            return step(name, false, why);
        }
        (self.seen, self.read, self.focus, self.app) = (Vec::new(), 0, String::new(), Vec::new());
        let launched = self
            .front()
            .and_then(|()| self.wait_all(0, &["page=conversations"], patience))
            .and_then(|()| {
                self.wait_until(patience, |seen| match last_of(seen, "connect=") {
                    Some("not-owed" | "settled" | "open") => None,
                    other => Some(format!(
                        "connect= line saying the start decided (last {other:?})"
                    )),
                })
            })
            .and_then(|()| match last_of(&self.seen, "page=") {
                Some("conversations") => self.app_takes_keys(),
                Some("connect") => Err(format!(
                    "the app settled on Connect, which a phone with no peer and no trust entry owes; the driver starts from Chats and does not leave Connect, which writes the profile; {}",
                    self.where_now()
                )),
                _ => Err(format!(
                    "the app left the conversation list on its own; {}",
                    self.where_now()
                )),
            });
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

    /// Chats -> the conversation whose row TAB reaches first, drawn.
    pub fn open_conversation(&mut self, patience: Duration) -> Step {
        let opened = self.press("conversation:", &["page=conversation"], patience);
        self.answer("open_conversation", opened)
    }

    /// The first Read the whole message TAB reaches whose focus key is
    /// not in `opened`, pressed, and the reader's first frame: the step
    /// carries the row's focus key as `read` and the trace's `ms`, from
    /// the press reaching the view to that frame (design/2026-11-04-
    /// long-message.md A5). What reaches the phone before the view, the
    /// key's way through `input` and the platform, is not in it.
    pub fn read_whole(&mut self, opened: &[String], patience: Duration) -> Step {
        let name = "read_whole";
        let reached = self.tab_until("a Read the whole message not yet opened", READ_TABS, |f| {
            f.starts_with("read:") && !opened.iter().any(|o| o == f)
        });
        if let Err(why) = reached {
            return step(name, false, why);
        }
        let read = self.focus.clone();
        let from = self.seen.len();
        let drawn = self
            .key("KEYCODE_ENTER")
            .and_then(|()| self.wait_all(from, &["page=reader"], patience))
            .and_then(|()| self.said(from, "reader=drawn ", patience));
        let mut out = self.timed(name, drawn);
        out.insert("read".to_owned(), Value::String(read));
        out
    }

    /// Show source, or Show formatted, in the open reader.
    pub fn reader_switch(&mut self) -> Step {
        // The switch draws no page: the next End line names the form it
        // measured, which is the check.
        let switched = self
            .tab_to("reader-source")
            .and_then(|()| self.key("KEYCODE_ENTER"))
            .and_then(|()| self.settle().map(drop));
        self.answer("reader_switch", switched)
    }

    /// End in the open reader, its list focused first, and the trace's
    /// time to the frame that shows the list at its end: the step carries
    /// `ms`, `frames`, `at-end` and the reader's `form` (A5).
    pub fn reader_end(&mut self, patience: Duration) -> Step {
        let ended = self.tab_to("reader-scroll").and_then(|()| {
            let from = self.seen.len();
            self.key("KEYCODE_MOVE_END")?;
            self.said(from, "reader=end ", patience)
        });
        self.timed("reader_end", ended)
    }

    /// Back from the reader to its conversation, focus on the row's Read
    /// the whole message (section 5.4).
    pub fn close_reader(&mut self, patience: Duration) -> Step {
        let closed = self.back(&["page=conversation"], patience).and_then(|()| {
            self.settle()?;
            if self.focus.starts_with("read:") {
                Ok(())
            } else {
                Err(format!(
                    "focus left on {:?}, not a Read the whole message",
                    self.focus
                ))
            }
        });
        self.answer("close_reader", closed)
    }

    /// The first Read the whole message TAB reaches whose focus key is
    /// not in `opened`, pressed, and Back sent `after` the press whatever
    /// the reader has drawn by then: a person who leaves while it opens
    /// (design/2026-11-04-long-message.md A5). Passes once the trace
    /// shows the reader's page and then the conversation's again, focus
    /// back on the opened row's Open the whole message. The step carries `read`; `drawn`,
    /// whether the reader's rows were drawn before it closed; and
    /// `enter-at` and `back-at`, the phone's time of day as each key's
    /// `input` was launched, to set beside the trace's logcat times.
    /// `on_row` runs once focus is on the row, before the keys: a
    /// recording started there spans the keys and not the TABs.
    ///
    /// With `second`, a second Back follows the first by that long, and
    /// the step passes once the trace shows the reader's page, the
    /// conversation's and then the chat list's: the first Back closed the
    /// reader and the second acted on the conversation, neither lost nor
    /// merged (section 5.4); focus is then the list's, not the row's, and
    /// the step adds `back2-at`.
    ///
    /// The keys go from one shell, each `input` but the last started in the
    /// background, so the gap between two is its sleep give or take the
    /// difference between their `input`s' start-up, not the sleep plus one
    /// start-up as separate adb calls would make it.
    pub fn read_whole_then_back(
        &mut self,
        opened: &[String],
        after: Duration,
        second: Option<Duration>,
        patience: Duration,
        on_row: impl FnOnce(),
    ) -> Step {
        let name = "read_whole_then_back";
        let reached = self.tab_until("a Read the whole message not yet opened", READ_TABS, |f| {
            f.starts_with("read:") && !opened.iter().any(|o| o == f)
        });
        if let Err(why) = reached {
            return step(name, false, why);
        }
        let read = self.focus.clone();
        on_row();
        let from = self.seen.len();
        // Each key's launch stamped on the phone's clock, the clock its
        // logcat stamps the trace by: a run's Back-to-close and how long
        // the first screen showed are read from the two together. The
        // Enter's stamp comes just before its launch and the Back's just
        // after, so the sleep alone sits between the two launches.
        let again = second.map_or_else(String::new, |gap| {
            format!(
                " sleep {:.3}; input keyevent KEYCODE_BACK & echo back2-at $(date +%T.%N);",
                gap.as_secs_f64()
            )
        });
        let keys = format!(
            "echo enter-at $(date +%T.%N); input keyevent KEYCODE_ENTER & sleep {:.3}; input keyevent KEYCODE_BACK & echo back-at $(date +%T.%N);{again} wait",
            after.as_secs_f64()
        );
        let reader = format!("{TRACE}page=reader");
        let conversation = format!("{TRACE}page=conversation");
        let list = format!("{TRACE}page=conversations");
        let pages: &[&str] = if second.is_some() {
            &["page=reader", "page=conversation", "page=conversations"]
        } else {
            &["page=reader", "page=conversation"]
        };
        let mut stamps = Vec::new();
        let closed = self
            .adb_ok(&["shell", &keys])
            .map(|said| {
                // The keys' stamps alone: any other line of the shell's
                // must not overwrite a field of the step.
                stamps = said
                    .lines()
                    .filter_map(|l| l.split_once(' '))
                    .filter(|(k, _)| matches!(*k, "enter-at" | "back-at" | "back2-at"))
                    .map(|(k, v)| (k.to_owned(), v.trim().to_owned()))
                    .collect();
            })
            .and_then(|()| self.wait_all(from, pages, patience))
            .and_then(|()| {
                let lines = &self.seen[from..];
                let opened_at = lines.iter().position(|l| *l == reader);
                let back_at = lines.iter().rposition(|l| *l == conversation);
                let left_at = lines.iter().rposition(|l| *l == list);
                match (opened_at, back_at, left_at) {
                    (Some(o), Some(b), None) if o < b && second.is_none() => Ok(()),
                    (Some(o), Some(b), Some(l)) if o < b && b < l && second.is_some() => Ok(()),
                    _ => Err(format!(
                        "the pages did not follow each other as the Backs ask ({pages:?}); {}",
                        self.where_now()
                    )),
                }
            })
            .and_then(|()| self.settle().map(drop))
            .and_then(|()| {
                if second.is_some() {
                    return Ok(());
                }
                // Its own row's control, not any: a conversation of several
                // cut rows has an earlier one to land on by mistake.
                if self.focus == read {
                    Ok(())
                } else {
                    Err(format!(
                        "focus left on {:?}, not the opened row's {read:?}",
                        self.focus
                    ))
                }
            });
        let drawn_line = format!("{TRACE}reader=drawn ");
        let drawn = self.seen[from..].iter().any(|l| l.starts_with(&drawn_line));
        let mut out = self.answer(name, closed);
        out.insert("read".to_owned(), Value::String(read));
        out.insert(
            "drawn".to_owned(),
            Value::String(if drawn { "yes" } else { "no" }.to_owned()),
        );
        for (key, at) in stamps {
            out.insert(key, Value::String(at));
        }
        out
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
    /// drawn. A note the add page shows instead -- a refusal, a failure
    /// -- or another result (`unchanged`, `checked`) ends the wait at
    /// once, named.
    pub fn add_by_field(&mut self, line: &str, patience: Duration) -> Step {
        let name = "add_by_field";
        if !typable(line) {
            return step(
                name,
                false,
                format!(
                    "{line:?} holds a character `input text` would mangle or key events \
                     cannot type (a capital or a shifted symbol: the Slint 1.18.1 limit)"
                ),
            );
        }
        let added = self
            .tab_to("address-field")
            .and_then(|()| self.adb_ok(&["shell", "input", "text", line]).map(drop))
            .and_then(|()| self.tab_to("add-action"))
            .and_then(|()| {
                let from = self.seen.len();
                self.key("KEYCODE_ENTER")?;
                self.wait_answer(
                    from,
                    &["add=added lines=1", "page=peers-add-trust"],
                    patience,
                )
            });
        self.answer(name, added)
    }

    /// The camera route: Scan pressed, the viewfinder open, and the add
    /// answered `added` with its trust step drawn once the camera read
    /// the other phone's code -- which needs the phones facing each
    /// other, and the camera granted. A note the add page shows -- the
    /// camera denied, the code refused -- or another result (`unchanged`
    /// on a phone already holding the address, `checked`) ends the wait,
    /// named, as on the field route.
    pub fn add_by_scan(&mut self, patience: Duration) -> Step {
        let scanned = self.tab_to("scan-code").and_then(|()| {
            let from = self.seen.len();
            self.key("KEYCODE_ENTER")?;
            self.wait_scanned(from, patience)
        });
        self.answer("add_by_scan", scanned)
    }

    /// A scan's answer among the trace's lines from `from` on: the scan
    /// page, the trust step and the add's `added`, or a note or another
    /// result seen first.
    fn wait_scanned(&mut self, from: usize, patience: Duration) -> Result<(), String> {
        self.wait_answer(from, &["page=peers-scan"], patience)?;
        self.wait_answer(from, &["page=peers-add-trust"], patience)?;
        // A scan adds every line the code carries: how many is the
        // other phone's, so only the kind is asked -- and any result
        // ends the wait, since `unchanged` draws the trust step too.
        self.wait_until(patience, |seen| {
            result_in(&seen[from..])
                .is_none()
                .then(|| "add= result".to_owned())
        })?;
        answered(&self.seen[from..], &[]).unwrap_or_else(Err)
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

    /// A cold launch of a profile whose key waits for the person
    /// (`user-presence`, past its validity): the conversation list with
    /// the window offline and the notice that asks for authentication,
    /// from the first render. Connect is not waited for: its reads need
    /// the runtime, which the locked key holds back.
    ///
    /// A keyguard up here fails the step rather than asking for an
    /// unlock: the unlock would open the key's window again, the very
    /// state this launch must not be in, so the person is asked before
    /// the validity wait (`identity_lines`'s `locked_launch`), not after.
    pub fn launch_locked(&mut self, patience: Duration) -> Step {
        let name = "launch_locked";
        if let Err(why) = self.awake_unasked() {
            return step(name, false, why);
        }
        let _ = self.adb(&["shell", "am", "force-stop", PACKAGE]);
        if let Err(why) = self.adb_ok(&["logcat", "-c"]) {
            return step(name, false, why);
        }
        (self.seen, self.read, self.focus, self.app) = (Vec::new(), 0, String::new(), Vec::new());
        let launched = self.front().and_then(|()| {
            self.wait_all(
                0,
                &[
                    "page=conversations",
                    "connectivity=offline",
                    "notice=authentication-needed",
                ],
                patience,
            )
        });
        self.answer(name, launched)
    }

    /// Conversations -> the app's menu -> Trust (Settings), where Show
    /// recovery phrase is.
    pub fn open_trust(&mut self, patience: Duration) -> Step {
        let opened = self
            .press("app-options", &["focus=app-trust"], patience)
            .and_then(|()| self.press("app-trust", &["page=trust"], patience));
        self.answer("open_trust", opened)
    }

    /// Trust -> Show recovery phrase: S1, before any prompt.
    pub fn open_phrase(&mut self, patience: Duration) -> Step {
        let opened = self.press("show-phrase", &["identity=s1"], patience);
        self.answer("open_phrase", opened)
    }

    /// S1's primary: the credential prompt for the words asked.
    pub fn ask_words(&mut self, patience: Duration) -> Step {
        let asked = self.press("identity-primary", &["prompt=phrase"], patience);
        self.answer("ask_words", asked)
    }

    /// The person's grant, and the words page drawn: `prompt=granted`,
    /// then `identity=s2`, both after `from` ([`Self::mark`]).
    pub fn words_shown(&mut self, from: usize, patience: Duration) -> Step {
        let shown = self.wait_all(from, &["prompt=granted", "identity=s2"], patience);
        self.answer("words_shown", shown)
    }

    /// Back from the words: the flow closed.
    pub fn close_words(&mut self, patience: Duration) -> Step {
        let closed = self.back(&["identity=closed"], patience);
        self.answer("close_words", closed)
    }

    /// Recents pressed while the prompt shows: the platform ends it as
    /// the person's cancel (`onError` 10 on the A40), `prompt=cancelled`.
    pub fn leave_by_recents(&mut self, patience: Duration) -> Step {
        let from = self.seen.len();
        let left = self
            .key("KEYCODE_APP_SWITCH")
            .and_then(|()| self.wait_all(from, &["prompt=cancelled"], patience));
        self.answer("leave_by_recents", left)
    }

    /// The app brought back to the front, not relaunched: its task
    /// resumed.
    pub fn bring_back(&mut self) -> Step {
        let back = self.front();
        self.answer("bring_back", back)
    }

    /// The locked notice's Unlock: the unlock prompt asked.
    pub fn unlock(&mut self, patience: Duration) -> Step {
        let asked = self.press("notice", &["prompt=unlock"], patience);
        self.answer("unlock", asked)
    }

    /// A cold launch onto the identity flow's `screen` (`w` on a first
    /// run, `k1` when the identity's key cannot be opened), its trace line
    /// seen: the onboarding pages, where `launch` waits for Conversations.
    pub fn launch_to_identity(&mut self, screen: &str, patience: Duration) -> Step {
        let name = "launch_to_identity";
        if let Err(why) = self.unlocked() {
            return step(name, false, why);
        }
        let _ = self.adb(&["shell", "am", "force-stop", PACKAGE]);
        if let Err(why) = self.adb_ok(&["logcat", "-c"]) {
            return step(name, false, why);
        }
        (self.seen, self.read, self.focus, self.app) = (Vec::new(), 0, String::new(), Vec::new());
        let want = format!("identity={screen}");
        let launched = self
            .front()
            .and_then(|()| self.wait_all(0, &[want.as_str()], patience));
        self.answer(name, launched)
    }

    /// Welcome's secondary, Restore: R1.
    pub fn open_restore(&mut self, patience: Duration) -> Step {
        let opened = self.press("identity-secondary", &["identity=r1"], patience);
        self.answer("open_restore", opened)
    }

    /// The screen's primary, Enter recovery phrase on R1 and on K1: R2.
    pub fn open_phrase_entry(&mut self, patience: Duration) -> Step {
        let opened = self.press("identity-primary", &["identity=r2"], patience);
        self.answer("open_phrase_entry", opened)
    }

    /// Back from an identity screen, and every one of `then` after it.
    pub fn identity_back(&mut self, then: &[&str], patience: Duration) -> Step {
        let left = self.back(then, patience);
        self.answer("identity_back", left)
    }

    /// Every one of `wants` among the lines after `from`.
    pub fn lines_after(&mut self, from: usize, wants: &[&str], patience: Duration) -> Step {
        let seen = self.wait_all(from, wants, patience);
        self.answer("lines_after", seen)
    }

    /// No line starting with `prefix` after `from` for all of `quiet`:
    /// a step that fails on the first such line. The positive control
    /// is the caller's: a line the same prefix carries that it waited
    /// for before.
    pub fn none_after(&mut self, from: usize, prefix: &str, quiet: Duration) -> Step {
        let start = format!("{TRACE}{prefix}");
        let deadline = Instant::now() + quiet;
        let held = loop {
            if let Err(why) = self.poll() {
                break Err(why);
            }
            while self.next_focus().is_some() {}
            if let Some(l) = self.seen[from..].iter().find(|l| l.starts_with(&start)) {
                break Err(format!("{l:?} within {quiet:?}; {}", self.where_now()));
            }
            if Instant::now() >= deadline {
                break Ok(());
            }
            std::thread::sleep(POLL);
        };
        self.answer("none_after", held)
    }

    /// How many trace lines have been read: the index a later wait
    /// reads from.
    #[must_use]
    pub fn mark(&self) -> usize {
        self.seen.len()
    }

    /// Every trace line read so far, for a run file.
    #[must_use]
    pub fn trace(&self) -> &[String] {
        &self.seen
    }

    /// The app's lines on its tag that are not the trace, as last read
    /// since the launch cleared the log: a step that failed on a page
    /// waiting for the runtime (`notice=no-daemon`) is judged by them.
    #[must_use]
    pub fn app_log(&self) -> &[String] {
        &self.app
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
        let out = self.dump(&["input_method"])?;
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
        self.tab_until(&format!("{key:?}"), TABS, |focus| {
            if key.ends_with(':') {
                focus.starts_with(key)
            } else {
                focus == key
            }
        })
    }

    /// TAB, at most `tabs` times, until the settled focus is `at`, or fail
    /// naming `what` and where focus went.
    fn tab_until(
        &mut self,
        what: &str,
        tabs: usize,
        at: impl Fn(&str) -> bool,
    ) -> Result<(), String> {
        self.settle()?;
        if at(&self.focus) {
            return Ok(());
        }
        let mut visited = Vec::new();
        for _ in 0..tabs {
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
            "{tabs} TABs never reached {what}; focus went {visited:?} on {}",
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

    /// The rest of the first trace line after `from` that starts with
    /// `prefix`, waited for.
    fn said(&mut self, from: usize, prefix: &str, patience: Duration) -> Result<String, String> {
        let want = format!("{TRACE}{prefix}");
        self.wait_until(patience, |seen| {
            (!seen[from..].iter().any(|l| l.starts_with(&want))).then(|| format!("{prefix:?}"))
        })?;
        Ok(self.seen[from..]
            .iter()
            .find_map(|l| l.strip_prefix(&want))
            .unwrap_or_default()
            .to_owned())
    }

    /// `name`'s answer, its detail where it stands and each `key=value`
    /// of a timing line as a field of its own.
    fn timed(&self, name: &str, said: Result<String, String>) -> Step {
        match said {
            Ok(line) => {
                let mut out = step(name, true, self.where_now());
                for (k, v) in fields(&line) {
                    out.insert(k.to_owned(), Value::String(v.to_owned()));
                }
                out
            }
            Err(why) => step(name, false, why),
        }
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

    /// Every one of `wants` seen from `from` on, or an add page's note
    /// or a result other than `added` seen first, which fails the wait
    /// naming it: a note is the add's answer as much as a result is.
    fn wait_answer(
        &mut self,
        from: usize,
        wants: &[&str],
        patience: Duration,
    ) -> Result<(), String> {
        self.wait_until(patience, |seen| answered(&seen[from..], wants).err())?;
        answered(&self.seen[from..], wants).unwrap_or_else(Err)
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
        self.app = app_lines(&log);
        Ok(())
    }

    /// Where the trace says the app is: its last page, the focus, and
    /// the last lines, for a step's detail.
    fn where_now(&self) -> String {
        let page = last_of(&self.seen, "page=").unwrap_or("(none)");
        let tail = &self.seen[self.seen.len().saturating_sub(8)..];
        format!(
            "page {page}, focus {:?}; last trace lines {tail:?}",
            self.focus
        )
    }

    /// The phone awake and unlocked, as [`crate::platform::until_unlocked`]
    /// says and on this serial: a launch behind a secure keyguard never
    /// ends, so a locked phone asks the person and is waited out, and
    /// one still locked after the patience fails the launch naming it.
    fn unlocked(&self) -> Result<(), String> {
        let patience = crate::platform::UNLOCK_PATIENCE;
        self.key("KEYCODE_WAKEUP")?;
        let locked = || crate::platform::keyguard_showing(&self.dump(&["window"])?);
        if !locked()? {
            return Ok(());
        }
        eprintln!(
            "PERSON: on the phone {}, unlock it (within {patience:?})",
            self.serial
        );
        let deadline = Instant::now() + patience;
        while locked()? {
            if Instant::now() >= deadline {
                return Err(format!("the phone stayed locked for {patience:?}"));
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        Ok(())
    }

    /// The app's window is the one key events reach. A system window in
    /// front -- the platform's permission prompt the app raises at launch
    /// on API 33+ while notifications are not granted, the notification
    /// shade, a keyguard -- takes every key the driver sends, and the
    /// trace then says nothing at all: fail here naming it instead.
    fn app_takes_keys(&self) -> Result<(), String> {
        let (window, app) = crate::platform::input_focus(&self.dump(&["window"])?)?;
        let ours = format!("{PACKAGE}/");
        if window.as_deref().is_some_and(|w| w.starts_with(&ours)) {
            return Ok(());
        }
        let prompt = app
            .as_deref()
            .is_some_and(|a| a.contains("GrantPermissionsActivity"));
        Err(format!(
            "key events go to window {window:?} (focused app {app:?}), not the app's{}",
            if prompt {
                ": the platform's permission prompt is in front; after a pm clear on API 33+ \
                 grant android.permission.POST_NOTIFICATIONS before the run"
            } else {
                ""
            }
        ))
    }

    /// `dumpsys <args>` on this serial, whole ([`crate::platform::dump_ended`]).
    fn dump(&self, args: &[&str]) -> Result<String, String> {
        let mut words = vec!["shell"];
        words.extend(crate::platform::dump_command(args));
        crate::platform::dump_ended(&self.adb_ok(&words)?)
    }

    /// The phone woken and found unlocked, asking no one: a keyguard up
    /// is an error naming why it cannot be asked for.
    fn awake_unasked(&self) -> Result<(), String> {
        self.key("KEYCODE_WAKEUP")?;
        if crate::platform::keyguard_showing(&self.dump(&["window"])?)? {
            return Err("the phone locked during the validity wait: an unlock now would open the user-presence key's window, so the launch cannot ask for one".to_owned());
        }
        Ok(())
    }

    /// `am start` of the main activity: launches it, or brings its task
    /// to the front. Refused behind a keyguard, asking no one: there
    /// `am start -W` waits with no end (51 minutes on the A40), so a
    /// step that may ask the person calls [`Self::unlocked`] first, and
    /// one that must not ([`Self::launch_locked`]) fails here instead.
    fn front(&self) -> Result<(), String> {
        if crate::platform::keyguard_showing(&self.dump(&["window"])?)? {
            return Err(format!(
                "the phone {} is locked: am start -W would wait behind the keyguard with no end",
                self.serial
            ));
        }
        self.adb_ok(&[
            "shell",
            "am",
            "start",
            "-W",
            "-n",
            &format!("{PACKAGE}/.MainActivity"),
        ])
        .map(drop)
    }

    fn key(&self, code: &str) -> Result<(), String> {
        self.adb_ok(&["shell", "input", "keyevent", code]).map(drop)
    }

    fn adb(&self, args: &[&str]) -> std::io::Result<Output> {
        Command::new(&self.program)
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

/// What the trace last said after `key` (`page=`, `connect=`), if it
/// said it at all.
fn last_of<'a>(seen: &'a [String], key: &str) -> Option<&'a str> {
    seen.iter()
        .rev()
        .find_map(|l| l.strip_prefix(TRACE).and_then(|l| l.strip_prefix(key)))
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

/// The lines of a `raw` logcat that are not the trace's, in order.
#[must_use]
pub fn app_lines(log: &str) -> Vec<String> {
    log.lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty() && !l.starts_with(TRACE))
        .map(str::to_owned)
        .collect()
}

/// Whether `input text` types `line` as it is: it reads `%s` as a space,
/// the device shell would read its metacharacters, and it sends key
/// events, whose Shift the app's Slint 1.18.1 backend drops
/// (apps/human-android/README.md): no capital and no shifted symbol
/// reaches the field. A `PeerId` has capitals, so the field route cannot
/// take one over adb until that bump; a lowercased one is refused as not
/// an address (A40 and Pixel, 2026-11-08/09).
fn typable(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '/' | '.' | '-'))
}

/// The first add-page note among trace `lines`, by kind: `none` is not
/// one, since Add pressed again clears the last note before its answer.
fn note_in(lines: &[String]) -> Option<&str> {
    lines.iter().find_map(|l| {
        l.strip_prefix(TRACE)?
            .strip_prefix("note=")
            .filter(|kind| *kind != "none")
    })
}

/// The first add result among trace `lines`, after its `add=`: a kind
/// and its `lines=` (`unchanged lines=1`). `add=waiting` and `add=none`
/// carry no `lines=` and are not one.
fn result_in(lines: &[String]) -> Option<&str> {
    lines.iter().find_map(|l| {
        l.strip_prefix(TRACE)?
            .strip_prefix("add=")
            .filter(|r| r.contains(" lines="))
    })
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

/// A trace line's `key=value` words.
fn fields(line: &str) -> impl Iterator<Item = (&str, &str)> {
    line.split(' ').filter_map(|w| w.split_once('='))
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

/// An add's answer among `seen`, the trace's lines since its key press:
/// `Ok` with the answer once a note or a result other than `added`
/// shows -- the failure naming it, whatever else is missing -- or once
/// every one of `wants` shows; `Err` naming what is still missing while
/// none has.
fn answered(seen: &[String], wants: &[&str]) -> Result<Result<(), String>, String> {
    if let Some(kind) = note_in(seen) {
        return Ok(Err(format!("answered note={kind}")));
    }
    if let Some(result) = result_in(seen).filter(|r| !r.starts_with("added ")) {
        return Ok(Err(format!("answered add={result}")));
    }
    let missing: Vec<&str> = wants
        .iter()
        .filter(|w| !seen.contains(&format!("{TRACE}{w}")))
        .copied()
        .collect();
    if missing.is_empty() {
        Ok(Ok(()))
    } else {
        Err(format!("{missing:?}"))
    }
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
    fn the_page_and_connect_are_the_last_the_trace_named() {
        let seen = |lines: &[&str]| lines.iter().map(|&l| l.to_owned()).collect::<Vec<_>>();
        let owed = seen(&[
            "ui-trace page=conversations",
            "ui-trace connect=owed",
            "ui-trace focus=app-options",
            "ui-trace connect=deferred",
            "ui-trace connect=owed",
            "ui-trace page=connect",
            "ui-trace connect=open",
            "ui-trace focus=connect-continue",
        ]);
        assert_eq!(last_of(&owed, "page="), Some("connect"));
        assert_eq!(last_of(&owed, "connect="), Some("open"));
        let waiting = seen(&["ui-trace page=conversations", "ui-trace connect=owed"]);
        assert_eq!(last_of(&waiting, "page="), Some("conversations"));
        assert_eq!(last_of(&waiting, "connect="), Some("owed"));
        assert_eq!(
            last_of(&seen(&["ui-trace focus=app-options"]), "page="),
            None
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
    fn the_apps_other_lines_are_every_line_the_trace_is_not() {
        let log = "the runtime started\nui-trace page=conversations\r\n\n  ui-trace not at the start\nstart answered 0\r\n";
        assert_eq!(
            app_lines(log),
            [
                "the runtime started",
                "  ui-trace not at the start",
                "start answered 0"
            ]
        );
    }

    #[test]
    fn only_an_address_input_text_types_as_it_is_is_typed() {
        assert!(typable("/ip4/192.168.100.6/tcp/4001/p2p"));
        assert!(typable("/dns4/peer-1.example/udp/4001/quic-v1"));
        for line in [
            "",
            "a b",
            "a%sb",
            "a;b",
            "a'b",
            "a\nb",
            "a$b",
            "a&b",
            // Key events lose Shift: a capital, `:` and `_` arrive changed.
            "/ip4/192.168.100.6/tcp/4001/p2p/12D3KooWExample",
            "/ip6/fe80::1/udp/4001/quic-v1",
            "a_b",
        ] {
            assert!(!typable(line), "{line:?}");
        }
    }

    /// Whether a stand-in call is a dump, by the command word
    /// `dump_command` builds (`tests/dump_reads.rs` keeps the word there).
    fn dumps(call: &str) -> bool {
        call.contains(crate::platform::dump_command(&[])[0])
    }

    /// Key focus on the app's own window, as the Pixel printed it.
    const APP_FOCUS: &str = "  mCurrentFocus=Window{1a2b u0 org.interweave.human/org.interweave.human.MainActivity}\n  mFocusedApp=ActivityRecord{77 u0 org.interweave.human/.MainActivity t9}";

    /// A stand-in adb in a scratch directory: a `dumpsys window` shows
    /// the keyguard for its first `locked` calls, logcat holds `trace`,
    /// and every call is appended to `calls`, one line each.
    fn stand_in(locked: u32, trace: &str) -> (tempfile::TempDir, Screens) {
        stand_in_focus(locked, trace, APP_FOCUS)
    }

    /// [`stand_in`], its `dumpsys window` printing `focus` too: the
    /// `mCurrentFocus=` and `mFocusedApp=` lines.
    fn stand_in_focus(locked: u32, trace: &str, focus: &str) -> (tempfile::TempDir, Screens) {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().display();
        let script = format!(
            r#"#!/bin/sh
echo "$*" >> {d}/calls
case "$*" in
  *dumpsys*)
    n=$(grep -c dumpsys {d}/calls)
    if [ "$n" -le {locked} ]; then echo "  isKeyguardShowing=true"; else echo "  isKeyguardShowing=false"; fi
    printf '%s\n' "{focus}"
    echo IW_END=0 ;;
  *logcat\ -d*) printf '%s\n' "{trace}" ;;
esac
"#
        );
        let program = dir.path().join("adb");
        std::fs::write(&program, script).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut screens = Screens::new("stand-in");
        screens.program = program.into();
        (dir, screens)
    }

    fn calls(dir: &tempfile::TempDir) -> Vec<String> {
        let calls = std::fs::read_to_string(dir.path().join("calls")).unwrap_or_default();
        calls.lines().map(str::to_owned).collect()
    }

    #[test]
    fn a_timing_line_is_waited_for_and_its_fields_answered() {
        let (_dir, mut s) = stand_in(
            0,
            "ui-trace page=reader\nui-trace reader=end ms=1250 frames=4 at-end=yes form=formatted",
        );
        let said = s.said(0, "reader=end ", Duration::from_millis(50));
        assert_eq!(
            said.as_deref(),
            Ok("ms=1250 frames=4 at-end=yes form=formatted")
        );
        let step = s.timed("reader_end", said);
        assert_eq!(step.get(keys::RESULT), Some(&json!(keys::PASS)), "{step:?}");
        for (k, v) in [
            ("ms", "1250"),
            ("frames", "4"),
            ("at-end", "yes"),
            ("form", "formatted"),
        ] {
            assert_eq!(step.get(k), Some(&json!(v)), "{k} in {step:?}");
        }
        // A line that never comes fails the step, naming what it waited for.
        let said = s.said(0, "reader=drawn ", Duration::from_millis(50));
        let step = s.timed("read_whole", said);
        assert_eq!(step.get(keys::RESULT), Some(&json!(keys::FAIL)), "{step:?}");
        assert!(
            step.get(keys::DETAIL)
                .and_then(Value::as_str)
                .is_some_and(|d| d.contains("reader=drawn")),
            "{step:?}"
        );
    }

    /// `wait_answer` ends on a note, named, on the wanted lines alone,
    /// and fails naming what never came (#608 F3, F6).
    #[test]
    fn an_adds_wait_ends_on_its_note_or_its_results() {
        let wants = ["add=added lines=1", "page=peers-add-trust"];
        let patience = Duration::from_millis(50);
        let (_dir, mut s) = stand_in(
            0,
            "ui-trace focus=add-action\nui-trace note=refused:grammar",
        );
        let why = s.wait_answer(0, &wants, patience).unwrap_err();
        // The note's own answer, not a timeout whose trace tail shows it.
        assert!(why.starts_with("answered note=refused:grammar"), "{why}");
        let (_dir, mut s) = stand_in(
            0,
            "ui-trace add=added lines=1\nui-trace page=peers-add-trust",
        );
        assert_eq!(s.wait_answer(0, &wants, patience), Ok(()));
        // The wait itself fails, not a read after it.
        let (_dir, mut s) = stand_in(0, "ui-trace add=added lines=1");
        let why = s.wait_answer(0, &wants, patience).unwrap_err();
        assert!(
            why.contains("page=peers-add-trust") && why.contains("within"),
            "{why}"
        );
    }

    /// `add_by_scan`'s wait ends on the add page's note, named, before
    /// the scan page or after it, and passes on the scan's results.
    #[test]
    fn a_scans_wait_ends_on_its_note_or_its_results() {
        let patience = Duration::from_millis(50);
        let (_dir, mut s) = stand_in(0, "ui-trace focus=scan-code\nui-trace note=camera-denied");
        let why = s.wait_scanned(0, patience).unwrap_err();
        assert!(why.starts_with("answered note=camera-denied"), "{why}");
        let (_dir, mut s) = stand_in(
            0,
            "ui-trace page=peers-scan\nui-trace note=refused:own-address",
        );
        let why = s.wait_scanned(0, patience).unwrap_err();
        assert!(
            why.starts_with("answered note=refused:own-address"),
            "{why}"
        );
        let (_dir, mut s) = stand_in(
            0,
            "ui-trace page=peers-scan\nui-trace note=none\nui-trace page=peers-add-trust\nui-trace add=added lines=2",
        );
        assert_eq!(s.wait_scanned(0, patience), Ok(()));
        // Any other result is the scan's answer, named, not a timeout
        // (#608 F13): after the trust step or before it.
        for trace in [
            "ui-trace page=peers-scan\nui-trace page=peers-add-trust\nui-trace add=unchanged lines=2",
            "ui-trace page=peers-scan\nui-trace add=checked lines=1\nui-trace page=peers-add-trust",
        ] {
            let (_dir, mut s) = stand_in(0, trace);
            let why = s.wait_scanned(0, Duration::from_secs(2)).unwrap_err();
            assert!(why.starts_with("answered add="), "{why}");
            assert!(!why.contains("within"), "{why}");
        }
    }

    #[test]
    fn no_am_start_runs_behind_a_keyguard() {
        let (dir, mut s) = stand_in(u32::MAX, "ui-trace page=conversations");
        let why = s.front().unwrap_err();
        assert!(why.contains("locked"), "{why}");
        let r = s.bring_back();
        assert_eq!(r.get(keys::RESULT), Some(&json!(keys::FAIL)), "{r:?}");
        let r = s.launch_locked(Duration::from_millis(50));
        assert_eq!(r.get(keys::RESULT), Some(&json!(keys::FAIL)), "{r:?}");
        let calls = calls(&dir);
        assert!(calls.iter().any(|c| dumps(c)), "{calls:?}");
        assert!(!calls.iter().any(|c| c.contains("am start")), "{calls:?}");
    }

    /// `launch` ends on the app's own window taking the keys, and fails
    /// naming a system window in front -- the permission prompt with
    /// what grants it, or any other -- once the start decided (#608 F15).
    #[test]
    fn a_launch_fails_naming_a_system_window_that_takes_the_keys() {
        let trace = "ui-trace page=conversations\nui-trace connect=settled";
        let patience = Duration::from_millis(200);
        let (_dir, mut s) = stand_in(0, trace);
        let r = s.launch(patience);
        assert_eq!(r.get(keys::RESULT), Some(&json!(keys::PASS)), "{r:?}");
        let detail = |r: &Step| {
            r.get(keys::DETAIL)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        };
        let prompt = "  mCurrentFocus=Window{2e297f1 u0 NotificationShade}\n  mFocusedApp=ActivityRecord{145622583 u0 com.android.permissioncontroller/.permission.ui.GrantPermissionsActivity t2046}";
        let (_dir, mut s) = stand_in_focus(0, trace, prompt);
        let r = s.launch(patience);
        assert_eq!(r.get(keys::RESULT), Some(&json!(keys::FAIL)), "{r:?}");
        let why = detail(&r);
        assert!(
            why.contains("NotificationShade")
                && why.contains("GrantPermissionsActivity")
                && why.contains("POST_NOTIFICATIONS"),
            "{why}"
        );
        // Another window: named, with no grant to suggest.
        let shade = "  mCurrentFocus=Window{2e297f1 u0 NotificationShade}\n  mFocusedApp=ActivityRecord{77 u0 org.interweave.human/.MainActivity t9}";
        let (_dir, mut s) = stand_in_focus(0, trace, shade);
        let r = s.launch(patience);
        assert_eq!(r.get(keys::RESULT), Some(&json!(keys::FAIL)), "{r:?}");
        let why = detail(&r);
        assert!(
            why.contains("NotificationShade") && !why.contains("POST_NOTIFICATIONS"),
            "{why}"
        );
    }

    #[test]
    fn a_launch_to_identity_waits_for_the_unlock_before_it_starts() {
        let (dir, mut s) = stand_in(2, "ui-trace identity=w");
        let r = s.launch_to_identity("w", Duration::from_secs(5));
        assert_eq!(r.get(keys::RESULT), Some(&json!(keys::PASS)), "{r:?}");
        let calls = calls(&dir);
        let start = calls
            .iter()
            .position(|c| c.contains("am start"))
            .expect("an am start");
        let dumps = calls[..start].iter().filter(|c| dumps(c)).count();
        assert!(dumps >= 3, "the start after the keyguard went: {calls:?}");
    }

    #[test]
    fn a_note_is_found_by_its_kind_and_nothing_else_is() {
        let lines = |l: &[&str]| l.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(note_in(&lines(&["ui-trace add=waiting"])), None);
        assert_eq!(note_in(&lines(&["ui-trace notice=none"])), None);
        assert_eq!(note_in(&lines(&["ui-trace note=none"])), None);
        assert_eq!(
            note_in(&lines(&[
                "ui-trace note=none",
                "ui-trace add=waiting",
                "ui-trace focus=address-field",
                "ui-trace note=refused:grammar",
            ])),
            Some("refused:grammar")
        );
    }

    /// `add_by_field`'s doc: a note the add page shows ends the wait at
    /// once, named, whatever the wanted lines are doing (#608 F3).
    #[test]
    fn a_note_ends_an_adds_wait_named_and_results_alone_pass_it() {
        let lines = |l: &[&str]| l.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let wants = ["add=added lines=1", "page=peers-add-trust"];
        assert_eq!(
            answered(
                &lines(&["ui-trace focus=add-action", "ui-trace note=refused:grammar"]),
                &wants
            ),
            Ok(Err("answered note=refused:grammar".to_owned())),
            "the note, with every wanted line missing"
        );
        assert_eq!(
            answered(
                &lines(&[
                    "ui-trace add=added lines=1",
                    "ui-trace page=peers-add-trust",
                    "ui-trace note=failed",
                ]),
                &wants
            ),
            Ok(Err("answered note=failed".to_owned())),
            "a note beside the results still fails"
        );
        assert_eq!(
            answered(
                &lines(&[
                    "ui-trace add=waiting",
                    "ui-trace add=unchanged lines=1",
                    "ui-trace page=peers-add-trust",
                ]),
                &wants
            ),
            Ok(Err("answered add=unchanged lines=1".to_owned())),
            "another result ends the wait, named (#608 F13)"
        );
        assert_eq!(
            answered(
                &lines(&["ui-trace add=waiting", "ui-trace add=none"]),
                &wants
            ),
            Err("[\"add=added lines=1\", \"page=peers-add-trust\"]".to_owned()),
            "waiting and none are no result"
        );
        assert_eq!(
            answered(
                &lines(&[
                    "ui-trace add=added lines=1",
                    "ui-trace page=peers-add-trust"
                ]),
                &wants
            ),
            Ok(Ok(()))
        );
        assert_eq!(
            answered(
                &lines(&["ui-trace add=added lines=1", "ui-trace note=none"]),
                &wants
            ),
            Err("[\"page=peers-add-trust\"]".to_owned()),
            "still waiting, naming what is missing"
        );
    }

    #[test]
    fn a_step_is_the_cases_shape() {
        let s = step("trust", false, "why".to_owned());
        assert_eq!(s.get(keys::CASE), Some(&json!("trust")));
        assert_eq!(s.get(keys::RESULT), Some(&json!(keys::FAIL)));
        assert_eq!(s.get(keys::DETAIL), Some(&json!("why")));
    }
}
