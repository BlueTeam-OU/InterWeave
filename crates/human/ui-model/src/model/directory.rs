// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The contacts as the windowed mode reads them (design/2026-10-27-start-
//! window.md section 11): every contact without its photo's bytes, each
//! photo read when a view shows it, and the state a failed read leaves --
//! in which nothing writes a contact, since a card's comparison with the
//! contacts is a security surface (ADR-0059) never made against contacts
//! that could not be read.

use std::collections::{HashSet, VecDeque};

use super::UiModel;
use super::cards::{ContactView, Photo};

/// How many decoded photos are held: about 16 MiB at 256 x 256 RGBA. A
/// photo shown in view is never the one evicted.
pub const PHOTO_CAP: usize = 64;

/// Whether the contacts were read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContactsState {
    /// Read: names, routes and which have a photo are known.
    #[default]
    Read,
    /// The read failed: no contact is known, and none is written.
    Unread,
    /// Try again is reading them.
    Reading,
}

/// A contact as the directory read gives it: no photo bytes, and whether
/// it has a photo to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryContact {
    /// The contact, its `photo` `None`.
    pub view: ContactView,
    /// Whether the store holds a photo for it.
    pub has_photo: bool,
}

/// A photo the model asks the facade to read: the contact's, as it stood
/// at `photo_at`, so a photo changed since is not the one shown.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhotoAsk {
    /// The contact.
    pub contact_id: String,
    /// When its photo was last set.
    pub photo_at: Option<u64>,
}

/// What a view shows in a contact's avatar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoState<'a> {
    /// The contact has no photo: its initials.
    None,
    /// It has one not read yet: its initials, at the photo's size, until
    /// the photo arrives. A contact page waits for it rather than opening
    /// on these (design section 11).
    Initials,
    /// It has one that cannot be read: its initials stay, and the photo
    /// can still be removed.
    Unreadable,
    /// Its photo.
    Shown(&'a Photo),
}

#[derive(Debug, Default)]
pub(super) struct Directory {
    pub(super) state: ContactsState,
    /// A read is asked and not taken yet.
    asked: bool,
    /// The contacts that have a photo to read, by id and photo time.
    has_photo: HashSet<(String, Option<u64>)>,
    /// Decoded photos, oldest first, each by its contact and photo time: a
    /// photo the contact no longer has is shown no more at once, since the
    /// time it is looked up by changed, and it leaves under the cap.
    photos: VecDeque<((String, Option<u64>), Photo)>,
    /// Photos that could not be read: their initials stay.
    unreadable: HashSet<(String, Option<u64>)>,
    in_flight: HashSet<(String, Option<u64>)>,
    /// The contacts whose avatar a view shows now.
    in_view: HashSet<String>,
    asks: VecDeque<PhotoAsk>,
}

impl UiModel {
    /// The contacts the facade read, every one without its photo's bytes,
    /// or the read's failure: no contact is then known -- a conversation
    /// title falls back to the short `PeerId` -- and none is written until
    /// a read succeeds (design section 11).
    pub fn contacts_directory(&mut self, answer: Result<Vec<DirectoryContact>, ()>) {
        if let Ok(contacts) = answer {
            let directory = &mut self.directory;
            directory.state = ContactsState::Read;
            directory.has_photo = contacts
                .iter()
                .filter(|c| c.has_photo)
                .map(|c| (c.view.contact_id.clone(), c.view.photo_at))
                .collect();
            self.contacts_listed(contacts.into_iter().map(|c| c.view).collect());
        } else {
            self.directory.state = ContactsState::Unread;
            self.contacts_listed(Vec::new());
        }
    }

    /// Whether the contacts were read.
    #[must_use]
    pub const fn contacts_state(&self) -> ContactsState {
        self.directory.state
    }

    /// The person pressed Try again on the Contacts list's failure row:
    /// every contact is read again, once -- with the row gone no second
    /// read can start while one is on its way. True when a read was asked.
    pub fn contacts_try_again(&mut self) -> bool {
        if self.directory.state != ContactsState::Unread {
            return false;
        }
        self.directory.state = ContactsState::Reading;
        self.directory.asked = true;
        true
    }

    /// Whether a read of every contact was asked since the last call.
    pub const fn take_contacts_ask(&mut self) -> bool {
        let asked = self.directory.asked;
        self.directory.asked = false;
        asked
    }

