// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identity screens on the Activity (plan section 20 steps 7-8,
//! design/2026-10-19-android-identity.md): before the network service is
//! bound, a profile with no identity, or one whose key the phone lost,
//! shows the flow instead of the chats. The flow is ui-model's; this
//! carries out what it asks -- through android-platform's onboarding with
//! the Keystore cipher -- and asks the Activity's Kotlin side, which
//! waits on [`ASKS`], for what only it can do: the window's secure flag,
//! the credential prompt, binding the service, finishing.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use interweave_human_android_platform::KeyUnlockPolicy;
use interweave_human_android_platform::custody::{self, RecordName, RecoveryCause};
use interweave_human_android_platform::onboarding::{self, Creation, WriteRefused, english};
use interweave_human_android_platform::{Expected, IdentityRefused};
use interweave_human_ui_model::{
    Admit, Effect, IdentityFlow, PresenceGate, Prompted, RecoveryReason, ScreenId, Start,
    StepFailed,
};
use interweave_human_ui_slint::{
    IdentityInput, View, apply_identity, defer, identity_page, invoke_on_window,
};
use interweave_profile_config::{ProfileConfig, ProfileLock, ProfilePaths, TrustBoundary};

use crate::jni::{PROFILE, log};
use crate::keystore::KeystoreCipher;

/// Where the window starts, as `Native.identityState` answers it.
pub(crate) mod state {
    use jni::sys::jint;
    /// The identity opens: the chats, and the service bound.
    pub(crate) const READY: jint = 0;
    /// No record: Welcome.
    pub(crate) const FIRST_RUN: jint = 1;
    /// The record does not open: recovery required.
    pub(crate) const RECOVERY: jint = 2;
    /// The key needs the person, and their last authentication is too
    /// old: the Activity raises the prompt before it binds anything
    /// (android-key-custody.md's failure list, 80594810).
    pub(crate) const AUTHENTICATION_NEEDED: jint = 3;
}

/// What the window asks of the Activity's Kotlin side
/// (`Native.waitIdentityAsk`).
pub(crate) mod ask {
    use jni::sys::jint;
    /// The flow is over and a frame without its page was drawn:
    /// `FLAG_SECURE` off. The Kotlin side sets it, synchronously, before
    /// any word can be drawn: at a start whose identity is not ready, and
    /// before Show my phrase takes the service offline (#309 F3).
    pub(crate) const SECURE_OFF: jint = 2;
    /// The device-credential or strong-biometric prompt; answered by
    /// `Native.prompted`.
    pub(crate) const PROMPT: jint = 3;
    /// The identity is ready: bind the network service as a launch does.
    pub(crate) const BIND: jint = 4;
    /// Back on a first screen: finish the Activity.
    pub(crate) const FINISH: jint = 5;
    /// Show my phrase: stop the service and keep it stopped; answered by
    /// `Native.offline`.
    pub(crate) const OFFLINE: jint = 6;
    /// Show my phrase is over: start the service again.
    pub(crate) const ONLINE: jint = 7;
    /// The same prompt as [`PROMPT`], for a key operation or a launch the
    /// presence window holds back: its title says it unlocks the app, not
    /// that it shows the phrase. Answered by `Native.prompted`.
    pub(crate) const UNLOCK: jint = 8;
    /// The store's screens: start the network service again -- R's Try
    /// again, and after a confirmed discard, which recreates the store
    /// (ADR-0060 decision 6). Unlike [`ONLINE`], not tied to Show my
    /// phrase's hold.
    pub(crate) const RESTART: jint = 9;
    /// Show my address opened (add-a-peer §5.4): `FLAG_SECURE` on, then
    /// `Native.showSecured`, after which the window draws its addresses;
    /// [`SECURE_OFF`] once a frame without it was drawn.
    pub(crate) const SHOW_SECURE: jint = 10;
}

/// The asks, in order, for one waiting Kotlin thread.
pub(crate) struct Asks {
    queue: Mutex<VecDeque<jni::sys::jint>>,
    changed: Condvar,
}

