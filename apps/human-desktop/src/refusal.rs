// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Why the app did not start, once a window can say it: every start-up
//! refusal a person can act on, as one screen (plan §18 clause (4), the
//! decision of 2026-10-20) beside the stderr line and the exit class the
//! terminal and the journal keep.
//!
//! Two refusals get no screen: a command line the app does not take,
//! which is the launcher's to fix, and a platform that cannot draw text
//! or lock a file. Both stay a stderr line and an exit class.

use std::fmt;
use std::path::PathBuf;

use interweave_human_ui_model::UiText;
use interweave_human_ui_slint::{RefusalPage, RefusalView};
use interweave_profile_config::profile_format::{self, ProfileFormatError};
use interweave_profile_config::{LoadError, PersistError};

use crate::run::{EX_CONFIG, EX_DATAERR, EX_IOERR, EX_NOPERM, EX_TEMPFAIL, EX_UNAVAILABLE};
use crate::startup::{Blocked, StartError};

/// A refusal after the command line and the platform check.
#[derive(Debug)]
pub enum Refusal {
    /// The profile could not be resolved or loaded.
    Profile(StartError),
    /// Another window holds this profile's store.
    AlreadyOpen,
    /// The single-instance lock could not be taken for another reason.
    Lock {
        /// Why.
        error: PersistError,
        /// The folder the lock lives in, which the screen names when the
        /// error names no path of its own.
        folder: PathBuf,
    },
    /// The store cannot be used.
    Store(Blocked),
}

impl Refusal {
    /// Its sysexits(3) class, as before the screen existed.
    #[must_use]
    pub const fn code(&self) -> u8 {
        match self {
            Self::Profile(_) => EX_CONFIG,
            Self::AlreadyOpen => EX_TEMPFAIL,
            Self::Lock {
                error: PersistError::UnsupportedPlatform,
                ..
            } => EX_UNAVAILABLE,
            Self::Store(Blocked::Recovery { .. }) => EX_DATAERR,
            Self::Lock {
                error:
                    PersistError::DirectoryNotPrivate { .. } | PersistError::FileNotPrivate { .. },
                ..
            }
            | Self::Store(Blocked::NotPrivate { .. }) => EX_NOPERM,
            Self::Lock { .. } | Self::Store(Blocked::Unavailable { .. }) => EX_IOERR,
        }
    }

    /// What the screen says, or none when no screen is shown for it.
    /// Try again is offered for the store's refusals: the root runs the
    /// open again under the lock it holds, and a person may have put the
    /// file right meanwhile. The others are decided before the lock and
    /// need a new start.
    #[must_use]
    pub fn page(&self) -> Option<RefusalPage> {
        let page = |reason, path: Option<PathBuf>, can_retry| RefusalPage {
            reason,
            path: path.map(|p| p.display().to_string()),
            can_retry,
        };
        Some(match self {
            Self::Profile(error) => {
                let (reason, path) = profile_reason(error);
                page(reason, path, false)
            }
            Self::AlreadyOpen => page(UiText::RefusalAlreadyOpen, None, false),
            Self::Lock {
                error: PersistError::UnsupportedPlatform,
                ..
            } => return None,
            Self::Lock { error, folder } => {
                let path = persist_path(error).unwrap_or_else(|| folder.clone());
                let reason = if self.code() == EX_NOPERM {
                    UiText::RefusalNotPrivate
                } else {
                    UiText::RefusalUnavailable
                };
                page(reason, Some(path), false)
            }
            Self::Store(Blocked::Recovery { path }) => {
                page(UiText::RefusalStoreRecovery, Some(path.clone()), true)
            }
            Self::Store(Blocked::NotPrivate { path, .. }) => {
                page(UiText::RefusalStoreNotPrivate, Some(path.clone()), true)
            }
            Self::Store(Blocked::Unavailable { path }) => {
                page(UiText::RefusalStoreUnavailable, Some(path.clone()), true)
            }
        })
    }
}