    /// Whether naming a person by hand (contact-cards section 5.6) is
    /// offered: not while the contacts are unread, since it writes a
    /// contact without knowing whether the route is already under one.
    #[must_use]
    pub fn naming_offered(&self) -> bool {
        self.directory.state == ContactsState::Read
    }

    /// A view shows `contact_id`'s avatar: its photo is asked, once, if it
    /// has one not held.
    pub fn photo_shown(&mut self, contact_id: &str) {
        self.directory.in_view.insert(contact_id.to_owned());
        let Some(key) = self.photo_key(contact_id) else {
            return;
        };
        let directory = &mut self.directory;
        if directory.has_photo.contains(&key)
            && !directory.photos.iter().any(|(k, _)| *k == key)
            && !directory.unreadable.contains(&key)
            && directory.in_flight.insert(key.clone())
        {
            directory.asks.push_back(PhotoAsk {
                contact_id: key.0,
                photo_at: key.1,
            });
        }
    }

    /// The avatar left the view: its photo may be evicted again.
    pub fn photo_hidden(&mut self, contact_id: &str) {
        self.directory.in_view.remove(contact_id);
    }

    /// The photos asked since the last call, for the facade to read.
    pub fn take_photo_asks(&mut self) -> Vec<PhotoAsk> {
        self.directory.asks.drain(..).collect()
    }

    /// A photo the facade read, `None` when it could not be decoded: its
    /// initials stay. One for a photo the contact no longer has is not
    /// kept.
    pub fn photo_arrived(&mut self, contact_id: &str, photo_at: Option<u64>, photo: Option<Photo>) {
        let key = (contact_id.to_owned(), photo_at);
        let directory = &mut self.directory;
        directory.in_flight.remove(&key);
        if !directory.has_photo.contains(&key) {
            return;
        }
        match photo {
            Some(photo) => {
                directory.photos.retain(|(k, _)| *k != key);
                directory.photos.push_back((key, photo));
                while directory.photos.len() > PHOTO_CAP {
                    let Some(at) = directory
                        .photos
                        .iter()
                        .position(|((id, _), _)| !directory.in_view.contains(id))
                    else {
                        break;
                    };
                    directory.photos.remove(at);
                }
            }
            None => {
                directory.unreadable.insert(key);
            }
        }
    }

    /// What `contact_id`'s avatar shows now.
    #[must_use]
    pub fn contact_photo(&self, contact_id: &str) -> PhotoState<'_> {
        let Some(contact) = self.contact(contact_id) else {
            return PhotoState::None;
        };
        // A contact handed to the model with its photo decoded -- the
        // model's own tests do so; the facade hands the directory, which
        // holds none (#501 review F3), and the photo is asked for.
        if let Some(photo) = &contact.photo {
            return PhotoState::Shown(photo);
        }
        let key = (contact_id.to_owned(), contact.photo_at);
        if !self.directory.has_photo.contains(&key) {
            return PhotoState::None;
        }
        if self.directory.unreadable.contains(&key) {
            return PhotoState::Unreadable;
        }
        self.directory
            .photos
            .iter()
            .find(|(k, _)| *k == key)
            .map_or(PhotoState::Initials, |(_, p)| PhotoState::Shown(p))
    }

    /// Whether `contact_id` has a photo in the store, read or not: what
    /// Remove photo is offered by, the windowed mode's contacts holding no
    /// photo's bytes.
    #[must_use]
    pub fn contact_has_photo(&self, contact_id: &str) -> bool {
        self.contact(contact_id).is_some_and(|c| {
            c.photo.is_some()
                || self
                    .directory
                    .has_photo
                    .contains(&(contact_id.to_owned(), c.photo_at))
        })
    }

    /// How many stored rows the app holds and cannot show: the facade's
    /// count from the last listing or start.
    pub const fn set_rows_not_shown(&mut self, count: usize) {
        self.rows_not_shown = count;
    }

    /// How many stored rows the app holds and cannot show; Chats ends with
    /// a line saying so when there are any (design section 6.1).
    #[must_use]
    pub const fn rows_not_shown(&self) -> usize {
        self.rows_not_shown
    }

    fn photo_key(&self, contact_id: &str) -> Option<(String, Option<u64>)> {
        self.contact(contact_id)
            .map(|c| (contact_id.to_owned(), c.photo_at))
    }
}

#[cfg(test)]
mod tests;