impl Asks {
    const fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            changed: Condvar::new(),
        }
    }

    pub(crate) fn push(&self, ask: jni::sys::jint) {
        self.queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(ask);
        self.changed.notify_all();
    }

    /// Drop what waits: asks an earlier Activity instance left unread
    /// belong to its window, and a late `SECURE_OFF` would clear the new
    /// window's flag while its flow runs (#309 review, risk).
    fn clear(&self) {
        self.queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    /// The next ask within `timeout`, or 0.
    pub(crate) fn wait(&self, timeout: Duration) -> jni::sys::jint {
        let queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut queue, _) = self
            .changed
            .wait_timeout_while(queue, timeout, |queue| queue.is_empty())
            .unwrap_or_else(PoisonError::into_inner);
        queue.pop_front().unwrap_or(0)
    }
}

pub(crate) static ASKS: Asks = Asks::new();

/// What `Native.identityState` found, and the cipher it made on a thread
/// the JVM called from (a native thread finds no app class), kept for the
/// window's thread.
struct Platform {
    app_data_dir: PathBuf,
    cipher: Arc<KeystoreCipher>,
    start: Option<Start>,
    /// The profile's key needs the person: key operations wait on the
    /// presence window (design section 6, acceptance 10).
    user_presence: bool,
}

static PLATFORM: Mutex<Option<Platform>> = Mutex::new(None);
/// Signalled once `Native.identityState` has answered, by [`answered`]:
/// the window's thread and the Activity's `onStart` run at once, and the
/// window must not choose the chats before it knows (#309 F5).
static PLATFORM_SET: Condvar = Condvar::new();
/// Whether `Native.identityState` has answered in this process, whatever
/// it answered: with no [`PLATFORM`] set it chose no flow, and the window
/// waits no longer (#309 review N5).
static ANSWERED: AtomicBool = AtomicBool::new(false);
/// How long the window waits for `onStart`'s answer before it shows the
/// chats and lets the service report what it finds.
const PLATFORM_WAIT: Duration = Duration::from_secs(10);

/// `Native.identityState`: whether the identity opens, and if not, where
/// the flow starts. Any other refusal is the service's to report, as it
/// does today, so it reads as ready.
pub(crate) fn identity_state(app_data_dir: PathBuf, cipher: KeystoreCipher) -> jni::sys::jint {
    // Called by onStart before its relay thread starts: whatever an
    // earlier instance left in the queue is no longer anyone's.
    ASKS.clear();
    let found = interweave_human_android_platform::start_identity(
        &app_data_dir,
        PROFILE,
        &cipher,
        interweave_human_android_platform::FirstRun::Refuse,
    );
    let mut return_code = state::READY;
    let start = match found {
        Ok(_) => None,
        Err(IdentityRefused::Unprovisioned) => Some(Start::FirstRun {
            void_paper: onboarding::void_paper(&app_data_dir, PROFILE),
        }),
        Err(IdentityRefused::RecoveryRequired(cause)) => {
            let paths = TrustBoundary::new(&app_data_dir)
                .and_then(|boundary| ProfilePaths::resolve_embedded(PROFILE, boundary))
                .ok();
            Some(Start::Recovery {
                reason: match cause {
                    RecoveryCause::KeyInvalidated | RecoveryCause::KeyMissing => {
                        RecoveryReason::Key
                    }
                    _ => RecoveryReason::Record,
                },
                // The PeerId the record's field names, read without the
                // cipher whatever the envelope's state; none readable
                // offers no restore (design section 3.4).
                expected: paths.as_ref().and_then(|resolved| {
                    match custody::named_identity(resolved) {
                        Ok(RecordName::Names(peer)) => Some(peer),
                        _ => None,
                    }
                }),
                // Unresolved paths name no PeerId either, so K1 offers no
                // restore and the line has nowhere to show.
                new_keys: paths
                    .as_ref()
                    .is_some_and(|resolved| !crate::identity_keys::holds(resolved)),
            })
        }
        Err(IdentityRefused::UserNotAuthenticated) => {
            // Nothing runs until the person authenticates: the window
            // says so, offline, and the grant binds the service.
            await_authentication(ask::BIND);
            return_code = state::AUTHENTICATION_NEEDED;
            None
        }
        Err(why) => {
            log(&format!(
                "the identity's state: {why:?}; the service reports it"
            ));
            None
        }
    };
    let code = match &start {
        None => return_code,
        Some(Start::FirstRun { .. }) => state::FIRST_RUN,
        Some(_) => state::RECOVERY,
    };
    // Anything but a refusal for authentication ends the locked state.
    // Cleared after the read, never before it: cleared first, a return
    // still locked would flicker to "will connect" and back, with a dead
    // Unlock between (#313 review 2).
    if code != state::AUTHENTICATION_NEEDED {
        clear_authentication();
    }
    let user_presence = needs_presence(&app_data_dir);
    *PLATFORM.lock().unwrap_or_else(PoisonError::into_inner) = Some(Platform {
        app_data_dir,
        cipher: Arc::new(cipher),
        start,
        user_presence,
    });
    code
}

