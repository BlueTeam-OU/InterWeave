// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The one order contacts take, wherever the client orders them or their
//! names (human-client-ui.md section 3, "How contacts are ordered"): by
//! display name under Unicode collation (UTS #10, the CLDR root order),
//! then, among names that collate equal, by the smallest of the contact's
//! routes, its `PeerId` in canonical form and then its route label, each
//! by code point (contact-cards design section 6.2). The Contacts list,
//! a Peers row's names, a proposal's notes and the command line all read
//! it from here; a store query orders nothing a person sees.
//!
//! No locale reaches the client yet (the localisation contract,
//! human-client-ui.md section 11), so the collator is the root's and is
//! built from the root's data alone: a locale's tailoring is a
//! megabyte of data no build of today can ask for. When a locale arrives,
//! the provider below gives way to the compiled data's own.

use std::borrow::Cow;
use std::cmp::Ordering;

use icu_collator::provider::{
    Baked, CollationDiacriticsV1, CollationJamoV1, CollationMetadataV1, CollationReorderingV1,
    CollationRootV1, CollationSpecialPrimariesV1, CollationTailoringV1,
};
use icu_collator::{Collator, CollatorPreferences, options::CollatorOptions};
use icu_normalizer::provider::{NormalizerNfdDataV1, NormalizerNfdTablesV1};
use icu_provider::prelude::*;

use crate::{ContactRef, ContactView};

/// The root collation's data and nothing a locale adds. The root's
/// metadata says it is neither tailored nor reordered, so the collator
/// never asks for those two; answering them "not found" rather than
/// from [`Baked`] is what keeps the tailorings out of the binary.
/// `the_root_only_data_orders_as_the_compiled_root_does` holds it to the
/// compiled data's root.
struct RootOnly;

macro_rules! from_compiled {
    ($($marker:ty => $data:path),* $(,)?) => {$(
        impl DataProvider<$marker> for RootOnly {
            fn load(&self, req: DataRequest<'_>) -> Result<DataResponse<$marker>, DataError> {
                $data.load(req)
            }
        }
    )*};
}

from_compiled!(
    CollationRootV1 => Baked,
    CollationJamoV1 => Baked,
    CollationSpecialPrimariesV1 => Baked,
    CollationDiacriticsV1 => Baked,
    CollationMetadataV1 => Baked,
    NormalizerNfdDataV1 => icu_normalizer::provider::Baked,
    NormalizerNfdTablesV1 => icu_normalizer::provider::Baked,
);

macro_rules! not_root {
    ($($marker:ty),* $(,)?) => {$(
        impl DataProvider<$marker> for RootOnly {
            fn load(&self, req: DataRequest<'_>) -> Result<DataResponse<$marker>, DataError> {
                Err(DataErrorKind::IdentifierNotFound.with_req(<$marker>::INFO, req))
            }
        }
    )*};
}

not_root!(CollationTailoringV1, CollationReorderingV1);

fn root_collator() -> Collator {
    // The root's data is compiled in, so only a defect in it fails here,
    // and every test that orders a name would show it first.
    #[expect(
        clippy::expect_used,
        reason = "compiled data, checked by every ordering test"
    )]
    Collator::try_new_unstable(
        &RootOnly,
        CollatorPreferences::default(),
        CollatorOptions::default(),
    )
    .expect("the root collation's compiled data loads")
}

thread_local! {
    // Per thread: a collator's data is not `Sync` in this build, and the
    // client orders contacts on the thread that draws them.
    static COLLATOR: Collator = root_collator();
}

/// Two names in the root's collation: case a tertiary difference and an
/// accent a secondary one, so "ben" and "Ben" stand together and "Åke"
/// comes before "Zoe". Names that collate equal answer `Equal`, which a
/// caller breaks with the route tie.
#[must_use]
pub fn compare_names(a: &str, b: &str) -> Ordering {
    COLLATOR.with(|c| c.as_borrowed().compare(a, b))
}

