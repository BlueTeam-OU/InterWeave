// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Activity's side: the views on the platform's `NativeActivity`,
//! attached to the network service's session through the hub. The
//! session is the Service's: an Activity destroyed or rotated detaches,
//! and the next one attaches and lists the store again.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use interweave_human_android_platform::start_rules::{Launch, StartRules};
use interweave_human_android_platform::{ServiceHost, ViewLink, ViewSide, onboarding};
use interweave_human_app_core::{Failure, ModelSide, Opener, Problem, Surface, Update};
use interweave_human_ui_model::{BOUND_MS, RuntimeHost, UiModel, ViewEvent, WindowEvent};
use interweave_human_ui_slint::{
    ActivityEvent, AndroidApp, PhotoPick, PhotoRequest, ShowOpen, TouchSettings, View, after,
    defer, init_android, invoke_on_window,
};

use crate::jni::{PROFILE, log};

/// The view, shared by every model side the Activity makes: one per
/// attach and one per change of the service's running state
/// (android-platform's `ViewSide`), since a service that starts again
/// starts its store from new.
/// The flag: a fresh model reads it, and has not yet been told what the
/// view reported to the last one.
struct AndroidSurface(Rc<RefCell<View>>, bool);

impl Surface for AndroidSurface {
    fn render(&mut self, model: &UiModel) {
        self.0.borrow_mut().render(model);
    }

    fn take_events(&mut self, model: &UiModel) -> Vec<ViewEvent> {
        self.0.borrow_mut().take_events(model)
    }

    // The open conversation's window, the avatars in view and the
    // Contacts list's Try again, as on the desktop: without them a
    // windowed start opens no conversation and reads no page.
    fn take_window(&mut self) -> Vec<WindowEvent> {
        let mut view = self.0.borrow_mut();
        if std::mem::take(&mut self.1) {
            view.forget_window_reports();
        }
        view.take_window_events()
    }

    // The view outlives each model the Activity makes over it: the open
    // conversation and the avatars in view are reported to the new one,
    // or it pages nothing and drops the conversation's arrivals. Marked
    // here and forgotten at the next take, which holds the view already:
    // a model is made wherever a message is applied, and the view is not
    // borrowed for it.
    fn model_made(&mut self) {
        self.1 = true;
    }
}

/// The person's ask for network access, raised on the window's thread
/// and taken by the Activity's Kotlin thread (`Native.waitNetworkAsk`),
/// which shows the platform's prompt or the app's settings. Asks made
/// before one is taken count as one.
pub(crate) struct Ask {
    asked: Mutex<bool>,
    changed: Condvar,
}

impl Ask {
    const fn new() -> Self {
        Self {
            asked: Mutex::new(false),
            changed: Condvar::new(),
        }
    }

    fn raise(&self) {
        *self.asked.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.changed.notify_all();
    }

    /// Wait at most `timeout` for an ask, and take it: true when there
    /// was one.
    pub(crate) fn wait(&self, timeout: Duration) -> bool {
        let asked = self.asked.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut asked, _) = self
            .changed
            .wait_timeout_while(asked, timeout, |asked| !*asked)
            .unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut *asked)
    }
}

pub(crate) static NETWORK_ASK: Ask = Ask::new();

/// A swipe armed: the Activity plays the platform's haptic tick, on its
/// own thread's wait (`Native.waitHaptic`), so nothing here calls into
/// the JVM.
pub(crate) static HAPTIC: Ask = Ask::new();

/// Back reached a screen with no way back: the Activity finishes, as the
/// platform would (`Native.waitLeave`).
pub(crate) static LEAVE: Ask = Ask::new();

thread_local! {
    // The window, for Back, whether the chats or the identity screens
    // show.
    static VIEW: RefCell<Option<Rc<RefCell<View>>>> = const { RefCell::new(None) };
    // Whether Show my address is open on this window, read without
    // borrowing the view, which may be borrowed when a frame's off runs.
    static SHOW_OPEN: RefCell<Option<ShowOpen>> = const { RefCell::new(None) };
}

