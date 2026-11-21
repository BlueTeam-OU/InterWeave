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
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use interweave_human_android_platform::custody::{self, RecordName, RecoveryCause};
use interweave_human_android_platform::onboarding::{self, Creation, WriteRefused, english};
use interweave_human_android_platform::{
    Expected, IdentityRefused, ServiceHost, StoreGeneration, locked_status, reseal_needs_presence,
};
use interweave_human_ui_model::{
    Effect, IdentityFlow, PresenceGate, Prompted, RecoveryReason, ScreenId, Start, StartHold,
    StepFailed,
};
use interweave_human_ui_slint::{
    IdentityInput, View, Wake, apply_identity, defer, identity_page, invoke_on_window,
};
use interweave_profile_config::{ProfileLock, ProfilePaths, TrustBoundary};

use crate::asks::Asks;
use crate::jni::{PROFILE, log};
use crate::keystore::KeystoreCipher;
use crate::ready_check::{AtFlowEnd, Before, Over, at_flow_end, secure_may_clear};
use crate::unlock_route::{Route, UnlockRoute, flow_prompts};
#[cfg(feature = "ui-trace")]
use interweave_human_ui_slint::{TracedPrompt, TracedStart, prompt_line, start_line};

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
    /// old -- or the record opens but the start's reseal to a configured
    /// user-presence will need the person (`reseal_needs_presence`), asked
    /// whether or not a window is open: the Activity raises the prompt
    /// before it binds anything (android-key-custody.md's failure list,
    /// 80594810; ADR-0042 A 2026-11-05).
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
    /// The same prompt as [`PROMPT`], for a key operation or a start the
    /// Keystore refused until the person authenticates: its title says it unlocks the app, not
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
    /// K1 raised over a running window ([`super::raise_recovery`]):
    /// `FLAG_SECURE` on before R2 can draw a word; [`SECURE_OFF`] once a
    /// frame without the flow was drawn, as at a launch.
    pub(crate) const SECURE_ON: jint = 16;
    /// Scan a code opened: the camera on, `CAMERA` asked first if the app
    /// does not hold it (add-a-peer §5.5: at the first Scan, nowhere else).
    pub(crate) const CAMERA_START: jint = 11;
    /// The viewfinder closed: the camera off.
    pub(crate) const CAMERA_STOP: jint = 12;
    /// Open settings on the camera's denied-for-good line.
    pub(crate) const OPEN_SETTINGS: jint = 13;
    /// The own card's Add a photo or Change photo: the platform's photo
    /// picker, images only, one item (contact-cards design 5.2); answered
    /// by `Native.photoRead` or `Native.photoPicked`, for the request the
    /// ask carries.
    pub(crate) const PICK_PHOTO: jint = 14;
    /// The editor that asked for a photo closed: an open still waiting on
    /// the provider is cancelled.
    pub(crate) const PHOTO_CANCEL: jint = 15;
    /// An identity screen's Copy `PeerId` (identity note section 5): the
    /// text the ask's mark names (`Native.peerIdClip`) on the clipboard,
    /// the mark in the clip's label (activity.rs, `AndroidClipboard`).
    pub(crate) const CLIP_COPY: jint = 17;
    /// `IdAlreadyCreated` showed after that copy (section 6): the clip
    /// cleared if its description's label still carries the ask's mark,
    /// its content never read; put off by `Native.peerIdRemovalDeferred`
    /// while the window lacks the focus, the device is locked, or the
    /// platform gives no clip description.
    pub(crate) const CLIP_REMOVE: jint = 18;
}

pub(crate) static ASKS: Asks = Asks::new();