/// The app's data directory and the Keystore bridge `Native.identityState`
/// made, for the store's screens on the window's thread; `None` before it
/// answered.
pub(crate) fn platform() -> Option<(PathBuf, Arc<KeystoreCipher>)> {
    PLATFORM
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map(|p| (p.app_data_dir.clone(), Arc::clone(&p.cipher)))
}

/// Ask the Activity's Kotlin side for `ask` (an [`ask`] code).
pub(crate) fn ask_activity(ask: jni::sys::jint) {
    ASKS.push(ask);
}

/// `Native.identityState` returned, on any path: wake the window's wait.
/// Set under [`PLATFORM`]'s lock, so the waiter cannot miss it between its
/// check and its wait.
pub(crate) fn answered() {
    let _platform = PLATFORM.lock().unwrap_or_else(PoisonError::into_inner);
    ANSWERED.store(true, Ordering::SeqCst);
    PLATFORM_SET.notify_all();
}

/// Whether the profile's configured unlock policy needs the person. A
/// profile not yet readable counts as not: its first run has no key to
/// open yet, and a seal under user-presence that the Keystore refuses
/// still answers `UserNotAuthenticated`.
fn needs_presence(app_data_dir: &Path) -> bool {
    TrustBoundary::new(app_data_dir)
        .and_then(|boundary| ProfilePaths::resolve_embedded(PROFILE, boundary))
        .ok()
        .and_then(|paths| ProfileConfig::load(&paths).ok())
        .is_some_and(|config| {
            config.runtime.android.key_unlock_policy == KeyUnlockPolicy::UserPresence
        })
}

/// Monotonic milliseconds for the presence window.
fn now_ms() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    u64::try_from(
        START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}

struct Onboarding {
    view: Rc<RefCell<View>>,
    flow: IdentityFlow,
    creation: Option<Creation>,
    app_data_dir: PathBuf,
    cipher: Arc<KeystoreCipher>,
    /// The presence window, and the key operation waiting on its prompt.
    gate: PresenceGate,
    /// Show my phrase, from Settings: the service is bound already, and
    /// is taken offline and back rather than bound (design section 3.5).
    from_settings: bool,
    /// The profile's lock, held from before the first word until the
    /// service is asked to start, and lent to each seal and restore
    /// (android-key-custody.md, Recovery). None where it could not be
    /// taken: the seal then takes its own.
    lock: Option<ProfileLock>,
    /// Show my phrase asked the service to stop: whatever ends the flow
    /// must ask it back (#309 F2).
    offline_asked: bool,
    /// The service was asked to start (BIND or ONLINE).
    service_asked: bool,
    /// Called once the flow hands over to the chats.
    done: Rc<dyn Fn()>,
}

thread_local! {
    static ONBOARDING: RefCell<Option<Onboarding>> = const { RefCell::new(None) };
}