/// Whether Show my address is open on this Activity's window, its
/// addresses perhaps drawn: `onboard::secure_off` then leaves
/// `FLAG_SECURE` on. Android-only wiring with no automated test of its
/// own: `ShowOpen` is ui-slint's (`the_open_state_follows_openings_and_closings`),
/// the decision `ready_check::secure_may_clear`'s, and no device case
/// ends a K1 over an open Show, which needs a restore (#619 F1).
pub(crate) fn show_open() -> bool {
    SHOW_OPEN.with(|slot| slot.borrow().as_ref().is_some_and(ShowOpen::get))
}

/// `Native.back`: the platform's Back, given to the window; when no screen
/// takes it, the Activity is asked to finish.
pub(crate) fn back() {
    invoke_on_window(|| {
        let handled = VIEW.with(|view| {
            view.borrow()
                .as_ref()
                .and_then(|view| view.try_borrow().ok().map(|view| view.back()))
        });
        match handled {
            Some(true) => defer(turn),
            Some(false) => LEAVE.raise(),
            // The view is busy: try again once it is free.
            None => defer(back),
        }
    });
}

/// Whether the phone has a camera (`Native.cameraAvailable`), read before
/// the window exists.
static CAMERA_PRESENT: AtomicBool = AtomicBool::new(false);

/// `Native.cameraAvailable`: Scan a code is offered only with a camera.
pub(crate) fn camera_available(present: bool) {
    CAMERA_PRESENT.store(present, Ordering::SeqCst);
    with_view(move |view| view.offer_scans(present, false, false));
}

/// The time the viewfinder last drew a frame: frames are drawn at most
/// every [`PREVIEW_EVERY`], and every one is read for a code.
static PREVIEWED: Mutex<Option<Instant>> = Mutex::new(None);
const PREVIEW_EVERY: Duration = Duration::from_millis(100);

/// `Native.cameraFrame`, on the camera's own thread: the frame is read for
/// a code there, off the window's thread, and drawn there at most every
/// [`PREVIEW_EVERY`].
pub(crate) fn camera_frame(width: usize, height: usize, stride: usize, plane: &[u8]) {
    let Some(len) = width.checked_mul(height) else {
        return;
    };
    if width == 0 || height == 0 || stride < width {
        return;
    }
    let Some(needed) = stride
        .checked_mul(height - 1)
        .and_then(|n| n.checked_add(width))
    else {
        return;
    };
    if plane.len() < needed {
        return;
    }
    let mut luma = Vec::with_capacity(len);
    for row in 0..height {
        let start = row * stride;
        luma.extend_from_slice(&plane[start..start + width]);
    }
    let text = match interweave_human_ui_slint::code::read_luma(width, height, &luma) {
        interweave_human_ui_slint::code::CodeRead::Text(text) => Some(text),
        interweave_human_ui_slint::code::CodeRead::NoCode => None,
    };
    let draw = {
        let mut last = PREVIEWED.lock().unwrap_or_else(PoisonError::into_inner);
        let now = Instant::now();
        let due = last.is_none_or(|t| now.duration_since(t) >= PREVIEW_EVERY);
        if due {
            *last = Some(now);
        }
        due
    };
    if !draw && text.is_none() {
        return;
    }
    // Assumed, not read: a back camera whose sensor sits a quarter turn
    // clockwise from the phone's portrait, as the Pixel 9a's does, so its
    // landscape frames are turned for the drawing. A sensor mounted the
    // other way, or a front camera, draws upside down or mirrored until
    // the sensor's orientation is passed with the frame (#418 F4).
    // Reading needs no turn.
    let (luma_drawn, dw, dh) = if draw && width > height {
        (
            interweave_human_ui_slint::code::turned_clockwise(&luma, width, height),
            height,
            width,
        )
    } else {
        (luma, width, height)
    };
    let (w, h) = (
        u32::try_from(dw).unwrap_or(0),
        u32::try_from(dh).unwrap_or(0),
    );
    with_view(move |view| {
        if draw {
            view.camera_frame(w, h, &luma_drawn);
        }
        if let Some(text) = &text {
            view.camera_read(text);
        }
    });
}