/// What `Native.identityState` found, and the cipher it made on a thread
/// the JVM called from (a native thread finds no app class), kept for the
/// window's thread.
struct Platform {
    app_data_dir: PathBuf,
    cipher: Arc<KeystoreCipher>,
    start: Option<Start>,
    /// `start` is a K1 [`recovery_refused`] kept in a process that found
    /// the identity ready: whichever window opens it opens it as over a
    /// running window ([`Over::Running`]), its end restarting the service
    /// whether the binding was kept or not; the check of a window that
    /// comes after its own went answers recovery ([`ready_state`]). A
    /// later start that runs drops it ([`start_answered`]), as does the
    /// end of the flow that shows it; Show my phrase's end keeps it
    /// ([`render`]).
    kept: bool,
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

/// `Native.identityState`: whether the identity opens and its start can
/// run without the person, and if not, the prompt or where the flow
/// starts. Any other refusal is the service's to report, as it does
/// today, so it reads as ready. `ready`: this process found the identity
/// ready before, so only whether a start now needs the person is asked
/// ([`ready_state`]).
pub(crate) fn identity_state(
    app_data_dir: PathBuf,
    cipher: KeystoreCipher,
    ready: bool,
) -> jni::sys::jint {
    if ready {
        return ready_state(&app_data_dir, &cipher);
    }
    // Called by onStart before its relay thread starts: whatever an
    // earlier instance left in the queue is no longer anyone's, but the
    // clipboard's copy and removal, which are the system clipboard's and
    // carry their own mark (Asks::clear). An Unlock a refused start asked
    // since `window_shown` is dropped with the rest, so the check below
    // asks its own.
    if let Some(through) = ASKS.clear(ask::UNLOCK, &[ask::CLIP_COPY, ask::CLIP_REMOVE]) {
        unlock_unasked(through);
    }
    let found = interweave_human_android_platform::start_identity(
        &app_data_dir,
        PROFILE,
        &cipher,
        interweave_human_android_platform::FirstRun::Refuse,
    );
    let mut return_code = state::READY;
    let start = match found {
        // The record opens, but this start's reseal to a configured
        // user-presence needs the person: prompt before the start, as for
        // a key that needs them to open (ADR-0042 A 2026-11-05).
        Ok(_) if reseal_needs_presence(&app_data_dir, PROFILE) => {
            locked_before_start(&app_data_dir);
            await_authentication(ask::BIND);
            let decided = route_state().checked();
            route(decided);
            return_code = state::AUTHENTICATION_NEEDED;
            None
        }
        Ok(_) => None,
        Err(IdentityRefused::Unprovisioned) => Some(Start::FirstRun {
            void_paper: onboarding::void_paper(&app_data_dir, PROFILE),
        }),
        Err(IdentityRefused::RecoveryRequired(cause)) => {
            Some(recovery_start(&app_data_dir, &cause))
        }
        Err(IdentityRefused::UserNotAuthenticated) => {
            // Nothing runs until the person authenticates: the window
            // says so, offline, and the grant binds the service.
            locked_before_start(&app_data_dir);
            await_authentication(ask::BIND);
            let decided = route_state().checked();
            route(decided);
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
    *PLATFORM.lock().unwrap_or_else(PoisonError::into_inner) = Some(Platform {
        app_data_dir,
        cipher: Arc::new(cipher),
        start,
        kept: false,
    });
    code
}

/// K1's start for a record refused for `cause` (design section 3.4).
fn recovery_start(app_data_dir: &Path, cause: &RecoveryCause) -> Start {
    let paths = TrustBoundary::new(app_data_dir)
        .and_then(|boundary| ProfilePaths::resolve_embedded(PROFILE, boundary))
        .ok();
    Start::Recovery {
        reason: match cause {
            RecoveryCause::KeyInvalidated | RecoveryCause::KeyMissing => RecoveryReason::Key,
            _ => RecoveryReason::Record,
        },
        // The PeerId the record's field names, read without the cipher
        // whatever the envelope's state; none readable offers no restore
        // (design section 3.4).
        expected: paths
            .as_ref()
            .and_then(|resolved| match custody::named_identity(resolved) {
                Ok(RecordName::Names(peer)) => Some(peer),
                _ => None,
            }),
        // Unresolved paths name no PeerId either, so K1 offers no restore
        // and the line has nowhere to show.
        new_keys: paths
            .as_ref()
            .is_some_and(|resolved| !crate::identity_keys::holds(resolved)),
    }
}

/// A start the Service made in this process answered `RecoveryRequired`
/// for `cause`: K1 rises over whatever the window shows, Chats, a
/// conversation or Settings, as it opens at a launch (design sections
/// 3.4 and 8). Kept for the window, which raises it at its next turn
/// ([`raise_recovery`]). Before `Native.identityState` answered in this
/// process there is no window to raise it over, and the launch's own
/// check finds the record refused.
pub(crate) fn recovery_refused(app_data_dir: &Path, cause: &RecoveryCause) {
    let start = recovery_start(app_data_dir, cause);
    {
        let mut platform = PLATFORM.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(platform) = platform.as_mut() else {
            return;
        };
        platform.start = Some(start);
        platform.kept = true;
    }
    invoke_on_window(crate::activity::raise_recovery);
}

/// [`identity_state`] once this process found the identity ready: a
/// start the Activity is about to make or retry -- a launch or a return
/// -- needs the person when the record does not open without them or its
/// reseal will need them, and is then answered as at a cold launch (unlock
/// note §11, L2 and L3): the locked view, the prompt or the focused
/// notice, and a grant that starts the service again. A client already
/// running holds its identity and is asked nothing. Any other answer of
/// the record is the service's to report, so it reads as ready -- but a
/// K1 kept for a window, its own window gone, answers recovery, so the
/// Activity sets `FLAG_SECURE` before this window draws it (#611 F1).
fn ready_state(app_data_dir: &Path, cipher: &KeystoreCipher) -> jni::sys::jint {
    let kept = PLATFORM
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .is_some_and(|p| p.kept && matches!(p.start, Some(Start::Recovery { .. })));
    match crate::ready_check::before_reading(
        interweave_human_android_platform::ServiceHost::global().is_running(),
        kept,
    ) {
        Before::Ready => return state::READY,
        Before::Recovery => return state::RECOVERY,
        Before::ReadRecord => {}
    }
    let locked = match interweave_human_android_platform::start_identity(
        app_data_dir,
        PROFILE,
        cipher,
        interweave_human_android_platform::FirstRun::Refuse,
    ) {
        Ok(_) => reseal_needs_presence(app_data_dir, PROFILE),
        Err(why) => why == IdentityRefused::UserNotAuthenticated,
    };
    if !locked {
        return state::READY;
    }
    locked_before_start(app_data_dir);
    // Bound or not, the service is started again: a recreation keeps a
    // binding whose service runs nothing.
    await_authentication(ask::RESTART);
    let decided = route_state().checked();
    route(decided);
    state::AUTHENTICATION_NEEDED
}

/// The process's routing of a locked start ([`UnlockRoute`]).
static ROUTE: Mutex<UnlockRoute> = Mutex::new(UnlockRoute::new());

fn route_state() -> std::sync::MutexGuard<'static, UnlockRoute> {
    ROUTE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Carry out `route`: the Unlock prompt asked of the Activity, or the
/// locked notice focused.
fn route(route: Route) {
    match route {
        Route::Prompt => ask_unlock(),
        Route::Notice => crate::activity::focus_locked_notice(),
        Route::Nothing | Route::Asked => {}
    }
}

/// The Unlock prompt, asked of the Activity, said in the trace whoever
/// asks it -- the person's Unlock, the presence gate, a locked start --
/// so a test reading the trace sees every prompt raised.
fn ask_unlock() {
    #[cfg(feature = "ui-trace")]
    log(&prompt_line(TracedPrompt::Unlock));
    ASKS.push(ask::UNLOCK);
}

/// The Unlock prompt for the effect `o`'s gate now holds, said in the
/// trace as [`ask_unlock`] says it, and numbered as its argument, which
/// Kotlin does not read (`Asks::push_numbered`; every other asker's is 0),
/// so the gate can be told when the ask is dropped ([`unlock_unasked`]).
fn gate_ask(o: &mut Onboarding) {
    route_state().asked();
    #[cfg(feature = "ui-trace")]
    log(&prompt_line(TracedPrompt::Unlock));
    o.gate.asked(ASKS.push_numbered(ask::UNLOCK));
}

/// Unlock asks were dropped before the Activity raised them (a new
/// showing, a new Activity instance), the gate's up to number `through`:
/// nothing will answer them. The route learns no prompt is outstanding,
/// which is not a decline -- the person decided nothing, and L2 counts
/// only an answered prompt's cancel -- and a key operation the gate held
/// for one of them is released on the window's thread as a cancelled
/// prompt releases it ([`not_run`]): no screen stays held by a prompt
/// nobody will raise, and nothing is retried.
fn unlock_unasked(through: u64) {
    route_state().unasked();
    if through == 0 {
        // Only the route's and the notice's asks were dropped: no gate
        // waited on them.
        return;
    }
    invoke_on_window(move || {
        let effects = ONBOARDING.with(|slot| {
            slot.borrow_mut()
                .as_mut()
                .and_then(|o| {
                    let effect = o.gate.unasked(through)?;
                    log("an identity key operation's prompt was dropped unraised: not run");
                    Some(not_run(o, &effect))
                })
                .unwrap_or_default()
        });
        run(effects);
        render();
    });
}

/// `Native.shown`: an Activity's window started (true) or stopped
/// (false) showing. An Unlock prompt asked for an earlier showing is not
/// raised over this one.
pub(crate) fn window_shown(shown: bool) {
    if shown && let Some(through) = ASKS.drop_all(ask::UNLOCK) {
        unlock_unasked(through);
    }
    // The Activity starts the service at each showing (MainActivity's
    // onStart; a recreation's binding restarts a start refused busy):
    // `IdStartBusy`, if it shows, waits on that start with Try again
    // disabled (identity note section 6, A return or a recreation). A
    // launch took the notice away first ([`launched`]).
    if shown {
        ServiceHost::global().hub().start_asked(true);
    }
    route_state().shown(shown);
}

/// `Native.launched`: the Activity was created anew, not recreated in
/// this process -- a launch, which prompts as a cold launch does.
pub(crate) fn launched() {
    // A new launch shows what its own start answers, never `IdStartBusy`
    // (identity note section 6, How long).
    ServiceHost::global().hub().start_asked(false);
    route_state().launched();
}

/// `IdStartBusy`'s Try again on Chats or a conversation, after the bound
/// (identity note section 6): one service start, by the path a granted
/// Unlock uses ([`authenticated`]), as the person's, so a refusal for the
/// screen lock prompts (L1) and Connect counts its openings from here
/// (add-a-peer section 5.1). The hub shows the notice's start running
/// until it answers, and the model offers no second press meanwhile
/// (ui-model's `a_busy_start_is_retried_once_until_it_answers`).
pub(crate) fn retry_start() {
    ServiceHost::global().hub().start_asked(true);
    person_asked();
    ASKS.push(ask::RESTART);
}

/// The person's action asks for the next start: a store screen's Try
/// again or discard, an identity screen's continue, a network grant. A
/// refusal for the person then prompts (L1).
pub(crate) fn person_asked() {
    route_state().person_asked();
    crate::activity::start_asked();
}

/// A start the Service made was refused until the person authenticates
/// (unlock note §11): the locked view with its Unlock, whose grant starts
/// the service again, and the prompt, the focused notice or nothing as
/// [`UnlockRoute::refused`] says. The Service never prompts: the Activity
/// raises it, and only while its window shows.
///
/// A start a held C4 or R4 asked ([`FLOW_START`]) is answered by the
/// flow's prompt over that screen when the route prompts: the screen stays
/// held, a grant starts the service again under it, and a cancel ends on
/// Chats locked ([`not_run`]; design section 6, Authentication needed and
/// Seal running).
pub(crate) fn refused_locked() {
    let flow = FLOW_START.swap(false, Ordering::SeqCst);
    await_authentication(ask::RESTART);
    let decided = route_state().refused();
    if flow_prompts(flow, decided) {
        invoke_on_window(flow_start_refused);
        return;
    }
    route(decided);
    if flow {
        // The held screen waited on this answer: decided now.
        invoke_on_window(crate::activity::turn_soon);
    }
}

/// A held C4's or R4's start was asked and has not answered yet: its
/// refusal for the person, if it comes, is the flow's to prompt for. Set
/// on the window's thread at the ask, taken by the start's answer on the
/// Service's ([`refused_locked`], [`start_answered`]).
static FLOW_START: AtomicBool = AtomicBool::new(false);

/// [`refused_locked`] on the window's thread: the held screen's gate
/// holds the start for the prompt. A screen no longer held -- its bound
/// passed meanwhile -- leaves the prompt to the route, over Chats.
fn flow_start_refused() {
    let held = ONBOARDING.with(|slot| {
        slot.borrow_mut().as_mut().is_some_and(|o| {
            let ours = o.flow.start_held() && o.service_asked && !o.gate.holding();
            if ours {
                prompt_for(o, Effect::StartService);
            }
            ours
        })
    });
    if !held {
        ask_unlock();
    }
    render();
}

/// Whether a held C4's or R4's start is the flow's to answer still: asked
/// and unanswered, or held by the gate for its prompt. The screen is not
/// decided meanwhile, or a refusal for the person would hand over to
/// Chats before the prompt over the screen answers it.
pub(crate) fn flow_answers_refusal() -> bool {
    FLOW_START.load(Ordering::SeqCst)
        || ONBOARDING.with(|slot| {
            slot.try_borrow()
                .is_ok_and(|o| o.as_ref().is_some_and(|o| o.gate.holding_start()))
        })
}

/// A start the Service made answered anything but a refusal for the
/// person, `ran` when it runs: the locked state ends, as at the
/// Activity's check (#313 review 1).
pub(crate) fn start_answered(ran: bool) {
    if FLOW_START.swap(false, Ordering::SeqCst) {
        invoke_on_window(crate::activity::turn_soon);
    }
    route_state().answered(ran);
    clear_authentication();
    // A K1 kept for a window is no longer owed once a start ran: the
    // record opened after all (the identity hook's one refusal spent, or
    // a restore through another window), and no later window opens it.
    if ran
        && let Some(platform) = PLATFORM
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
        && platform.kept
    {
        platform.start = None;
        platform.kept = false;
    }
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

/// Monotonic milliseconds for a held screen's clock (add-a-peer §5.1).
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

impl Over {
    /// The ask that starts the service at the flow's end.
    const fn start_ask(self) -> jni::sys::jint {
        match self {
            Self::Launch => ask::BIND,
            Self::Settings => ask::ONLINE,
            Self::Running => ask::RESTART,
        }
    }
}

struct Onboarding {
    view: Rc<RefCell<View>>,
    flow: IdentityFlow,
    creation: Option<Creation>,
    app_data_dir: PathBuf,
    cipher: Arc<KeystoreCipher>,
    /// The key operation refused for the person, waiting on its prompt.
    gate: PresenceGate,
    /// Where the flow was opened, which says how its end starts the
    /// service.
    over: Over,
    /// The profile's lock, held from before the first word until the
    /// service is asked to start, and lent to each seal and restore
    /// (android-key-custody.md, Recovery). None where it could not be
    /// taken: the seal then takes its own.
    lock: Option<ProfileLock>,
    /// Show my phrase asked the service to stop: whatever ends the flow
    /// must ask it back (#309 F2).
    offline_asked: bool,
    /// The service was asked to start (BIND, ONLINE or RESTART).
    service_asked: bool,
    /// The clock of a C4 or R4 held after Continue (add-a-peer §5.1),
    /// from the press until the start decides.
    hold: Option<StartHold>,
    /// The hub's count of start answers before the service was asked: a
    /// held start decides on answers counted past it only.
    asked_at: StoreGeneration,
    /// `done` was called: the chats are attached under a held C4 or R4,
    /// so the start's reads run while the screen holds.
    handed: bool,
    /// Called once the flow hands over to the chats -- or, under a held
    /// C4 or R4, once the service is asked, so the start's reads run while
    /// the screen holds and the first screen shown is the one they decide.
    done: Rc<dyn Fn()>,
}

impl Onboarding {
    /// The ask that starts the service now. The first is the flow's own
    /// ([`Over::start_ask`]). Any later one -- `IdStartBusy`'s Try again,
    /// a held start resumed after a granted prompt, the Unlock after that
    /// prompt was cancelled -- finds the service bound and started: a bind
    /// would find the binding (MainActivity.connect) and Show my phrase's
    /// way back the service already back (NetworkService.online), and
    /// either would start nothing. A restart starts it again, and binds
    /// when the service is unbound since.
    const fn start_ask(&self) -> jni::sys::jint {
        if self.service_asked {
            ask::RESTART
        } else {
            self.over.start_ask()
        }
    }
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
        kept,
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
    let raised = matches!(start, Start::Recovery { .. });
    // A K1 kept in a running process, its window gone: the service is
    // bound or binds, and its end restarts it, as over a window. The flag
    // is on already, set at the Activity's check; asked again for a
    // window made with no check before it.
    let over = if *kept {
        ASKS.push(ask::SECURE_ON);
        Over::Running
    } else {
        Over::Launch
    };
    let onboarding = Onboarding {
        view: Rc::clone(view),
        flow,
        creation: None,
        app_data_dir: app_data_dir.clone(),
        cipher: Arc::clone(cipher),
        gate: PresenceGate::default(),
        over,
        lock,
        offline_asked: false,
        service_asked: false,
        hold: None,
        asked_at: StoreGeneration::default(),
        handed: false,
        done: Rc::new(done),
    };
    drop(platform);
    begin(view, onboarding);
    // A refused start raised K1 in place of Chats: focus on its heading
    // (section 8).
    if raised {
        view.borrow().raise_identity_heading();
    }
    true
}

/// K1 over `view`, the chats staying attached underneath, for a start
/// [`recovery_refused`] kept; nothing while a flow shows already, whose
/// end raises it or leaves it to the start it asks ([`render`]). Focus
/// goes to its heading (section 8), and `FLAG_SECURE` goes on before R2
/// can draw a word: K1 itself draws none. Its end starts the service
/// again.
pub(crate) fn raise_recovery(view: &Rc<RefCell<View>>) {
    if ONBOARDING.with(|slot| slot.borrow().is_some()) {
        log("K1 kept: an identity flow shows, whose end decides it");
        return;
    }
    let platform = PLATFORM.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(Platform {
        app_data_dir,
        cipher,
        start: Some(start @ Start::Recovery { .. }),
        ..
    }) = platform.as_ref()
    else {
        return;
    };
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
        gate: PresenceGate::default(),
        over: Over::Running,
        lock,
        offline_asked: false,
        service_asked: false,
        hold: None,
        asked_at: StoreGeneration::default(),
        handed: false,
        done: Rc::new(|| {}),
    };
    drop(platform);
    ASKS.push(ask::SECURE_ON);
    begin(view, onboarding);
    view.borrow().raise_identity_heading();
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
        gate: PresenceGate::default(),
        over: Over::Settings,
        // Taken once the service has stopped (`offline`).
        lock: None,
        offline_asked: false,
        service_asked: false,
        hold: None,
        asked_at: StoreGeneration::default(),
        handed: false,
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
            .map(|o| {
                let was_held = o.flow.start_held();
                let effects = apply_identity(&mut o.flow, input);
                // Continue on C4 or R4: the screen holds from the press.
                // The bound counts from it unless a seal must commit the
                // identity first (§5.1, the bound).
                if o.flow.start_held() && !was_held {
                    // C4's Try again under `IdStorageUnknown` commits as
                    // its seal would, once the read-back finds the record.
                    let committed = !effects
                        .iter()
                        .any(|e| matches!(e, Effect::Seal | Effect::ReadBack(_)));
                    o.hold = Some(StartHold::pressed(now_ms(), committed));
                    #[cfg(feature = "ui-trace")]
                    log(&start_line(TracedStart::Pressed));
                }
                effects
            })
            .unwrap_or_default()
    });
    run(effects);
    render();
}

/// What a granted prompt starts once no flow is on screen: the service a
/// launch refused for the person held back (BIND), the one a
/// cancelled restart after Show my phrase left stopped (ONLINE), or a
/// start refused for the screen lock, or found locked at the Activity's
/// check in a running process (RESTART), which the grant runs again
/// once. `None` when nothing waits for the person.
static AFTER_PROMPT: Mutex<Option<jni::sys::jint>> = Mutex::new(None);

/// The unlock status of a locked check, for the locked view: the record
/// header's policy and the last start's pending reason, as a start
/// refused for the person would have left it -- here no start runs before
/// the prompt.
fn locked_before_start(app_data_dir: &Path) {
    let host = interweave_human_android_platform::ServiceHost::global();
    host.locked_before_start(locked_status(app_data_dir, PROFILE, host.unlock_status()));
    host.say_unlock(app_data_dir, PROFILE);
}

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
/// Raised by the person only, never on a retry of the app's own. Raised
/// whether or not a prompt is outstanding: one shown covers the notice, so
/// the press means none shows, and a prompt that never answered must not
/// leave the person without a way to unlock.
pub(crate) fn authenticate() {
    if AFTER_PROMPT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_some()
    {
        route_state().asked();
        ask_unlock();
    }
}

/// A prompt answered with no flow on screen: granted, the held start goes
/// ahead and the notice goes; cancelled, the person stays offline with
/// the notice and its Unlock.
fn authenticated(granted: bool) {
    if !granted {
        // Offline and locked: on Chats or a conversation the notice takes
        // focus (F11, F12); a store screen or Settings keeps its own.
        if AFTER_PROMPT
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
        {
            crate::activity::focus_locked_notice();
        }
        return;
    }
    let held = AFTER_PROMPT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    if let Some(start) = held {
        clear_authentication();
        // The grant runs the refused start again, once, as the person's.
        person_asked();
        ASKS.push(start);
    }
}

/// `Native.prompted`: the credential prompt's answer; `unlock` when it
/// was the Unlock prompt, whose cancel is a decline (L2).
pub(crate) fn prompted(granted: bool, unlock: bool) {
    if unlock {
        route_state().prompted(granted);
    }
    #[cfg(feature = "ui-trace")]
    log(&prompt_line(if granted {
        TracedPrompt::Granted
    } else {
        TracedPrompt::Cancelled
    }));
    invoke_on_window(move || {
        if ONBOARDING.with(|slot| slot.borrow().is_none()) {
            authenticated(granted);
            return;
        }
        let effects = ONBOARDING.with(|slot| {
            slot.borrow_mut()
                .as_mut()
                .map(|o| {
                    if granted && let Some(hold) = o.hold.as_mut() {
                        hold.prompt_granted(now_ms());
                    }
                    match o.gate.prompted(granted) {
                        // The prompt a key operation waited on.
                        Prompted::Resume(effect) => vec![effect],
                        Prompted::Dropped(effect) => not_run(o, &effect),
                        // S1's own prompt.
                        Prompted::NotHeld => o.flow.prompted(granted),
                    }
                })
                .unwrap_or_default()
        });
        run(effects);
        render();
    });
    // Queued after the answer's own work, so it is read after it.
    #[cfg(feature = "ui-trace")]
    crate::activity::trace_focus_settled();
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

/// A key operation the presence gate held did not run: its prompt was
/// cancelled or denied ([`prompted`]), or dropped before anything raised
/// it ([`unlock_unasked`]). One answer for both, so a prompt that ends
/// without running what it held always gives the screen back.
fn not_run(o: &mut Onboarding, effect: &Effect) -> Vec<Effect> {
    match effect {
        // A restore not run gives R2's check back, or R3's Yes and No; a
        // seal leaves C4 as it was, Continue enabled again (§5.1).
        // Nothing was committed, and nothing runs again until the
        // primary action is used again (design section 6). A read-back
        // not run shows `IdStorageUnknown` instead, focused: the write
        // before it may have committed the identity (section 6,
        // Authentication needed). The flow tells them apart.
        Effect::RestoreRecorded(_)
        | Effect::RestoreConfirmed(..)
        | Effect::Seal
        | Effect::ReadBack(_) => {
            o.flow.step_not_run(effect);
            Vec::new()
        }
        // A service start not run ends the flow with nothing started: the
        // person is offline, the window says why, and its Unlock raises
        // the prompt again (#309 acceptance 10; the custody doc's failure
        // list). A dropped ask ends it the same way rather than giving the
        // held C4 or R4 back: the identity is committed, so C4's Continue
        // would seal again, and asking again by itself is the retry
        // section 6 forbids. Whether a return prompts is the route's
        // ([`UnlockRoute`]), which counts no decline for a dropped ask.
        Effect::StartService => {
            let start = o.start_ask();
            o.service_asked = true;
            await_authentication(start);
            // C4's or R4's continue ends on Chats with the notice focused
            // -- the held screen's start is decided, refused -- S2's Done
            // on Settings, which keeps its own (identity note §10 item
            // 10).
            crate::activity::focus_locked_notice();
            o.flow.decided()
        }
        _ => Vec::new(),
    }
}

fn failed(refused: &WriteRefused) -> StepFailed {
    log(&format!("an identity write was refused: {refused:?}"));
    match refused {
        WriteRefused::Busy => StepFailed::Busy,
        WriteRefused::AlreadyProvisioned => StepFailed::AlreadyProvisioned,
        // After the publish, and not read back: `IdStorageUnknown`, never
        // "nothing was changed" (design section 6).
        WriteRefused::ReadBackFailed(_) => StepFailed::StorageUnknown,
        // After the publish too, and the read-back waits on the person:
        // the screen's prompt, then the read-back again.
        WriteRefused::ReadBackNeedsAuthentication => StepFailed::ReadBackNeedsAuthentication,
        WriteRefused::RecordNamesOther(recorded) => StepFailed::RestoreOther(recorded.clone()),
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
        // Every key operation runs at once: the app keeps no clock of the
        // Keystore's window, and a refusal for the person is its only
        // reading of it, answered by the prompt and a retry (design
        // section 6, Authentication needed; ADR-0042, A 2026-10-20).
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
                    // The debug build's hook: answered as a phone holding
                    // another identity, nothing sealed (seal_hook.rs).
                    Some(_) if crate::seal_hook::take() => {
                        log("seal hook: answered AlreadyProvisioned, nothing sealed");
                        Err(StepFailed::AlreadyProvisioned)
                    }
                    Some(creation) => match onboarding::seal(
                        &o.app_data_dir,
                        PROFILE,
                        o.cipher.as_ref(),
                        creation,
                        o.lock.as_ref(),
                    ) {
                        Err(WriteRefused::UserNotAuthenticated) => {
                            prompt_for(o, Effect::Seal);
                            return Vec::new();
                        }
                        sealed => sealed.map_err(|why| failed(&why)),
                    },
                    None => Err(StepFailed::Storage),
                };
                if outcome.is_ok() {
                    sealed_now(o);
                }
                o.flow.sealed(outcome)
            }
            // `IdStorageUnknown`'s Try again, or a granted prompt's
            // read-back: the read-back alone, answered as the step it reads
            // back for. A refusal for the person asks the prompt again;
            // whatever else stops it, the app still cannot tell, so it is
            // the notice again.
            Effect::ReadBack(peer) => {
                let outcome =
                    onboarding::read_back(&o.app_data_dir, PROFILE, o.cipher.as_ref(), &peer)
                        .map_err(|why| {
                            log(&format!("an identity read-back did not find it: {why:?}"));
                            match why {
                                WriteRefused::ReadBackNeedsAuthentication => {
                                    StepFailed::ReadBackNeedsAuthentication
                                }
                                _ => StepFailed::StorageUnknown,
                            }
                        });
                if o.flow.screen() == ScreenId::C4 {
                    if outcome.is_ok() {
                        sealed_now(o);
                    }
                    o.flow.sealed(outcome)
                } else {
                    if outcome.is_ok() {
                        connect_owed(o);
                    }
                    o.flow.restored(outcome)
                }
            }
            Effect::Check(text) => o.flow.checked(onboarding::check(text)),
            Effect::RestoreRecorded(text) => {
                // Kept for the retry a refusal for the person asks: the
                // words stay on the screen meanwhile anyway (section 6).
                let retry = Effect::RestoreRecorded(text.clone());
                let outcome = match onboarding::restore(
                    &o.app_data_dir,
                    PROFILE,
                    o.cipher.as_ref(),
                    text,
                    Expected::Recorded,
                    o.lock.as_ref(),
                ) {
                    Err(WriteRefused::UserNotAuthenticated) => {
                        prompt_for(o, retry);
                        return Vec::new();
                    }
                    restored => restored.map_err(|why| failed(&why)),
                };
                if outcome.is_ok() {
                    connect_owed(o);
                }
                o.flow.restored(outcome)
            }
            Effect::RestoreConfirmed(text, peer) => {
                let retry = Effect::RestoreConfirmed(text.clone(), peer.clone());
                let outcome = match onboarding::restore(
                    &o.app_data_dir,
                    PROFILE,
                    o.cipher.as_ref(),
                    text,
                    Expected::Confirmed(&peer),
                    o.lock.as_ref(),
                ) {
                    Err(WriteRefused::UserNotAuthenticated) => {
                        prompt_for(o, retry);
                        return Vec::new();
                    }
                    restored => restored.map_err(|why| failed(&why)),
                };
                if outcome.is_ok() {
                    connect_owed(o);
                }
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
                #[cfg(feature = "ui-trace")]
                log(&prompt_line(TracedPrompt::Phrase));
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
            // A read-back refused for the person: the prompt, and the
            // read-back again once it is granted.
            effect @ Effect::AuthenticateReadBack(_) => {
                prompt_for(o, effect);
                Vec::new()
            }
            Effect::OpenChats | Effect::OpenSettings => Vec::new(),
        }
    })
}