/// Two contacts in the client's one order: by name, then by the smallest
/// route. Two contacts never share a route, so only two contacts with no
/// route at all reach the store's id, which keeps the order total.
#[must_use]
pub fn contact_order(a: &ContactView, b: &ContactView) -> Ordering {
    compare_names(&a.name, &b.name)
        .then_with(|| smallest_route(a).cmp(&smallest_route(b)))
        .then_with(|| a.contact_id.cmp(&b.contact_id))
}

/// Contact names in the one order: each ordered as the contact `contacts`
/// holds under its id, by [`contact_order`] itself, so a note can never
/// be ordered by another rule than the list; a name no contact holds is
/// ordered as a contact of that name with no route.
pub(crate) fn sort_refs(refs: &mut Vec<ContactRef>, contacts: &[ContactView]) {
    let mut keyed: Vec<(Cow<'_, ContactView>, ContactRef)> = refs
        .drain(..)
        .map(|r| {
            let view = contacts
                .iter()
                .find(|c| c.contact_id == r.contact_id)
                .map_or_else(
                    || {
                        Cow::Owned(ContactView {
                            contact_id: r.contact_id.clone(),
                            name: r.name.clone(),
                            name_from_card: false,
                            name_at: None,
                            photo_from_card: None,
                            photo_at: None,
                            photo: None,
                            routes: Vec::new(),
                        })
                    },
                    Cow::Borrowed,
                );
            (view, r)
        })
        .collect();
    keyed.sort_by(|(a, _), (b, _)| contact_order(a, b));
    refs.extend(keyed.into_iter().map(|(_, r)| r));
}

/// The smallest of a contact's routes, a route compared by its `PeerId`'s
/// canonical text and then its label, each by code point -- which, for
/// UTF-8, is the order of their bytes.
fn smallest_route(c: &ContactView) -> Option<(&str, &str)> {
    c.routes
        .iter()
        .map(|r| (r.peer.as_str(), r.endpoint.as_str()))
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RouteView;
    use interweave_transport_api::{EndpointId, TransportIdentity};

    // Two PeerIds, P before Q by code point ('D' < 'K' at the ninth).
    const P: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
    const Q: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";

    fn contact(id: &str, name: &str, routes: &[(&str, &str)]) -> ContactView {
        ContactView {
            contact_id: id.to_owned(),
            name: name.to_owned(),
            name_from_card: true,
            name_at: None,
            photo_from_card: None,
            photo_at: None,
            photo: None,
            routes: routes
                .iter()
                .map(|(p, e)| RouteView {
                    peer: TransportIdentity::parse(*p).expect("a PeerId"),
                    endpoint: EndpointId::parse(*e).expect("a label"),
                    fingerprint: None,
                })
                .collect(),
        }
    }

    fn names(list: &[ContactView]) -> Vec<&str> {
        list.iter().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn names_take_the_root_collation_order() {
        // Mutations: the lowercase-then-code-point order that stood in
        // before puts "Åke" after "Zoe"; byte order puts "ben" after
        // "Zoe" as well; a comparison ignoring case lets the input order
        // decide between "ben" and "Ben".
        let mut list: Vec<ContactView> = ["Zoe", "Ben", "Chloe", "Åke", "ben"]
            .iter()
            .zip([Q, P, Q, P, Q])
            .enumerate()
            .map(|(i, (n, p))| contact(&format!("c{i}"), n, &[(p, "human")]))
            .collect();
        list.sort_by(contact_order);
        assert_eq!(names(&list), ["Åke", "ben", "Ben", "Chloe", "Zoe"]);
        assert_eq!(
            compare_names("ben", "Ben"),
            Ordering::Less,
            "case is a tertiary difference"
        );
        assert_eq!(
            compare_names("Åke", "Ake"),
            Ordering::Greater,
            "an accent a secondary one"
        );
        assert_eq!(
            compare_names("Åke", "Ben"),
            Ordering::Less,
            "and never a primary one"
        );
    }

    #[test]
    fn names_that_collate_equal_go_by_the_smaller_route_peer_id_first() {
        // "Åke" composed and decomposed are one name to the collator, so
        // the routes decide. Mutations: the largest route instead of the
        // smallest (c3 first: c2's largest is Q too, and "a" < "work"); the
        // label before the PeerId (c3's "a" would win over c2's P); the store's id in place
        // of the route (c1 < c2 < c3 by id, here reversed on purpose).
        let composed = "\u{c5}ke";
        let decomposed = "A\u{30a}ke";
        assert_eq!(compare_names(composed, decomposed), Ordering::Equal);
        let mut list = [
            contact("c1", composed, &[(Q, "zed")]),
            contact("c2", decomposed, &[(Q, "work"), (P, "work")]),
            contact("c3", composed, &[(Q, "a")]),
        ];
        list.sort_by(contact_order);
        let ids: Vec<&str> = list.iter().map(|c| c.contact_id.as_str()).collect();
        assert_eq!(
            ids,
            ["c2", "c3", "c1"],
            "P before Q; then label 'a' before 'zed'"
        );
    }

    #[test]
    fn a_proposals_names_take_the_same_order_with_the_route_tie() {
        // Each note is ordered as the list orders its contact: the two
        // "Ada" by their routes, c9's P before c1's Q, against the order
        // of their ids; c7, renamed "Zoltan" since the standing was read,
        // where the list has it, after c5's "Zoe", which no contact holds.
        // Mutations: the refs by name alone (then by id) put c1 first; a
        // second comparator over the refs' own names puts c7 ("Åke")
        // before c5.
        let contacts = vec![
            contact("c1", "Ada", &[(Q, "human")]),
            contact("c9", "Ada", &[(P, "human")]),
            contact("c7", "Zoltan", &[(P, "z")]),
        ];
        let mut refs: Vec<ContactRef> = ["Zoe", "Ada", "Åke", "Ada"]
            .iter()
            .zip(["c5", "c9", "c7", "c1"])
            .map(|(n, id)| ContactRef {
                contact_id: id.to_owned(),
                name: (*n).to_owned(),
            })
            .collect();
        sort_refs(&mut refs, &contacts);
        let ids: Vec<&str> = refs.iter().map(|r| r.contact_id.as_str()).collect();
        assert_eq!(ids, ["c9", "c1", "c5", "c7"]);
    }

    #[test]
    fn the_root_only_data_orders_as_the_compiled_root_does() {
        // The provider above answers the root's own data and refuses a
        // tailoring; the compiled data's root collator is the reference.
        // Names from scripts some locale tailors (Swedish å, Spanish ñ,
        // German ß, Lithuanian y, Han, Hangul, Cyrillic, Arabic), so a
        // provider that served a tailoring would disagree somewhere.
        // Mutation: the compiled data's provider for every marker, so a
        // tailoring could be served, still agrees here (the root asks for
        // none); what this holds is that refusing the two markers changes
        // no order.
        let compiled =
            Collator::try_new(CollatorPreferences::default(), CollatorOptions::default())
                .expect("compiled data");
        let corpus = [
            "Åke",
            "Ake",
            "Zoe",
            "Ñandú",
            "Nadia",
            "Straße",
            "Strasse",
            "Yvonne",
            "Iveta",
            "李雷",
            "王芳",
            "김민준",
            "Ölçer",
            "Øyvind",
            "Ostap",
            "Юлия",
            "Jens",
            "ليلى",
            "ben",
            "Ben",
            "ch",
            "cz",
            "dž",
            "Ð",
            "Þóra",
            "",
            "1",
            "a1",
            "A",
        ];
        for a in corpus {
            for b in corpus {
                assert_eq!(
                    compare_names(a, b),
                    compiled.compare(a, b),
                    "{a:?} against {b:?}"
                );
            }
        }
    }
}