/// `Native.cameraDenied`: the CAMERA permission refused at Scan a code.
pub(crate) fn camera_denied(for_good: bool) {
    with_view(move |view| view.camera_denied(for_good));
}

/// `Native.photoPicked`: what the photo picker came to for `request` --
/// the image's bytes, empty when they could not be read, or `None` when
/// nothing was chosen. The view takes it only as the answer to its latest
/// request.
pub(crate) fn photo_picked(request: u64, chosen: Option<Vec<u8>>) {
    // Handed over once: `with_view` may run its task again on a retry.
    let chosen = Mutex::new(Some(chosen));
    with_view(move |view| {
        if let Some(chosen) = chosen.lock().ok().and_then(|mut c| c.take()) {
            view.photo_picked(PhotoRequest(request), chosen);
        }
    });
}

/// `Native.photoRead`: the chosen file for `request`, read within what
/// the open left of the pick's deadline (`left`) unless a newer pick, the
/// editor's close or the Activity's end gives it up first (j68 P3-1;
/// #483 F4: one deadline for the open and the read).
pub(crate) fn photo_read(request: u64, fd: std::os::fd::OwnedFd, left: Duration) {
    let deadline = Instant::now() + left.min(crate::photo_import::PHOTO_DEADLINE);
    match PHOTO_IMPORTS.read(request, fd, deadline) {
        crate::photo_import::Read::Bytes(bytes) => photo_picked(request, Some(bytes)),
        crate::photo_import::Read::Unusable => photo_picked(request, Some(Vec::new())),
        crate::photo_import::Read::NotWanted => {}
    }
}

/// The own card editor's picks: the latest the only one read.
static PHOTO_IMPORTS: std::sync::LazyLock<crate::photo_import::Imports> =
    std::sync::LazyLock::new(crate::photo_import::Imports::new);

/// `Native.cameraClosed`: the camera stopped under the scan.
pub(crate) fn camera_closed() {
    with_view(|view| view.camera_closed());
}

/// A credential prompt was answered: the trace says where focus stands
/// once the answer has settled, after what the answer asked of the window
/// (ui-slint's trace.rs).
#[cfg(feature = "ui-trace")]
pub(crate) fn trace_focus_settled() {
    with_view(|view| view.trace_focus_settled());
}

/// The locked notice is to take focus once it shows on Chats or a
/// conversation (unlock note §11, F11 and F12): asked of the window that
/// shows, or, with none yet -- a recreation's starts before its window --
/// of the next one made.
pub(crate) fn focus_locked_notice() {
    NOTICE_FOCUS.store(true, Ordering::SeqCst);
    invoke_on_window(apply_notice_focus);
}

static NOTICE_FOCUS: AtomicBool = AtomicBool::new(false);

/// K1 over the window that shows, for a start the Service made that
/// answered `RecoveryRequired` (onboard.rs, `recovery_refused`); with no
/// window, nothing here: the next window's check answers recovery for the
/// K1 kept, and that window opens it (`ready_state`).
pub(crate) fn raise_recovery() {
    let Some(view) = VIEW.with(|view| view.borrow().clone()) else {
        return;
    };
    if view.try_borrow_mut().is_err() {
        defer(raise_recovery);
        return;
    }
    crate::onboard::raise_recovery(&view);
}

fn apply_notice_focus() {
    let applied = VIEW.with(|view| {
        view.borrow().as_ref().map(|view| {
            view.try_borrow().map(|view| {
                if NOTICE_FOCUS.swap(false, Ordering::SeqCst) {
                    view.focus_locked_notice();
                }
            })
        })
    });
    match applied {
        Some(Ok(())) => defer(turn),
        Some(Err(_)) => defer(apply_notice_focus),
        None => {}
    }
}