/// `effect` was refused until the person authenticates: the gate holds it
/// and the screen raises the prompt over itself, never the service.
fn prompt_for(o: &mut Onboarding, effect: Effect) {
    o.gate.refused(effect);
    // The person's time, not the app's: a held screen's clock stops while
    // the prompt is up (§5.1).
    if let Some(hold) = o.hold.as_mut() {
        hold.prompt_shown(now_ms());
    }
    // The prompt shows over the screen: a copy's confirmation there
    // closes (identity note section 5).
    o.view.borrow().close_peer_id_copied();
    gate_ask(o);
}

/// C4's identity is committed, by its seal or by the read-back of
/// `IdStorageUnknown`'s Try again: the creation dropped, Connect owed, and
/// the bound counting from here (§5.1).
fn sealed_now(o: &mut Onboarding) {
    o.creation = None;
    connect_owed(o);
    if let Some(hold) = o.hold.as_mut() {
        hold.committed(now_ms());
    }
    #[cfg(feature = "ui-trace")]
    log(&start_line(TracedStart::Sealed));
}

/// A new or restored identity owes onboarding's Connect on the first
/// start that reaches Chats (add-a-peer §5.1). A marker that could not be
/// written costs only Connect: Chats opens, with its no-reach notice.
fn connect_owed(o: &Onboarding) {
    if let Err(why) = onboarding::set_connect_owed(&o.app_data_dir, PROFILE, true) {
        log(&format!("writing the connect-owed marker: {why:?}"));
    }
}