/// Show the flow over `view` if the identity is not ready, calling `done`
/// once it hands over to the chats. False when there is nothing to show.
pub(crate) fn start(view: &Rc<RefCell<View>>, done: impl Fn() + 'static) -> bool {
    let platform = PLATFORM.lock().unwrap_or_else(PoisonError::into_inner);
    let (platform, waited) = PLATFORM_SET
        .wait_timeout_while(platform, PLATFORM_WAIT, |platform| {
            platform.is_none() && !ANSWERED.load(Ordering::SeqCst)
        })
        .unwrap_or_else(PoisonError::into_inner);
    if waited.timed_out() {
        log("the identity's state did not arrive; the chats show");
    }
    let Some(Platform {
        app_data_dir,
        cipher,
        start: Some(start),
        user_presence,
    }) = platform.as_ref()
    else {
        return false;
    };
    // Before the network service is bound, so nothing else holds it.
    // Refused, no word is shown or taken until a Try again takes it.
    let mut flow = IdentityFlow::new(english(), start.clone());
    let lock = match onboarding::hold(app_data_dir, PROFILE) {
        Ok(lock) => Some(lock),
        Err(why) => {
            flow.held(Err(failed(&why)));
            None
        }
    };
    let onboarding = Onboarding {
        view: Rc::clone(view),
        flow,
        creation: None,
        app_data_dir: app_data_dir.clone(),
        cipher: Arc::clone(cipher),
        gate: PresenceGate::new(*user_presence),
        from_settings: false,
        lock,
        offline_asked: false,
        service_asked: false,
        done: Rc::new(done),
    };
    drop(platform);
    begin(view, onboarding);
    true
}

/// Settings' Show recovery phrase: S1 over `view`, the chats staying
/// attached underneath; Done or Back returns to Settings.
pub(crate) fn show_phrase(view: &Rc<RefCell<View>>) {
    if ONBOARDING.with(|slot| slot.borrow().is_some()) {
        return;
    }
    let platform = PLATFORM.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(Platform {
        app_data_dir,
        cipher,
        user_presence,
        ..
    }) = platform.as_ref()
    else {
        log("Show my phrase: no identity state was read");
        return;
    };
    let onboarding = Onboarding {
        view: Rc::clone(view),
        flow: IdentityFlow::new(english(), Start::ShowPhrase),
        creation: None,
        app_data_dir: app_data_dir.clone(),
        cipher: Arc::clone(cipher),
        gate: PresenceGate::new(*user_presence),
        from_settings: true,
        // Taken once the service has stopped (`offline`).
        lock: None,
        offline_asked: false,
        service_asked: false,
        done: Rc::new(|| {}),
    };
    drop(platform);
    begin(view, onboarding);
}

fn begin(view: &Rc<RefCell<View>>, onboarding: Onboarding) {
    view.borrow().on_identity(|input| {
        // Applied after the callback returns: the flow may call back into
        // the view, which is borrowed while its callback runs.
        defer(move || input_later(input));
    });
    ONBOARDING.with(|slot| *slot.borrow_mut() = Some(onboarding));
    render();
}

fn input_later(input: IdentityInput) {
    let effects = ONBOARDING.with(|slot| {
        slot.borrow_mut()
            .as_mut()
            .map(|o| apply_identity(&mut o.flow, input))
            .unwrap_or_default()
    });
    run(effects);
    render();
}

/// What a granted prompt starts once no flow is on screen: the service a
/// launch outside the presence window held back (BIND), or the one a
/// cancelled restart after Show my phrase left stopped (ONLINE). `None`
/// when nothing waits for the person.
static AFTER_PROMPT: Mutex<Option<jni::sys::jint>> = Mutex::new(None);

/// Nothing runs until the person authenticates: the window shows it,
/// offline with the notice whose action raises the prompt, and a grant
/// asks `start` of the Activity.
fn await_authentication(start: jni::sys::jint) {
    *AFTER_PROMPT.lock().unwrap_or_else(PoisonError::into_inner) = Some(start);
    interweave_human_android_platform::ServiceHost::global()
        .hub()
        .set_authentication_needed(true);
}

/// Nothing waits on the person any more: a start by any route ends the
/// locked state, or the window would read "offline because it is
/// locked" while the service runs (#313 review 1).
pub(crate) fn clear_authentication() {
    AFTER_PROMPT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    interweave_human_android_platform::ServiceHost::global()
        .hub()
        .set_authentication_needed(false);
}

/// The notice's Unlock: the prompt again, while something waits on it.
/// Raised by the person only, never on a retry of the app's own.
pub(crate) fn authenticate() {
    if AFTER_PROMPT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_some()
    {
        ASKS.push(ask::UNLOCK);
    }
}

/// A prompt answered with no flow on screen: granted, the held start goes
/// ahead and the notice goes; cancelled, the person stays offline with
/// the notice and its Unlock.
fn authenticated(granted: bool) {
    if !granted {
        return;
    }
    let held = AFTER_PROMPT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    if let Some(start) = held {
        clear_authentication();
        ASKS.push(start);
    }
}