/// `Native.settingsReturned`: back from the app's system settings.
pub(crate) fn settings_returned() {
    with_view(|view| view.settings_returned());
}

/// Run `task` on the window's view at its next turn, then a turn of the
/// root; tried again while the view is busy, dropped with no window.
fn with_view(task: impl Fn(&mut View) + Send + Sync + 'static) {
    let task: Arc<dyn Fn(&mut View) + Send + Sync> = Arc::new(task);
    invoke_on_window(move || run_on_view(&task));
}

fn run_on_view(task: &Arc<dyn Fn(&mut View) + Send + Sync>) {
    let applied = VIEW.with(|view| {
        view.borrow().as_ref().map(|view| {
            view.try_borrow_mut()
                .map(|mut view| task(&mut view))
                .is_ok()
        })
    });
    match applied {
        Some(true) => defer(turn),
        Some(false) => {
            let task = Arc::clone(task);
            defer(move || run_on_view(&task));
        }
        None => {}
    }
}

/// `Native.showSecured`: the Activity set `FLAG_SECURE` as Show asked, so
/// the window may draw its addresses.
pub(crate) fn show_secured() {
    invoke_on_window(|| {
        let applied = VIEW.with(|view| {
            view.borrow().as_ref().and_then(|view| {
                view.try_borrow_mut()
                    .ok()
                    .map(|mut view| view.show_secured())
            })
        });
        match applied {
            Some(()) => defer(turn),
            // The view is busy: try again once it is free.
            None => defer(show_secured),
        }
    });
}

/// The platform's touch values as the Activity last read them
/// (`Native.touch`): kept for a window that starts after they were read.
static TOUCH: Mutex<Option<TouchSettings>> = Mutex::new(None);

/// The Activity read its touch values again -- at its start, at a resume,
/// at a layout change: the window takes them.
pub(crate) fn touch_changed(settings: TouchSettings) {
    *TOUCH.lock().unwrap_or_else(PoisonError::into_inner) = Some(settings);
    invoke_on_window(apply_touch);
}

fn apply_touch() {
    let settings = *TOUCH.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(settings) = settings else {
        return;
    };
    let applied = ROOT.with(|root| {
        let Ok(root) = root.try_borrow() else {
            return false;
        };
        if let Some(root) = root.as_ref() {
            root.view.borrow().set_touch_settings(settings);
        }
        true
    });
    if !applied {
        defer(apply_touch);
    }
}

/// The app left the foreground: the snackbar closes (swipe spec §5).
fn stopped() {
    let applied = ROOT.with(|root| {
        let Ok(root) = root.try_borrow() else {
            return false;
        };
        if let Some(root) = root.as_ref() {
            let view = root.view.borrow();
            view.close_snackbar();
            // Never left on screen unasked: the background, or the
            // screen locked (add-a-peer §5.4). The camera stops with the
            // Activity, so its viewfinder closes too.
            view.leave_show();
            view.camera_closed();
        }
        true
    });
    if applied {
        defer(turn);
    } else {
        defer(stopped);
    }
}

/// What the window opens outside itself: links not yet -- the platform's
/// intent is a later step, and a link pressed now is logged, never
/// followed -- and the ask for network access, which the Activity's
/// Kotlin side carries out.
struct AndroidOpener;

impl Opener for AndroidOpener {
    fn open(&mut self, _destination: &str) {
        log("a link was pressed: opening links is not built yet on Android");
    }

    fn ask_network_access(&mut self) {
        NETWORK_ASK.raise();
    }

    fn authenticate(&mut self) {
        crate::onboard::authenticate();
    }

    fn connect_settled(&mut self) {
        if let Some((dir, _)) = crate::onboard::platform()
            && let Err(why) = onboarding::set_connect_owed(&dir, PROFILE, false)
        {
            // Left set, Connect is decided again at the next start, and
            // opens only if the device still knows no one.
            log(&format!("clearing the connect-owed marker: {why:?}"));
        }
    }