/// Show the page for the flow's screen, or hand over once it is done.
/// A C4 or R4 held after Continue stays shown while its start runs: the
/// chats attach under it once the service is asked, and the hand-over
/// waits for [`start_decided`], so the first screen shown is Connect or
/// Chats, not Chats and then Connect (add-a-peer §5.1). `FLAG_SECURE` is
/// left as it was meanwhile: it is cleared from the hand-over below, as
/// before, so the decided page is the next to change it.
/// The flow does not end while the presence gate holds an effect, or the
/// prompt's answer would find nothing to resume (#309 F1); its end asks
/// the service back whenever the flow stopped it or a first run awaits it,
/// a cancelled prompt and a failed step included (F2); and `FLAG_SECURE`,
/// set by the Activity before any word, is cleared only once a frame
/// without the flow's page has been drawn (F3).
fn render() {
    let step = ONBOARDING.with(|slot| {
        let mut slot = slot.borrow_mut();
        let o = slot.as_mut()?;
        // A hold the flow gave back -- a failed seal, a cancelled seal
        // prompt -- ends its clock: nothing shows under Try again.
        if !o.flow.start_held() {
            o.hold = None;
        }
        let now = now_ms();
        if let Some(hold) = &o.hold {
            let starting = hold.starting(now);
            #[cfg(feature = "ui-trace")]
            if starting && !o.flow.starting() {
                log(&start_line(TracedStart::Starting));
            }
            o.flow.set_starting(starting);
        }
        // Said before the page is set, so a page's line comes before any
        // ask its showing leads to (SECURE_OFF after `identity=closed`).
        let page = (o.flow.screen() != ScreenId::Done).then(|| identity_page(&o.flow));
        #[cfg(feature = "ui-trace")]
        o.view
            .borrow_mut()
            .trace_identity(Some(o.flow.screen()), page.as_ref());
        if let Some(page) = page {
            o.view.borrow().show_identity(Some(page));
            schedule_hold(o.hold.as_ref(), now);
            // Held with the service asked: the chats attach under the
            // screen now, so the start's reads run while it holds and the
            // first screen shown is the one they decide (§5.1).
            if o.flow.start_held() && o.service_asked && !o.handed {
                o.handed = true;
                return Some(Step::Attach(Rc::clone(&o.done)));
            }
            return None;
        }
        schedule_hold(None, now);
        o.view.borrow().show_identity(None);
        if o.gate.holding() {
            return None;
        }
        if !o.service_asked && (o.over != Over::Settings || o.offline_asked) {
            ask_service(o);
        }
        let cleared = o.view.borrow_mut().after_next_frame(secure_off);
        if !cleared {
            log("no frame notifier: FLAG_SECURE stays on for this window");
        }
        Some(Step::Handover {
            done: Rc::clone(&o.done),
            over: o.over,
            start_asked: o.service_asked,
            handed: o.handed,
        })
    });
    let (done, over, start_asked, handed) = match step {
        None => return,
        Some(Step::Attach(done)) => {
            done();
            return;
        }
        Some(Step::Handover {
            done,
            over,
            start_asked,
            handed,
        }) => (done, over, start_asked, handed),
    };
    ONBOARDING.with(|slot| slot.borrow_mut().take());
    // A K1 a refused start kept while Show my phrase showed is still
    // owed when Show my phrase ends: `raise_recovery` held back for it.
    let mut raise = false;
    if let Ok(mut platform) = PLATFORM.lock()
        && let Some(platform) = platform.as_mut()
    {
        match at_flow_end(over, platform.kept, start_asked) {
            AtFlowEnd::Drop => {
                platform.start = None;
                platform.kept = false;
            }
            AtFlowEnd::Keep => {}
            AtFlowEnd::Raise => raise = true,
        }
    }
    if !handed {
        done();
    }
    if raise {
        invoke_on_window(crate::activity::raise_recovery);
    }
}

