// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The pending-add record (schema v15; STATE.md's pending-add paragraph,
//! ADR-0061 decision 7, A 2026-11-26): one row per local `EndpointId` the
//! add form asked for and the client does not yet know to be an identity.
//! Written after the form generates the key pair and before the key file
//! is sealed, the platform key made or the administrative add sent, so a
//! key the form made is always named by a row; deleted when the id is an
//! identity, when a form left on a known refusal deleted its key, or when
//! a discard of the record ended. Insert and delete only: a row is never
//! updated.
//!
//! The record is the client's, not an identity's, so every handle of one
//! open reads and writes the same rows (`a_pending_add_is_one_for_every_handle`),
//! as [`HumanStore::last_front`]'s row is.

use rusqlite::params;

use interweave_transport_api::EndpointId;

use super::{Core, HumanStore};
use crate::StoreError;

/// At most this many rows: the profile's bound on endpoints, so a full
/// record names as many ids as a profile can hold. Held equal to
/// profile-config's `MAX_ENDPOINTS` by the assertion below, not by this
/// sentence.
pub const PENDING_ADD_CAP: usize = 64;

const _: () = assert!(PENDING_ADD_CAP == interweave_profile_config::MAX_ENDPOINTS);

impl HumanStore {
    /// Write `endpoint` to the pending-add record. A row already held is
    /// accepted again and stays one row, also at the bound, so a write
    /// retried after an answer that was lost is safe
    /// (`writing_the_same_pending_add_twice_keeps_one_row`).
    ///
    /// # Errors
    /// [`StoreError::PendingAddBound`] when [`PENDING_ADD_CAP`] other ids
    /// are held: nothing is written and no row is evicted
    /// (`the_65th_pending_add_is_refused_and_no_row_is_evicted`); a
    /// storage error otherwise.
    pub fn add_pending_add(&mut self, endpoint: &EndpointId) -> Result<(), StoreError> {
        self.lock().add_pending_add(endpoint)
    }

    /// Every id the pending-add record names, in the order they were
    /// written.
    ///
    /// # Errors
    /// A storage error, or [`StoreError::Corrupt`] for a row that is not a
    /// derived `EndpointId`, which this build does not write.
    pub fn pending_adds(&self) -> Result<Vec<EndpointId>, StoreError> {
        self.lock().pending_adds()
    }

    /// Delete `endpoint`'s row: whether there was one. Deleting a row
    /// that is not there is not an error.
    ///
    /// # Errors
    /// A storage error.
    pub fn forget_pending_add(&mut self, endpoint: &EndpointId) -> Result<bool, StoreError> {
        self.lock().forget_pending_add(endpoint)
    }
}

impl Core {
    pub(crate) fn add_pending_add(&mut self, endpoint: &EndpointId) -> Result<(), StoreError> {
        let written = (|| -> Result<bool, rusqlite::Error> {
            let tx = self.conn.transaction()?;
            // The OTHER rows are counted, so an id already held is not
            // counted against the bound twice and is accepted at it; the
            // count and the insert share one transaction.
            let others: i64 = tx.query_row(
                "SELECT COUNT(*) FROM pending_adds WHERE endpoint_id <> ?1",
                params![endpoint.as_str()],
                |r| r.get(0),
            )?;
            if usize::try_from(others).map_or(true, |n| n >= PENDING_ADD_CAP) {
                return Ok(false);
            }
            tx.execute(
                "INSERT INTO pending_adds (endpoint_id) VALUES (?1)
                 ON CONFLICT(endpoint_id) DO NOTHING",
                params![endpoint.as_str()],
            )?;
            tx.commit()?;
            Ok(true)
        })();
        match written {
            Ok(true) => Ok(()),
            Ok(false) => Err(StoreError::PendingAddBound),
            Err(e) => Err(self.note_failure(e)),
        }
    }

    pub(crate) fn pending_adds(&self) -> Result<Vec<EndpointId>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT endpoint_id FROM pending_adds ORDER BY rowid")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|row| {
            EndpointId::parse_derived(row?).map_err(|e| StoreError::Corrupt(e.to_string()))
        })
        .collect()
    }

    pub(crate) fn forget_pending_add(&mut self, endpoint: &EndpointId) -> Result<bool, StoreError> {
        let deleted = self
            .conn
            .execute(
                "DELETE FROM pending_adds WHERE endpoint_id = ?1",
                params![endpoint.as_str()],
            )
            .map_err(|e| self.note_failure(e))?;
        Ok(deleted != 0)
    }
}