    fn unlock_line_seen(&mut self, start: u64) {
        ServiceHost::global().hub().unlock_line_seen(start);
    }
}

struct Root {
    view: Rc<RefCell<View>>,
    side: ViewSide<AndroidSurface, AndroidOpener>,
    link: ViewLink,
    /// The rules of a start with Connect owed (add-a-peer section 5.1;
    /// android-platform's `start_rules` says them and tests them).
    rules: StartRules,
    /// The root's clock for those rules.
    epoch: Instant,
}

impl Root {
    fn attach(view: Rc<RefCell<View>>) -> Self {
        let link = ServiceHost::global().hub().attach(Arc::new(|| {
            invoke_on_window(turn);
        }));
        let surface = Rc::clone(&view);
        // Chats shows at this launch from here: what is opened from it
        // counts against an owed Connect (add-a-peer §5.1). A reattach
        // keeps the rules it had, and with them where the count began.
        let mut rules = StartRules::new();
        rules.count_from(view.borrow().opened_from_chats());
        Self {
            // Each fresh model -- a service restart makes one -- asks the
            // marker again, so a Connect a start left undecided is owed to
            // the next (add-a-peer §5.1).
            side: ViewSide::new(move || {
                let mut side =
                    ModelSide::new(AndroidSurface(Rc::clone(&surface), false), AndroidOpener);
                if crate::onboard::platform()
                    .is_some_and(|(dir, _)| onboarding::connect_owed(&dir, PROFILE))
                {
                    side.connect_owed();
                }
                side
            }),
            view,
            link,
            rules,
            epoch: Instant::now(),
        }
    }

    /// A later start was asked for: what is opened from Chats counts
    /// against its owed Connect from here, not from the launch (§5.1).
    fn start_asked(&mut self) {
        let opened = self.view.borrow().opened_from_chats();
        self.rules.count_from(opened);
    }

    /// A held C4's or R4's start, decided or past its bound: the screen
    /// goes (§5.1). Asked after the turn, whose reads may have decided it.
    fn held_rule(&mut self) {
        let Some(asked_at) = crate::onboard::held_start() else {
            return;
        };
        if crate::onboard::hold_bound_passed() {
            // Deferred now, so the trace says it for this start; a service
            // that has not answered makes its own model when it does, and
            // that one is deferred at once.
            self.side.side_mut().connect_defer();
            let refused = !self.side.running() && self.side.start_decided(asked_at);
            self.rules.bound_passed(asked_at, refused);
            crate::onboard::start_decided();
        } else if self.side.start_decided(asked_at) {
            crate::onboard::start_decided();
        }
    }