/// What a render leaves to do once the flow's slot is let go.
enum Step {
    /// Attach the chats under a held C4 or R4.
    Attach(Rc<dyn Fn()>),
    /// The flow is over: hand over, attaching the chats unless a held
    /// screen attached them already.
    Handover {
        done: Rc<dyn Fn()>,
        over: Over,
        start_asked: bool,
        handed: bool,
    },
}

thread_local! {
    /// Wakes a held C4 or R4 when its clock next changes what it shows or
    /// decides: the 1 s line, the 10 s bound.
    static HOLD_TIMER: Wake = Wake::default();
}

/// The hold's next wake, or none.
fn schedule_hold(hold: Option<&StartHold>, now: u64) {
    HOLD_TIMER.with(|timer| match hold.and_then(|hold| hold.next_change(now)) {
        Some(at) => timer.at(Duration::from_millis(at.saturating_sub(now)), hold_tick),
        None => timer.stop(),
    });
}

/// The held screen's clock changed something: drawn again, and past the
/// bound the root's turn ends the hold ([`hold_bound_passed`]).
fn hold_tick() {
    render();
    if hold_bound_passed() {
        crate::activity::turn_soon();
    }
}

/// The hub's count of answers before a held C4's or R4's start was
/// asked, while one is held with its start asked; `None` otherwise. The
/// root asks it each turn ([`start_decided`]).
pub(crate) fn held_start() -> Option<StoreGeneration> {
    ONBOARDING.with(|slot| {
        let o = slot.try_borrow().ok()?;
        let o = o.as_ref()?;
        (o.flow.start_held() && o.service_asked && o.hold.is_some()).then_some(o.asked_at)
    })
}