/// The stderr line: the technical account, for the terminal and the
/// journal. It names paths and causes and never message content.
impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Profile(e) => e.fmt(f),
            // What, not why: the lock's path and holder are nothing a
            // person can act on (human-client-ui.md section 12, as
            // EndpointInUse).
            Self::AlreadyOpen => f.write_str("this profile's window is already open"),
            Self::Lock { error, .. } => error.fmt(f),
            Self::Store(b) => b.fmt(f),
        }
    }
}

fn profile_reason(error: &StartError) -> (UiText, Option<PathBuf>) {
    match error {
        StartError::Config { error, paths } => match error {
            LoadError::ProfileFormat(ProfileFormatError::Newer { path, .. }) => {
                (UiText::RefusalProfileNewer, Some(path.clone()))
            }
            LoadError::ProfileFormat(_) => (
                UiText::RefusalProfileSettings,
                Some(profile_format::path_for(paths)),
            ),
            LoadError::ConfigDirUnguarded(e) => (
                UiText::RefusalNotPrivate,
                persist_path(e).or_else(|| Some(paths.config_file())),
            ),
            LoadError::ConfigFileUnguarded { path, .. } => {
                (UiText::RefusalNotPrivate, Some(path.clone()))
            }
            _ => (UiText::RefusalProfileSettings, Some(paths.config_file())),
        },
        StartError::NoHumanEndpoint { paths } => {
            (UiText::RefusalProfileSettings, Some(paths.config_file()))
        }
        // An error that names a path is a privacy refusal, and reads as
        // one with the folder shown; the rest name no folder to show.
        StartError::Directories(e) | StartError::Paths(e) | StartError::Sockets(e) => {
            match persist_path(e) {
                Some(path) => (UiText::RefusalNotPrivate, Some(path)),
                None => (UiText::RefusalProfileFolders, None),
            }
        }
        StartError::RolesOverlap => (UiText::RefusalProfileFolders, None),
    }
}

fn persist_path(error: &PersistError) -> Option<PathBuf> {
    match error {
        PersistError::DirectoryNotPrivate { path, .. }
        | PersistError::FileNotPrivate { path, .. } => Some(path.clone()),
        _ => None,
    }
}

/// What Try again runs: the refused step again. What it may write is
/// that step's: the store's open writes nothing for a store refused for
/// its header, and may checkpoint a WAL for one refused later
/// (`HumanStore::open_within`'s NOT COVERED case).
pub type Retry<T> = Box<dyn FnMut() -> Result<T, Refusal>>;

/// Show `refusal` until the person closes the screen, running `retry`
/// at each Try again: its success closes the screen and is returned, its
/// refusal is shown in place, said on stderr like the first.
///
/// No window can be shown -- no display, no backend -- and this returns
/// at once: the stderr line already said it.
///
/// # Errors
/// The exit class of the last refusal, once the screen is closed or
/// cannot be shown.
pub fn show<T: 'static>(refusal: &Refusal, retry: Option<Retry<T>>) -> Result<T, u8> {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    let Some(mut page) = refusal.page() else {
        return Err(refusal.code());
    };
    // Offered only where there is something to run again.
    page.can_retry &= retry.is_some();
    let Some(retry) = retry else {
        let Ok(view) = RefusalView::new(&page) else {
            return Err(refusal.code());
        };
        let _ = view.run();
        return Err(refusal.code());
    };
    let Ok(view) = RefusalView::new(&page) else {
        return Err(refusal.code());
    };
    let view = Rc::new(view);
    let code = Rc::new(Cell::new(refusal.code()));
    let opened: Rc<RefCell<Option<T>>> = Rc::new(RefCell::new(None));
    let retry = RefCell::new(retry);
    {
        let (weak, code, opened) = (Rc::downgrade(&view), Rc::clone(&code), Rc::clone(&opened));
        view.on_retry(move || {
            let Some(view) = weak.upgrade() else { return };
            match press(&opened, &mut *retry.borrow_mut()) {
                Pressed::Opened => view.close(),
                Pressed::Refused(again) => {
                    eprintln!("human-desktop: {again}");
                    if let Some(page) = again.page() {
                        view.show_page(&page);
                    }
                    view.tried_again();
                    code.set(again.code());
                }
                Pressed::AlreadyOpen => {}
            }
        });
    }
    // A screen that could not be shown has said nothing more than the
    // stderr line, which is the same answer.
    let _ = view.run();
    let value = opened.borrow_mut().take();
    value.ok_or_else(|| code.get())
}