    /// One turn: what the hub sent, applied; the person's commands, to
    /// the facade.
    fn pump(&mut self) {
        let hub = ServiceHost::global().hub();
        for message in self.link.take() {
            // A change of the service's running state starts a fresh model
            // either way (android-platform's ViewSide says why).
            if self.side.apply(message).is_err() {
                log("the message store could not be listed");
            }
        }
        // The last start's answer: the store's screen in place of the
        // chats, gone once a start opened the store or the restart after a
        // discard was refused elsewhere (ADR-0060; store::follow).
        crate::store::follow(&self.view, self.side.store_answer());
        if self.link.lost() {
            // Detached by the hub: attach again, and start from a listing.
            // The rules go with the view: a deferral they owe is the next
            // model's, which the fresh attach makes.
            let view = Rc::clone(&self.view);
            let mut rules = std::mem::take(&mut self.rules);
            rules.reattached();
            *self = Self::attach(view);
            self.rules = rules;
            defer(turn);
            return;
        }
        // The rules of a start with Connect owed, then the model's turn,
        // in that order (start_rules::StartRules::turn says why). An
        // opening the turn has yet to take is counted already, so the
        // reads it settles open nothing over it.
        let opened = self.view.borrow().opened_from_chats();
        let now = u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX);
        let held = crate::onboard::held_start().is_some();
        let turned = self.rules.turn(&mut self.side, held, opened, now);
        if turned.started {
            #[cfg(feature = "ui-trace")]
            log(&interweave_human_ui_slint::start_line(
                interweave_human_ui_slint::TracedStart::Running,
            ));
        }
        if turned.launch == Launch::Armed {
            after(Duration::from_millis(BOUND_MS), turn);
        }
        for command in turned.commands {
            if let Err(command) = hub.command(command) {
                // Failed explicitly, so the model lets go of it and says so,
                // rather than holding it in flight with no answer (owner's
                // finding P2-2).
                log("a command found no running network service");
                self.side.side_mut().apply(Update::Failed {
                    command,
                    why: Failure::ServiceStopped,
                });
            }
        }
        for problem in self.side.side_mut().take_problems() {
            match problem {
                Problem::Command { why, .. } => log(&format!("a command failed: {why:?}")),
                Problem::UnreadNotListed(why) => {
                    log(&format!("unread messages could not be read again: {why:?}"));
                }
                Problem::CardsNotRead(why) => {
                    log(&format!("cards or contacts could not be read: {why:?}"));
                }
                Problem::ShareNotRecorded(why) => {
                    log(&format!(
                        "a card was accepted and its share could not be recorded yet: {why:?}"
                    ));
                }
            }
        }
        self.held_rule();
    }
}

thread_local! {
    // The window side, on the Activity's thread only; the facade thread
    // reaches it through `invoke_on_window`, which runs on this thread.
    static ROOT: RefCell<Option<Root>> = const { RefCell::new(None) };
}

/// A later start was asked for (`onboard::person_asked`), from any
/// thread: on the window's, once no turn holds the root, what is opened
/// from Chats counts from there. The asks come from a store screen, an
/// identity screen or a system prompt, each over Chats, so nothing can be
/// opened from Chats between the ask and this. No root yet: the attach
/// that makes one starts the count itself.
pub(crate) fn start_asked() {
    fn now() {
        let applied = ROOT.with(|root| {
            let Ok(mut root) = root.try_borrow_mut() else {
                return false;
            };
            if let Some(root) = root.as_mut() {
                root.start_asked();
            }
            true
        });
        if !applied {
            defer(now);
        }
    }
    invoke_on_window(now);
}

/// A turn of the window side, soon: a held screen's bound passed.
pub(crate) fn turn_soon() {
    defer(turn);
}

/// One turn of the window side, if it is there and not already turning.
fn turn() {
    ROOT.with(|root| {
        if let Ok(mut root) = root.try_borrow_mut() {
            if let Some(root) = root.as_mut() {
                root.pump();
            }
        } else {
            defer(turn);
        }
    });
}

/// The window gained or lost the person's focus: reads follow it, and
/// so do the notices (a focused window reads what arrives).
fn focus(focused: bool) {
    ServiceHost::global().hub().set_focused(focused);
    let applied = ROOT.with(|root| {
        let Ok(root) = root.try_borrow() else {
            return false;
        };
        if let Some(root) = root.as_ref() {
            root.view.borrow_mut().set_window_focused(focused);
        }
        true
    });
    if applied {
        defer(turn);
    } else {
        defer(move || focus(focused));
    }
}