/// The held screen's 10 s bound has passed (§5.1).
pub(crate) fn hold_bound_passed() -> bool {
    ONBOARDING.with(|slot| {
        slot.try_borrow().is_ok_and(|o| {
            o.as_ref()
                .and_then(|o| o.hold.as_ref())
                .is_some_and(|hold| hold.bound_passed(now_ms()))
        })
    })
}

/// The start under a held screen was refused with the profile in use
/// before its bound: the screen stays, with `IdStartBusy` and Try again,
/// focused (identity note section 6). Its Try again asks the service
/// again ([`ask_service`]), a restart of the bound one. Called by the root
/// as [`start_decided`] is.
pub(crate) fn start_busy() {
    ONBOARDING.with(|slot| {
        if let Some(o) = slot.borrow_mut().as_mut() {
            o.flow.start_busy();
        }
    });
    render();
}

/// The start under a held C4 or R4 decided -- Connect or Chats, which
/// the root's model shows -- or the bound passed, or the start ended
/// undecided: the screen goes, and the flow hands over. Called by the
/// root around its turn, outside any borrow of the flow.
pub(crate) fn start_decided() {
    let effects = ONBOARDING.with(|slot| {
        slot.borrow_mut()
            .as_mut()
            .map(|o| o.flow.decided())
            .unwrap_or_default()
    });
    run(effects);
    render();
}

