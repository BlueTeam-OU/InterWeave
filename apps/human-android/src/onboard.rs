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
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use interweave_human_android_platform::custody::{self, RecoveryCause};
use interweave_human_android_platform::onboarding::{self, Creation, WriteRefused, english};
use interweave_human_android_platform::{Expected, IdentityRefused};
use interweave_human_ui_model::{
    Effect, IdentityFlow, RecoveryReason, ScreenId, Start, StepFailed,
};
use interweave_human_ui_slint::{
    IdentityInput, View, apply_identity, defer, identity_page, invoke_on_window,
};
use interweave_profile_config::{ProfilePaths, TrustBoundary};

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
}

/// What the window asks of the Activity's Kotlin side
/// (`Native.waitIdentityAsk`).
pub(crate) mod ask {
    use jni::sys::jint;
    /// Words are shown or entered: `FLAG_SECURE` on.
    pub(crate) const SECURE_ON: jint = 1;
    /// No word on screen: `FLAG_SECURE` off.
    pub(crate) const SECURE_OFF: jint = 2;
    /// The device-credential or strong-biometric prompt; answered by
    /// `Native.prompted`.
    pub(crate) const PROMPT: jint = 3;
    /// The identity is ready: bind the network service as a launch does.
    pub(crate) const BIND: jint = 4;
    /// Back on a first screen: finish the Activity.
    pub(crate) const FINISH: jint = 5;
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

    fn push(&self, ask: jni::sys::jint) {
        self.queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(ask);
        self.changed.notify_all();
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
}

static PLATFORM: Mutex<Option<Platform>> = Mutex::new(None);

/// `Native.identityState`: whether the identity opens, and if not, where
/// the flow starts. Any other refusal is the service's to report, as it
/// does today, so it reads as ready.
pub(crate) fn identity_state(app_data_dir: PathBuf, cipher: KeystoreCipher) -> jni::sys::jint {
    let found = interweave_human_android_platform::start_identity(
        &app_data_dir,
        PROFILE,
        &cipher,
        interweave_human_android_platform::FirstRun::Refuse,
    );
    let start = match found {
        Ok(_) => None,
        Err(IdentityRefused::Unprovisioned) => Some(Start::FirstRun {
            void_paper: onboarding::void_paper(&app_data_dir, PROFILE),
        }),
        Err(IdentityRefused::RecoveryRequired(cause)) => Some(Start::Recovery {
            reason: match cause {
                RecoveryCause::KeyInvalidated | RecoveryCause::KeyMissing => RecoveryReason::Key,
                _ => RecoveryReason::Record,
            },
            // The PeerId the record names, read without the cipher; none
            // readable offers no restore (design section 3.4).
            expected: TrustBoundary::new(&app_data_dir)
                .and_then(|boundary| ProfilePaths::resolve_embedded(PROFILE, boundary))
                .ok()
                .and_then(|paths| custody::recorded_identity(&paths).ok().flatten()),
        }),
        Err(why) => {
            log(&format!(
                "the identity's state: {why:?}; the service reports it"
            ));
            None
        }
    };
    let code = match &start {
        None => state::READY,
        Some(Start::FirstRun { .. }) => state::FIRST_RUN,
        Some(_) => state::RECOVERY,
    };
    *PLATFORM.lock().unwrap_or_else(PoisonError::into_inner) = Some(Platform {
        app_data_dir,
        cipher: Arc::new(cipher),
        start,
    });
    code
}

struct Onboarding {
    view: Rc<RefCell<View>>,
    flow: IdentityFlow,
    creation: Option<Creation>,
    app_data_dir: PathBuf,
    cipher: Arc<KeystoreCipher>,
    secure: bool,
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
    let Some(Platform {
        app_data_dir,
        cipher,
        start: Some(start),
    }) = platform.as_ref()
    else {
        return false;
    };
    let onboarding = Onboarding {
        view: Rc::clone(view),
        flow: IdentityFlow::new(english(), start.clone()),
        creation: None,
        app_data_dir: app_data_dir.clone(),
        cipher: Arc::clone(cipher),
        secure: false,
        done: Rc::new(done),
    };
    drop(platform);
    view.borrow().on_identity(|input| {
        // Applied after the callback returns: the flow may call back into
        // the view, which is borrowed while its callback runs.
        defer(move || input_later(input));
    });
    ONBOARDING.with(|slot| *slot.borrow_mut() = Some(onboarding));
    render();
    true
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

/// `Native.prompted`: the credential prompt's answer.
pub(crate) fn prompted(granted: bool) {
    invoke_on_window(move || {
        let effects = ONBOARDING.with(|slot| {
            slot.borrow_mut()
                .as_mut()
                .map(|o| o.flow.prompted(granted))
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
        match effect {
            Effect::Finish => {
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
                    Some(creation) => {
                        onboarding::seal(&o.app_data_dir, PROFILE, o.cipher.as_ref(), creation)
                            .map_err(|why| failed(&why))
                    }
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
                )
                .map_err(|why| failed(&why));
                o.flow.restored(outcome)
            }
            Effect::Prompt => {
                ASKS.push(ask::PROMPT);
                Vec::new()
            }
            // Show my phrase is reached from Settings, which gains its
            // entry with the service's stop and restart; until then no
            // screen asks this.
            Effect::GoOffline => o.flow.offline(Err(StepFailed::Storage)),
            Effect::StartService => {
                ASKS.push(ask::BIND);
                Vec::new()
            }
            Effect::OpenChats | Effect::OpenSettings => Vec::new(),
        }
    })
}

/// Show the page for the flow's screen, or hand over to the chats once it
/// is done; keep the secure flag on while a word is on screen.
fn render() {
    let handover = ONBOARDING.with(|slot| {
        let mut slot = slot.borrow_mut();
        let o = slot.as_mut()?;
        let screen = o.flow.screen();
        let secure = matches!(
            screen,
            ScreenId::C2 | ScreenId::C3 | ScreenId::R2 | ScreenId::R3 | ScreenId::S2
        );
        if secure != o.secure {
            o.secure = secure;
            ASKS.push(if secure {
                ask::SECURE_ON
            } else {
                ask::SECURE_OFF
            });
        }
        if screen == ScreenId::Done {
            o.view.borrow().show_identity(None);
            Some(Rc::clone(&o.done))
        } else {
            o.view.borrow().show_identity(Some(identity_page(&o.flow)));
            None
        }
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