/// Called by the `NativeActivity` glue on a thread of its own, once per
/// Activity instance.
#[allow(
    unsafe_code,
    clippy::no_mangle_with_rust_abi,
    reason = "android-activity's glue calls android_main by its unmangled name with the Rust ABI"
)]
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    crate::jni::log_panics();
    log("android_main: the Activity's views start");
    if let Err(e) = init_android(app, |event| match event {
        ActivityEvent::Focus(focused) => focus(focused),
        ActivityEvent::Stop => stopped(),
        ActivityEvent::Destroy => {
            // The window and its editor go: a read for either is not
            // wanted (the Activity cancels its open itself).
            PHOTO_IMPORTS.abandon();
            ServiceHost::global().hub().set_focused(false);
        }
    }) {
        log(&format!("the views' platform could not be set: {e}"));
        return;
    }
    let mut view = match View::new(RuntimeHost::Embedded) {
        Ok(view) => view,
        Err(e) => {
            log(&format!("no window: {e}"));
            return;
        }
    };
    // From Android 13 (API 33) the platform confirms a copy itself, and a
    // snackbar of the app's would say it twice (#304 F11).
    if AndroidApp::sdk_version() >= 33 {
        view.set_copy_feedback(interweave_human_ui_slint::CopyFeedback::Platform);
    }
    // The debug build's trace, for the screen-driving tests: kinds only,
    // never text, an address or a PeerId (ui-slint's trace.rs).
    #[cfg(feature = "ui-trace")]
    view.on_trace(log);
    let handle = view.handle();
    view.set_wake(|| defer(turn));
    view.on_haptic(|| HAPTIC.raise());
    // Show my address draws this device's addresses only in a secured
    // window, as the phrase pages do (add-a-peer §5.4).
    let show_open = view.guard_show(|on| {
        use crate::onboard::{ASKS, ask};
        if on {
            ASKS.push(ask::SHOW_SECURE);
        } else {
            crate::onboard::secure_off();
        }
    });
    SHOW_OPEN.with(|slot| *slot.borrow_mut() = Some(show_open));
    // Scan a code: the Activity's camera, on while the viewfinder is open
    // (add-a-peer §5.5), offered only on a phone that has one.
    view.camera_hook(|on| {
        use crate::onboard::{ASKS, ask};
        ASKS.push(if on {
            ask::CAMERA_START
        } else {
            ask::CAMERA_STOP
        });
    });
    view.on_open_settings(|| {
        use crate::onboard::{ASKS, ask};
        ASKS.push(ask::OPEN_SETTINGS);
    });
    // The own card's photo: Android's photo picker, which needs no
    // permission (contact-cards design section 3).
    view.photo_picker_hook(|pick| {
        use crate::onboard::{ASKS, ask};
        match pick {
            PhotoPick::Open(PhotoRequest(request)) => {
                PHOTO_IMPORTS.begin(request);
                ASKS.push_with(ask::PICK_PHOTO, request);
            }
            PhotoPick::Abandon => {
                PHOTO_IMPORTS.abandon();
                ASKS.push(ask::PHOTO_CANCEL);
            }
        }
    });
    view.offer_scans(CAMERA_PRESENT.load(Ordering::SeqCst), false, false);
    let view = Rc::new(RefCell::new(view));
    VIEW.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&view)));
    // No identity yet, or one the phone lost: the identity screens first,
    // and the chats attach once the flow hands over (onboard.rs).
    let attach = {
        let view = Rc::clone(&view);
        move || {
            // Weak: the window holds the callback, and a strong view in it
            // would keep the window alive past the Activity.
            let weak = Rc::downgrade(&view);
            view.borrow().offer_show_phrase(move || {
                let weak = weak.clone();
                defer(move || {
                    if let Some(view) = weak.upgrade() {
                        crate::onboard::show_phrase(&view);
                    }
                });
            });
            let view = Rc::clone(&view);
            ROOT.with(|root| *root.borrow_mut() = Some(Root::attach(view)));
            defer(turn);
        }
    };
    if !crate::onboard::start(&view, attach.clone()) {
        attach();
    }
    // Values the Activity read before this window existed.
    defer(apply_touch);
    defer(apply_notice_focus);
    defer(turn);
    if let Err(e) = handle.run() {
        log(&format!("the event loop ended: {e}"));
    }
    // The Activity is gone: its link goes with it, and the hub detaches
    // it on its next send. The session stays the Service's.
    ROOT.with(|root| root.borrow_mut().take());
    VIEW.with(|slot| slot.borrow_mut().take());
    SHOW_OPEN.with(|slot| slot.borrow_mut().take());
    log("android_main: the Activity's views end");
}