/// `FLAG_SECURE` off, once a frame without the flow was drawn or Show my
/// address closed -- unless what it secures is still drawn by the time it
/// is asked ([`secure_may_clear`]): an identity flow, as K1 raised at Show
/// my phrase's end begins in the same turn of the event loop, before that
/// frame (#615 F1); or Show my address, open under a K1 that a refused
/// start raised over it and a restore ended. Whichever is left asks again
/// at its own end. The decision is host-tested; this reading of its two
/// inputs is Android-only, and a device case covers the flow's
/// (`kept_recovery.rs`), none Show's (#619 F1).
pub(crate) fn secure_off() {
    // A slot already borrowed counts as a flow showing, said on a line of
    // its own: the off is asked from a frame task or Show's off, which
    // run outside `render` and an input, and were one to run inside, the
    // flag staying on is the side a wrong guess may fall on.
    let read = ONBOARDING.with(|slot| slot.try_borrow().map(|o| o.is_some()));
    let flow = *read.as_ref().unwrap_or(&true);
    let show = crate::activity::show_open();
    if !secure_may_clear(flow, show) {
        log(match read {
            Err(_) => "the identity flow is busy: FLAG_SECURE stays on",
            Ok(true) => "an identity flow shows: FLAG_SECURE stays on",
            Ok(false) => "Show my address is open: FLAG_SECURE stays on",
        });
        return;
    }
    ASKS.push(ask::SECURE_OFF);
}

/// Ask the Activity to start the service: bound as a launch does after a
/// first run or a recovery, brought back after Show my phrase. The lock
/// goes first: the service takes it.
fn ask_service(o: &mut Onboarding) {
    o.lock = None;
    let start = o.start_ask();
    o.service_asked = true;
    let hub = ServiceHost::global().hub();
    // Before the ask: an answer counted past this is this start's.
    o.asked_at = hub.store_generation();
    // A held screen shows its own start's answer: a notice an earlier
    // start left on Chats is not this one's.
    hub.start_asked(false);
    clear_authentication();
    person_asked();
    FLOW_START.store(o.flow.start_held(), Ordering::SeqCst);
    ASKS.push(start);
}