/// What one press of Try again came to.
enum Pressed {
    /// The step succeeded; its value waits in `opened`.
    Opened,
    /// The step refused again.
    Refused(Refusal),
    /// An earlier press had already succeeded: nothing was run. A press
    /// queued before the screen's close reaches the event loop arrives
    /// after that success, and a second open would replace the store the
    /// first one handed over (`a_press_after_success_runs_nothing`).
    AlreadyOpen,
}

fn press<T>(
    opened: &std::cell::RefCell<Option<T>>,
    retry: &mut dyn FnMut() -> Result<T, Refusal>,
) -> Pressed {
    if opened.borrow().is_some() {
        return Pressed::AlreadyOpen;
    }
    match retry() {
        Ok(value) => {
            *opened.borrow_mut() = Some(value);
            Pressed::Opened
        }
        Err(again) => Pressed::Refused(again),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use interweave_profile_config::{ProfilePaths, XdgRoots};

    use super::*;

    fn paths() -> ProfilePaths {
        let at = |sub: &str| PathBuf::from("/x").join(sub);
        let roots = XdgRoots {
            config_home: at("config"),
            data_home: at("data"),
            state_home: at("state"),
            cache_home: at("cache"),
            runtime_dir: Some(at("run")),
        };
        ProfilePaths::resolve("desk", &roots).expect("paths")
    }

    fn config(error: LoadError) -> Refusal {
        Refusal::Profile(StartError::Config {
            error,
            paths: Box::new(paths()),
        })
    }

    fn lock(error: PersistError) -> Refusal {
        Refusal::Lock {
            error,
            folder: PathBuf::from("/x/state/human"),
        }
    }

    fn shown(refusal: &Refusal) -> (UiText, Option<String>, bool) {
        let page = refusal.page().expect("a screen");
        (page.reason, page.path, page.can_retry)
    }

    fn path(p: &Path) -> Option<String> {
        p.to_str().map(str::to_owned)
    }

    #[test]
    fn only_the_stores_refusals_offer_try_again() {
        let store = PathBuf::from("/x/human.sqlite");
        let cases = [
            (
                Refusal::Store(Blocked::Recovery {
                    path: store.clone(),
                }),
                true,
            ),
            (
                Refusal::Store(Blocked::NotPrivate {
                    path: store.clone(),
                    cause: "mode 0644".to_owned(),
                }),
                true,
            ),
            (Refusal::Store(Blocked::Unavailable { path: store }), true),
            (Refusal::AlreadyOpen, false),
            (lock(PersistError::Io(std::io::Error::other("x"))), false),
            (config(LoadError::Parse("x".to_owned())), false),
            (Refusal::Profile(StartError::RolesOverlap), false),
        ];
        for (refusal, retry) in cases {
            assert_eq!(shown(&refusal).2, retry, "{refusal:?}");
        }
    }

    #[test]
    fn each_refusal_keeps_its_exit_class() {
        let store = PathBuf::from("/x/human.sqlite");
        let not_private = || PersistError::DirectoryNotPrivate {
            path: "/x".into(),
            detail: "mode 0755".to_owned(),
        };
        let cases = [
            (
                Refusal::Store(Blocked::Recovery {
                    path: store.clone(),
                }),
                EX_DATAERR,
            ),
            (
                Refusal::Store(Blocked::NotPrivate {
                    path: store.clone(),
                    cause: String::new(),
                }),
                EX_NOPERM,
            ),
            (
                Refusal::Store(Blocked::Unavailable { path: store }),
                EX_IOERR,
            ),
            (Refusal::AlreadyOpen, EX_TEMPFAIL),
            (lock(not_private()), EX_NOPERM),
            (lock(PersistError::UnsupportedPlatform), EX_UNAVAILABLE),
            (lock(PersistError::Io(std::io::Error::other("x"))), EX_IOERR),
            (config(LoadError::Parse("x".to_owned())), EX_CONFIG),
            (Refusal::Profile(StartError::RolesOverlap), EX_CONFIG),
        ];
        for (refusal, code) in cases {
            assert_eq!(refusal.code(), code, "{refusal:?}");
        }
    }

    #[test]
    fn a_profile_refusal_names_the_file_a_person_would_open() {
        let paths = paths();
        let marker = PathBuf::from("/x/state/marker.json");
        assert_eq!(
            shown(&config(LoadError::ProfileFormat(
                ProfileFormatError::Newer {
                    path: marker.clone(),
                    found: 2,
                    own: 1,
                }
            ))),
            (UiText::RefusalProfileNewer, path(&marker), false)
        );
        assert_eq!(
            shown(&config(LoadError::ProfileFormat(
                ProfileFormatError::TooLarge
            ))),
            (
                UiText::RefusalProfileSettings,
                path(&profile_format::path_for(&paths)),
                false
            )
        );
        assert_eq!(
            shown(&config(LoadError::Parse("x".to_owned()))),
            (
                UiText::RefusalProfileSettings,
                path(&paths.config_file()),
                false
            )
        );
        assert_eq!(
            shown(&Refusal::Profile(StartError::NoHumanEndpoint {
                paths: Box::new(paths.clone()),
            })),
            (
                UiText::RefusalProfileSettings,
                path(&paths.config_file()),
                false
            )
        );
        let file = PathBuf::from("/x/config/config.yaml");
        assert_eq!(
            shown(&config(LoadError::ConfigFileUnguarded {
                path: file.clone(),
                detail: String::new(),
            })),
            (UiText::RefusalNotPrivate, path(&file), false)
        );
        assert_eq!(
            shown(&Refusal::Profile(StartError::RolesOverlap)),
            (UiText::RefusalProfileFolders, None, false)
        );
        assert_eq!(
            shown(&Refusal::Profile(StartError::Paths(
                PersistError::DirectoryNotPrivate {
                    path: "/x/state".into(),
                    detail: String::new(),
                }
            ))),
            (
                UiText::RefusalNotPrivate,
                Some("/x/state".to_owned()),
                false
            ),
            "a profile folder others can reach reads as not private, with the folder"
        );
    }

    #[test]
    fn a_lock_refusal_names_the_folder_and_an_unsupported_platform_gets_no_screen() {
        assert_eq!(
            shown(&lock(PersistError::DirectoryNotPrivate {
                path: "/x/state".into(),
                detail: String::new(),
            })),
            (
                UiText::RefusalNotPrivate,
                Some("/x/state".to_owned()),
                false
            )
        );
        assert_eq!(
            shown(&lock(PersistError::Io(std::io::Error::other("x")))),
            (
                UiText::RefusalUnavailable,
                Some("/x/state/human".to_owned()),
                false
            ),
            "the lock's folder, when the error names no path"
        );
        assert_eq!(
            shown(&Refusal::AlreadyOpen),
            (UiText::RefusalAlreadyOpen, None, false)
        );
        assert!(lock(PersistError::UnsupportedPlatform).page().is_none());
    }
    #[test]
    fn a_press_after_success_runs_nothing() {
        let opened = std::cell::RefCell::new(None);
        let mut runs = 0;
        let mut retry = || {
            runs += 1;
            if runs == 1 {
                Err(Refusal::AlreadyOpen)
            } else {
                Ok(runs)
            }
        };
        assert!(matches!(press(&opened, &mut retry), Pressed::Refused(_)));
        assert!(matches!(press(&opened, &mut retry), Pressed::Opened));
        assert!(matches!(press(&opened, &mut retry), Pressed::AlreadyOpen));
        assert_eq!(runs, 2, "the third press ran nothing");
        assert_eq!(*opened.borrow(), Some(2), "the first success is kept");
    }
}