/// `Native.prompted`: the credential prompt's answer.
pub(crate) fn prompted(granted: bool) {
    invoke_on_window(move || {
        if ONBOARDING.with(|slot| slot.borrow().is_none()) {
            authenticated(granted);
            return;
        }
        let effects = ONBOARDING.with(|slot| {
            slot.borrow_mut()
                .as_mut()
                .map(|o| match o.gate.prompted(granted, now_ms()) {
                    // The prompt a key operation waited on.
                    Prompted::Resume(effect) => vec![effect],
                    Prompted::Dropped(
                        Effect::RestoreRecorded(_) | Effect::RestoreConfirmed(..),
                    ) => {
                        o.flow.cancelled_step();
                        Vec::new()
                    }
                    // A cancelled service start ends the flow with nothing
                    // started: the person is offline, the window says why,
                    // and its Unlock raises the prompt again (#309
                    // acceptance 10; the custody doc's failure list).
                    Prompted::Dropped(Effect::StartService) => {
                        o.service_asked = true;
                        await_authentication(if o.from_settings {
                            ask::ONLINE
                        } else {
                            ask::BIND
                        });
                        Vec::new()
                    }
                    // A cancelled seal leaves C4 as it was.
                    Prompted::Dropped(_) => Vec::new(),
                    // S1's own prompt.
                    Prompted::NotHeld => o.flow.prompted(granted),
                })
                .unwrap_or_default()
        });
        run(effects);
        render();
    });
}

/// `Native.offline`: whether the service stopped for Show my phrase. The
/// words are read only once it has and the flow holds the profile's
/// lock: a lock still held elsewhere shows nothing.
pub(crate) fn offline(stopped: bool) {
    invoke_on_window(move || {
        let effects = ONBOARDING.with(|slot| {
            slot.borrow_mut()
                .as_mut()
                .map(|o| {
                    let outcome = if stopped {
                        // A Try again after a failed read keeps the lock
                        // the first attempt took: taking it again in this
                        // process would be refused as Busy.
                        o.lock
                            .take()
                            .map_or_else(|| onboarding::hold(&o.app_data_dir, PROFILE), Ok)
                            .and_then(|lock| {
                                o.lock = Some(lock);
                                onboarding::shown_phrase(
                                    &o.app_data_dir,
                                    PROFILE,
                                    o.cipher.as_ref(),
                                )
                            })
                            .map_err(|why| failed(&why))
                    } else {
                        log("Show my phrase: the service did not stop");
                        Err(StepFailed::Storage)
                    };
                    o.flow.offline(outcome)
                })
                .unwrap_or_default()
        });
        run(effects);
        render();
    });
}

fn failed(refused: &WriteRefused) -> StepFailed {
    log(&format!("an identity write was refused: {refused:?}"));
    match refused {
        WriteRefused::Busy => StepFailed::Busy,
        WriteRefused::AlreadyProvisioned => StepFailed::AlreadyProvisioned,
        _ => StepFailed::Storage,
    }
}

/// Carry out what the flow asked, feeding its answers back; an answer may
/// ask for more.
fn run(mut effects: Vec<Effect>) {
    while !effects.is_empty() {
        let mut next = Vec::new();
        for effect in effects {
            next.extend(one(effect));
        }
        effects = next;
    }
}

fn one(effect: Effect) -> Vec<Effect> {
    ONBOARDING.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(o) = slot.as_mut() else {
            return Vec::new();
        };
        // A key operation outside the presence window waits for the
        // prompt, raised here, by the screen, never by the service.
        let effect = match o.gate.admit(effect, now_ms()) {
            Admit::Run(effect) => effect,
            Admit::Prompt => {
                ASKS.push(ask::UNLOCK);
                return Vec::new();
            }
        };
        match effect {
            Effect::Finish => {
                o.lock = None;
                ASKS.push(ask::FINISH);
                Vec::new()
            }
            Effect::Generate => match Creation::new() {
                Ok((creation, words, peer, check)) => {
                    o.creation = Some(creation);
                    o.flow.created(words, peer, check);
                    Vec::new()
                }
                Err(why) => {
                    log(&format!("no new identity: {why:?}"));
                    o.flow.not_created(StepFailed::Storage);
                    Vec::new()
                }
            },
            Effect::Discard => {
                o.creation = None;
                Vec::new()
            }
            Effect::VoidPaper(set) => {
                if let Err(why) = onboarding::set_void_paper(&o.app_data_dir, PROFILE, set) {
                    log(&format!("the void-paper marker: {why:?}"));
                }
                Vec::new()
            }
            Effect::Seal => {
                let outcome = match o.creation.as_ref() {
                    Some(creation) => onboarding::seal(
                        &o.app_data_dir,
                        PROFILE,
                        o.cipher.as_ref(),
                        creation,
                        o.lock.as_ref(),
                    )
                    .map_err(|why| failed(&why)),
                    None => Err(StepFailed::Storage),
                };
                if outcome.is_ok() {
                    o.creation = None;
                }
                o.flow.sealed(outcome)
            }
            Effect::Check(text) => o.flow.checked(onboarding::check(text)),
            Effect::RestoreRecorded(text) => {
                let outcome = onboarding::restore(
                    &o.app_data_dir,
                    PROFILE,
                    o.cipher.as_ref(),
                    text,
                    Expected::Recorded,
                    o.lock.as_ref(),
                )
                .map_err(|why| failed(&why));
                o.flow.restored(outcome)
            }
            Effect::RestoreConfirmed(text, peer) => {
                let outcome = onboarding::restore(
                    &o.app_data_dir,
                    PROFILE,
                    o.cipher.as_ref(),
                    text,
                    Expected::Confirmed(&peer),
                    o.lock.as_ref(),
                )
                .map_err(|why| failed(&why));
                o.flow.restored(outcome)
            }
            Effect::Hold => {
                match onboarding::hold(&o.app_data_dir, PROFILE) {
                    Ok(lock) => {
                        o.lock = Some(lock);
                        o.flow.held(Ok(()));
                    }
                    Err(why) => o.flow.held(Err(failed(&why))),
                }
                Vec::new()
            }
            Effect::Prompt => {
                ASKS.push(ask::PROMPT);
                Vec::new()
            }
            Effect::GoOffline => {
                o.offline_asked = true;
                ASKS.push(ask::OFFLINE);
                Vec::new()
            }
            Effect::StartService => {
                ask_service(o);
                Vec::new()
            }
            Effect::OpenChats | Effect::OpenSettings => Vec::new(),
        }
    })
}

/// Show the page for the flow's screen, or hand over once it is done.
/// The flow does not end while the presence gate holds an effect, or the
/// prompt's answer would find nothing to resume (#309 F1); its end asks
/// the service back whenever the flow stopped it or a first run awaits it,
/// a cancelled prompt and a failed step included (F2); and `FLAG_SECURE`,
/// set by the Activity before any word, is cleared only once a frame
/// without the flow's page has been drawn (F3).
fn render() {
    let handover = ONBOARDING.with(|slot| {
        let mut slot = slot.borrow_mut();
        let o = slot.as_mut()?;
        if o.flow.screen() != ScreenId::Done {
            o.view.borrow().show_identity(Some(identity_page(&o.flow)));
            return None;
        }
        o.view.borrow().show_identity(None);
        if o.gate.holding() {
            return None;
        }
        if !o.service_asked && (!o.from_settings || o.offline_asked) {
            ask_service(o);
        }
        let cleared = o
            .view
            .borrow_mut()
            .after_next_frame(|| ASKS.push(ask::SECURE_OFF));
        if !cleared {
            log("no frame notifier: FLAG_SECURE stays on for this window");
        }
        Some(Rc::clone(&o.done))
    });
    if let Some(done) = handover {
        ONBOARDING.with(|slot| slot.borrow_mut().take());
        if let Ok(mut platform) = PLATFORM.lock()
            && let Some(platform) = platform.as_mut()
        {
            platform.start = None;
        }
        done();
    }
}

/// Ask the Activity to start the service: bound as a launch does after a
/// first run or a recovery, brought back after Show my phrase. The lock
/// goes first: the service takes it.
fn ask_service(o: &mut Onboarding) {
    o.lock = None;
    o.service_asked = true;
    clear_authentication();
    ASKS.push(if o.from_settings {
        ask::ONLINE
    } else {
        ask::BIND
    });
}
