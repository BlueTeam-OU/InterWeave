// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The windowed start's failure paths (design/2026-10-27-start-window.md
//! section 10), headless: the real view on Slint's testing backend, the
//! facade on its own thread over a real store whose reads the app-core
//! test hooks fail, hold or slow. What it does NOT prove: a screen
//! reader's speech (announcements are read from the view's live
//! regions), a real window's focus, a real daemon -- the desktop
//! end-to-end suite's.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::ElementHandle;
use interweave_human_app_core::test_hooks::{Fault, StoreFaults};
use interweave_human_app_core::{FacadeSide, Opener};
use interweave_human_desktop::app::{App, SlintSurface};
use interweave_human_desktop::facade_thread::FacadeThread;
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{ConversationKey, RuntimeHost, UiText, placeholder_en};
use interweave_human_ui_slint::View;
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::ComponentHandle as _;

type Desktop = App<SlintSurface, NoLinks>;

fn human() -> EndpointId {
    EndpointId::parse("human").expect("endpoint")
}

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer")
}

fn node() -> FakeConfig {
    FakeConfig {
        peer: peer(),
        endpoints: vec![FakeEndpoint::open(human(), false)],
        default_endpoint: Some(human()),
        queue_bound: 16,
    }
}

fn direct(from: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: from.clone(),
        endpoint: Some(human()),
    }
}

struct NoLinks;

impl Opener for NoLinks {
    fn open(&mut self, _destination: &str) {
        panic!("no link is activated in these tests");
    }

    fn ask_network_access(&mut self) {
        panic!("the desktop root never reports network access withheld");
    }

    fn authenticate(&mut self) {
        panic!("the desktop root never reports authentication needed");
    }
}

/// A store holding `count` unread text rows from `from`, the newest last.
fn flooded(from: &TransportIdentity, count: u32) -> HumanStore {
    let mut store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
    flood(&mut store, from, 1..=count);
    store
}

/// Unread text rows `rows` from `from` committed to `store`, each row's
/// number its id, its text and its time.
fn flood(store: &mut HumanStore, from: &TransportIdentity, rows: std::ops::RangeInclusive<u32>) {
    use interweave_human_store::{AppMessageId, InboundKind, InboundOrigin, NewInbound};
    use interweave_transport_api::MediaType;
    for n in rows {
        let id = format!("{n:032x}");
        store
            .commit_unread_inbound(&NewInbound {
                app_message_id: AppMessageId::parse(id.clone()).expect("an id"),
                origin: InboundOrigin {
                    peer: from.clone(),
                    endpoint: Some(human()),
                    channel: None,
                },
                media_type: Some(
                    MediaType::parse("application/vnd.interweave-human-chat+json;v=2")
                        .expect("media"),
                ),
                payload: format!(
                    r#"{{"v":2,"kind":"text","app_message_id":"{id}","text":"message {n}"}}"#
                )
                .into_bytes(),
                received_at: u64::from(n),
                kind: InboundKind::Text,
            })
            .expect("committed");
    }
}

/// The desktop over `store`, its reads under `faults`, its window shown
/// at a wide size and its clock the returned one, which the test steps.
fn desktop(store: HumanStore, faults: &Arc<StoreFaults>) -> (Desktop, Rc<Cell<Instant>>) {
    let (a, _b) = FakeNetwork::pair(node(), node());
    let faults = Arc::clone(faults);
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Daemon).expect("a window");
    let at = Rc::new(Cell::new(Instant::now()));
    let read = Rc::clone(&at);
    view.set_monotonic(move || read.get());
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 600.0));
    view.window().show().expect("shown");
    let thread = FacadeThread::spawn(
        move || {
            let client = TransportClient::new(
                a.clone(),
                a,
                store,
                ClientConfig {
                    client_kind: "human-client".to_owned(),
                    endpoints: vec![human()],
                    channels: vec![],
                    max_payload_bytes: 49_152,
                },
                Box::new(|| 1_786_600_000_000),
                0,
            )
            .expect("facade");
            Ok(FacadeSide::new(client, || 1_786_600_000_000).with_store_faults(faults))
        },
        || Ok(true),
        Arc::new(|| {}),
        |_| {},
    )
    .expect("the facade thread");
    (App::new(SlintSurface(view), NoLinks, thread), at)
}

fn view(app: &Desktop) -> &View {
    &app.side().surface().0
}

/// A few frames of the toolkit's work, each frame's layout forced by
/// reading where the elements stand -- the testing backend draws nothing,
/// and the list reports what is in view only once laid out -- then one
/// turn of the app.
fn turn(app: &mut Desktop) {
    for _ in 0..2 {
        slint::platform::update_timers_and_animations();
        if let Some(first) = all(view(app)).first() {
            let _ = first.absolute_position();
        }
    }
    app.pump();
}

/// Turn until `done`, or fail after a while.
fn until(app: &mut Desktop, what: &str, done: impl Fn(&Desktop) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(app) {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        turn(app);
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Turn for `wait` of real time: what a test needs to say nothing came.
fn turn_for(app: &mut Desktop, wait: Duration) {
    let end = Instant::now() + wait;
    while Instant::now() < end {
        turn(app);
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

fn all(view: &View) -> Vec<ElementHandle> {
    i_slint_backend_testing::ElementQuery::from_root(view.window()).find_all()
}

fn labelled(app: &Desktop, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(view(app).window(), label).collect()
}

fn announced(app: &Desktop) -> (String, String) {
    let window = view(app).window();
    (
        window.get_announcement_a().to_string(),
        window.get_announcement_b().to_string(),
    )
}

fn held(app: &Desktop, key: &ConversationKey) -> usize {
    app.side().model().messages(key).len()
}

/// Shift+Tab until focus is on `key`, as a person moves it.
fn backtab_to(app: &Desktop, key: &str) -> bool {
    let back: slint::SharedString = slint::platform::Key::Backtab.into();
    let window = view(app).window();
    for _ in 0..2_000 {
        if window.get_focus_id() == key {
            return true;
        }
        window
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: back.clone() });
        window
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: back.clone() });
    }
    false
}

/// A conversation of 200 unread rows opened, its first page held.
fn opened(app: &mut Desktop, from: &TransportIdentity) -> ConversationKey {
    let key = direct(from);
    until(app, "the conversation's summary", |app| {
        app.side()
            .model()
            .conversations()
            .iter()
            .any(|c| c.key == key)
    });
    app.side_mut().surface_mut().0.select(key.clone());
    until(app, "the first page", |app| held(app, &key) > 0);
    key
}

/// Tab once, as a person moves focus on.
fn tab(app: &Desktop) {
    let next: slint::SharedString = slint::platform::Key::Tab.into();
    let window = view(app).window().window();
    window.dispatch_event(slint::platform::WindowEvent::KeyPressed { text: next.clone() });
    window.dispatch_event(slint::platform::WindowEvent::KeyReleased { text: next });
}

fn focus(app: &Desktop) -> String {
    view(app).window().get_focus_id().to_string()
}

/// Focus moved onto the top's line by the person. The app placed focus
/// there as the conversation opened, and on the testing backend that
/// request does not land; focus arriving there is then the request
/// landing, not the person's move (`window_marks.rs`,
/// `focus_the_app_placed_is_the_persons_once_it_moved`), so focus goes on
/// and comes back.
fn focus_line(app: &mut Desktop) {
    let line = "earlier-unread:t";
    assert!(backtab_to(app, line), "focus on the top's line");
    turn(app);
    tab(app);
    turn(app);
    assert!(backtab_to(app, line), "and on it again");
}

/// Section 10 items 3b and 7, through the facade thread and a store whose
/// page reads a test hook holds or fails, the top reached by moving focus
/// onto its line (the keyboard's way there, section 8): the next page
/// held shows no progress row at once and shows it at 250 ms; the read
/// failing then puts the failure row in its place, announced once; the
/// top reached again retries nothing; Try again, the fault lifted, loads
/// the page with focus on what was below the row, the line. Mutation:
/// the progress gate taken away (shown at once). The view's own rules --
/// announced once, a failed edge's reach asking nothing -- are broken in
/// ui-slint's and ui-model's tests; here they hold over the real facade.
#[test]
fn a_held_then_failing_page_read_shows_progress_then_failure_and_try_again_loads_it() {
    let from = peer();
    let faults = Arc::new(StoreFaults::default());
    let (mut app, at) = desktop(flooded(&from, 200), &faults);
    let key = opened(&mut app, &from);
    let first = held(&app, &key);
    let loading = text(UiText::EarlierLoading);
    let failed = text(UiText::EarlierLoadFailed);
    let line = "earlier-unread:t";

    faults.set_pages(Fault::Hold);
    focus_line(&mut app);
    until(&mut app, "the top's page on its way", |app| {
        app.side()
            .model()
            .window_marks(&key)
            .iter()
            .any(|m| matches!(m.kind, interweave_human_ui_model::MarkKind::Loading(_)))
    });
    assert!(
        labelled(&app, loading).is_empty(),
        "no progress row at once"
    );
    at.set(at.get() + Duration::from_millis(250));
    turn(&mut app);
    assert_eq!(
        labelled(&app, loading).len(),
        1,
        "the progress row at 250 ms"
    );

    faults.set_pages(Fault::Fail);
    until(&mut app, "the failure row", |app| {
        !labelled(app, failed).is_empty()
    });
    assert!(
        labelled(&app, loading).is_empty(),
        "in the progress row's place"
    );
    let said = announced(&app);
    assert!(
        said.0.contains(failed) || said.1.contains(failed),
        "announced: {said:?}"
    );
    turn(&mut app);
    turn(&mut app);
    assert_eq!(announced(&app), said, "once, not on each turn");
    assert_eq!(held(&app, &key), first, "nothing read");

    // A reach of the failed edge asks nothing: with the fault lifted, a
    // read would load the page.
    faults.set_pages(Fault::Pass);
    tab(&app);
    turn(&mut app);
    assert_ne!(focus(&app), line, "focus moved off the line");
    assert!(backtab_to(&app, line), "and back onto it");
    turn_for(&mut app, Duration::from_millis(300));
    assert_eq!(
        held(&app, &key),
        first,
        "the top reached again reads nothing"
    );
    assert_eq!(labelled(&app, failed).len(), 1, "the failure row stays");

    assert!(backtab_to(&app, "mark-action:t"), "Try again takes focus");
    labelled(&app, text(UiText::TryAgain))[0].invoke_accessible_default_action();
    until(&mut app, "the page Try again reads", |app| {
        held(app, &key) > first
    });
    turn(&mut app);
    assert_eq!(
        focus(&app),
        line,
        "on what was below the failure row: the line, with rows still above"
    );
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}

fn shows(app: &Desktop, label: &str) -> bool {
    !labelled(app, label).is_empty()
}

fn press(app: &mut Desktop, label: &str) {
    let found = labelled(app, label);
    assert!(!found.is_empty(), "a control labelled {label:?}");
    found[0].invoke_accessible_default_action();
    turn(app);
}

/// Whether any element on screen is labelled with `name` in it.
fn names(app: &Desktop, name: &str) -> bool {
    all(view(app))
        .iter()
        .any(|e| e.accessible_label().is_some_and(|l| l.contains(name)))
}

/// A store holding one unread row from `from` and a contact "Anna Rossi"
/// on that route.
fn with_contact(from: &TransportIdentity) -> HumanStore {
    let mut store = flooded(from, 1);
    let _ = add_contact(&mut store, from, ("Anna", "Rossi"), None);
    store
}

/// A contact named `name` on `from`'s route, with `photo` as its photo;
/// its id.
fn add_contact(
    store: &mut HumanStore,
    from: &TransportIdentity,
    name: (&str, &str),
    photo: Option<&[u8]>,
) -> String {
    store
        .accept_card(
            &interweave_human_store::AcceptedCard {
                peer: from,
                endpoint: &human(),
                name: name.0,
                surname: Some(name.1),
                identity_key: [7; 32],
                photo,
            },
            1,
        )
        .expect("a contact")
        .contact_id
}

/// A photo as the client stores one: an 8 x 8 image, prepared.
fn photo() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(8, 8, image::Rgba([200, 40, 40, 255]));
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("a PNG");
    interweave_human_app_core::card_photo::prepare_photo(&png)
        .expect("prepared")
        .bytes
}

/// Section 10 item 11's failure path, through the facade thread and a
/// store whose contact reads a test hook fails or holds: the contacts
/// failed at start leave Chats' title without the contact's name and
/// show Contacts' failure row alone, nothing said on opening; Try again
/// with the read held takes the row away and focus to the page title,
/// the progress row at 250 ms; failing again, focus on Try again and the
/// failure announced; the read let through, the list with focus on its
/// first row and the name on Chats. Mutation: the start's contacts read
/// past the hook.
#[test]
fn contacts_failed_at_start_recover_through_try_again() {
    let from = peer();
    let faults = Arc::new(StoreFaults::default());
    faults.set_contacts(Fault::Fail);
    let (mut app, at) = desktop(with_contact(&from), &faults);
    let key = direct(&from);
    until(&mut app, "the conversation's summary", |app| {
        app.side()
            .model()
            .conversations()
            .iter()
            .any(|c| c.key == key)
    });
    turn(&mut app);
    assert!(!names(&app, "Anna Rossi"), "no contact's name on Chats");

    press(&mut app, text(UiText::AppOptions));
    press(&mut app, text(UiText::ContactsTitle));
    let failed = text(UiText::ContactsLoadFailed);
    assert!(shows(&app, failed), "the failure row");
    assert!(!shows(&app, text(UiText::ContactsEmpty)), "no empty claim");
    let said = |app: &Desktop| {
        let (a, b) = announced(app);
        a.contains(failed) || b.contains(failed)
    };
    assert!(!said(&app), "opened already failed: nothing said");

    faults.set_contacts(Fault::Hold);
    press(&mut app, text(UiText::TryAgain));
    turn(&mut app);
    assert!(!shows(&app, failed), "the row goes at once");
    assert_eq!(focus(&app), "cards-title", "focus on the page title");
    let loading = text(UiText::ContactsLoading);
    assert!(!shows(&app, loading), "no progress row at once");
    at.set(at.get() + Duration::from_millis(250));
    turn(&mut app);
    assert!(shows(&app, loading), "the progress row at 250 ms");

    faults.set_contacts(Fault::Fail);
    until(&mut app, "the second failure", |app| shows(app, failed));
    turn(&mut app);
    assert!(said(&app), "a failed Try again is announced");
    assert_eq!(focus(&app), "contacts-try-again");

    faults.set_contacts(Fault::Pass);
    press(&mut app, text(UiText::TryAgain));
    until(&mut app, "the contacts", |app| names(app, "Anna Rossi"));
    turn(&mut app);
    assert!(
        focus(&app).starts_with("contact:"),
        "on the first row: {}",
        focus(&app)
    );
    assert!(view(&app).back(), "Back to Chats");
    turn(&mut app);
    assert!(
        names(&app, "Anna Rossi"),
        "Chats' title is the contact's name"
    );
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}

/// Item 11's success with no contact stored: `ContactsEmpty`, with focus
/// on it, through the real facade (ui-slint's cards tests break the
/// focus rule).
#[test]
fn contacts_read_after_a_failure_with_none_stored_say_so_with_focus() {
    let from = peer();
    let faults = Arc::new(StoreFaults::default());
    faults.set_contacts(Fault::Fail);
    let (mut app, _at) = desktop(flooded(&from, 1), &faults);
    until(&mut app, "the start", |app| {
        !app.side().model().conversations().is_empty()
    });
    press(&mut app, text(UiText::AppOptions));
    press(&mut app, text(UiText::ContactsTitle));
    assert!(shows(&app, text(UiText::ContactsLoadFailed)));
    faults.set_contacts(Fault::Pass);
    press(&mut app, text(UiText::TryAgain));
    until(&mut app, "the empty list", |app| {
        shows(app, text(UiText::ContactsEmpty))
    });
    turn(&mut app);
    assert_eq!(focus(&app), "contacts-empty");
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}

fn at_limit(app: &Desktop, key: &ConversationKey) -> bool {
    app.side()
        .model()
        .window_marks(key)
        .iter()
        .any(|m| matches!(m.kind, interweave_human_ui_model::MarkKind::AtLimit(_)))
}

/// Section 10 item 12 at a test bound of one page, through the facade
/// thread: the top reached with the session full shows the limit row and
/// reads nothing; Show more messages, pressed with focus on it, loads the
/// page with focus on what was below the row, and paging then goes on
/// without asking; A closed and opened again, reaching the top shows the
/// limit row again. Mutation: the test bound not taken (no limit row).
#[test]
fn at_the_session_bound_the_limit_row_asks_and_asks_again_after_a_reopen() {
    use interweave_human_ui_model::{HELD_ROW_CAP, READ_PROPOSAL_CAP};
    let (from, other) = (peer(), peer());
    let mut store = flooded(&from, 200);
    flood(&mut store, &other, 201..=201);
    let faults = Arc::new(StoreFaults::default());
    let (mut app, _at) = desktop(store, &faults);
    app.side_mut()
        .set_bounds_for_test(64, HELD_ROW_CAP, READ_PROPOSAL_CAP);
    let key = opened(&mut app, &from);
    let first = held(&app, &key);
    assert_eq!(first, 64, "one page fills the bound");
    focus_line(&mut app);
    until(&mut app, "the limit row", |app| at_limit(app, &key));
    turn_for(&mut app, Duration::from_millis(300));
    assert_eq!(held(&app, &key), first, "nothing read at the limit");
    assert!(shows(&app, text(UiText::EarlierShowAnyway)));

    assert!(
        backtab_to(&app, "mark-action:t"),
        "Show more messages takes focus"
    );
    labelled(&app, text(UiText::EarlierShowAnyway))[0].invoke_accessible_default_action();
    until(&mut app, "the page asked for", |app| {
        held(app, &key) > first
    });
    turn(&mut app);
    assert_eq!(focus(&app), "earlier-unread:t", "on what was below the row");
    let second = held(&app, &key);
    focus_line(&mut app);
    until(&mut app, "the next page, unasked", |app| {
        held(app, &key) > second
    });
    assert!(!at_limit(&app, &key), "no limit row while A stays open");

    app.side_mut().surface_mut().0.select(direct(&other));
    until(&mut app, "B open", |app| held(app, &direct(&other)) > 0);
    app.side_mut().surface_mut().0.select(key.clone());
    turn(&mut app);
    focus_line(&mut app);
    until(&mut app, "the limit row again", |app| at_limit(app, &key));
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}

/// The images drawn at a size: a photo, since initials are text.
fn images(app: &Desktop) -> usize {
    ElementHandle::find_by_element_type_name(view(app).window(), "Image")
        .filter(|e| {
            let size = e.size();
            size.width > 0.0 && size.height > 0.0
        })
        .count()
}

/// Section 10 item 15, photo reads slowed by a test hook: the Contacts
/// row shows the initials, then the photo in their place, nothing else
/// moving; a photo that cannot be read leaves the initials, and that
/// contact's page opens on them with Remove photo, which removes the
/// stored photo. Mutations: the delay not taken (the photo at once); an
/// unreadable photo read as readable (drawn).
#[test]
fn photos_come_in_place_of_initials_and_an_unreadable_one_keeps_remove_photo() {
    let (anna, bruno) = (peer(), peer());
    let mut store = flooded(&anna, 1);
    let bytes = photo();
    let _ = add_contact(&mut store, &anna, ("Anna", "Rossi"), Some(&bytes));
    let unreadable = add_contact(&mut store, &bruno, ("Bruno", "Bianchi"), Some(&bytes));
    let faults = Arc::new(StoreFaults::default());
    faults.set_photo_delay(Duration::from_millis(400));
    faults.set_unreadable_photo(&unreadable);
    let (mut app, _at) = desktop(store, &faults);
    until(&mut app, "the start", |app| {
        !app.side().model().conversations().is_empty()
    });
    press(&mut app, text(UiText::AppOptions));
    press(&mut app, text(UiText::ContactsTitle));
    let name = interweave_human_ui_model::isolated("Anna Rossi");
    until(&mut app, "the contacts", |app| shows(app, &name));
    // Asked as the row shows, and read in 400 ms: still the initials half
    // way through.
    turn_for(&mut app, Duration::from_millis(200));
    assert_eq!(images(&app), 0, "the initials while the photo is read");
    let name_at = labelled(&app, &name)[0].absolute_position();
    until(&mut app, "the photo", |app| images(app) == 1);
    assert_eq!(
        labelled(&app, &name)[0].absolute_position(),
        name_at,
        "nothing else moves"
    );
    turn_for(&mut app, Duration::from_millis(600));
    assert_eq!(
        images(&app),
        1,
        "Bruno's unreadable photo leaves his initials"
    );

    press(
        &mut app,
        &interweave_human_ui_model::isolated("Bruno Bianchi"),
    );
    until(&mut app, "Bruno's page", |app| {
        view(app).window().get_cards_page() == interweave_human_ui_slint::CardsPage::Contact
    });
    assert!(
        !view(&app).window().get_contact_photo_shown(),
        "on the initials"
    );
    press(&mut app, text(UiText::RemovePhoto));
    // Confirmed once the store has written the removal (the facade's
    // change, not the press).
    until(&mut app, "the photo removed", |app| {
        shows(app, text(UiText::ConfirmPhotoRemoved))
    });
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}

/// The numbers of A's rows held, in the order drawn.
fn numbers(app: &Desktop, key: &ConversationKey) -> Vec<u32> {
    app.side()
        .model()
        .messages(key)
        .iter()
        .map(|m| {
            m.source
                .strip_prefix("message ")
                .and_then(|n| n.parse().ok())
                .expect("a numbered row")
        })
        .collect()
}

fn gap_marks(app: &Desktop, key: &ConversationKey) -> Vec<interweave_human_ui_model::MarkKind> {
    use interweave_human_ui_model::{Edge, MarkKind};
    app.side()
        .model()
        .window_marks(key)
        .into_iter()
        .map(|m| m.kind)
        .filter(|k| {
            matches!(
                k,
                MarkKind::EarlierUnread {
                    edge: Edge::GapUpper(_) | Edge::GapLower(_),
                    ..
                } | MarkKind::Failed(Edge::GapUpper(_) | Edge::GapLower(_))
                    | MarkKind::Loading(Edge::GapUpper(_) | Edge::GapLower(_))
            )
        })
        .collect()
}

/// Tab, a turn after each, until `done` or `max` presses.
fn tab_until(app: &mut Desktop, max: usize, done: impl Fn(&Desktop) -> bool) -> bool {
    for _ in 0..max {
        if done(app) {
            return true;
        }
        tab(app);
        turn(app);
    }
    done(app)
}

/// Section 10 item 14 at a test ceiling of 80 rows, through the facade
/// thread: paging up past the ceiling lets rows go and leaves a gap
/// whose line counts them, the newest row held; moving focus down onto
/// the gap with its read failing shows the failure row, announced once;
/// lifted, its Try again loads the gap's rows, and once it is empty the
/// rows are in order, none missing and none twice. Mutation: the test
/// ceiling not taken (no gap).
#[test]
fn past_the_held_ceiling_a_gap_forms_and_fills_in_order_through_a_failure() {
    use interweave_human_ui_model::{READ_PROPOSAL_CAP, SESSION_ITEM_CAP};
    let from = peer();
    let faults = Arc::new(StoreFaults::default());
    let (mut app, _at) = desktop(flooded(&from, 200), &faults);
    app.side_mut()
        .set_bounds_for_test(SESSION_ITEM_CAP, 80, READ_PROPOSAL_CAP);
    let key = opened(&mut app, &from);
    focus_line(&mut app);
    until(&mut app, "a gap past the ceiling", |app| {
        !gap_marks(app, &key).is_empty()
    });
    let held_now = numbers(&app, &key);
    assert!(
        held_now.len() <= 80,
        "held within the ceiling: {}",
        held_now.len()
    );
    assert_eq!(held_now.last(), Some(&200), "the newest row held");

    faults.set_pages(Fault::Fail);
    let failed = |app: &Desktop| {
        gap_marks(app, &key)
            .iter()
            .any(|k| matches!(k, interweave_human_ui_model::MarkKind::Failed(_)))
    };
    assert!(
        tab_until(&mut app, 200, failed),
        "the gap reached by focus fails"
    );
    turn(&mut app);
    let said = announced(&app);
    let text_failed = text(UiText::EarlierLoadFailed);
    assert!(
        said.0.contains(text_failed) || said.1.contains(text_failed),
        "announced: {said:?}"
    );

    faults.set_pages(Fault::Pass);
    labelled(&app, text(UiText::TryAgain))[0].invoke_accessible_default_action();
    until(&mut app, "the gap's rows", |app| !failed(app));
    // On down through the gap until it holds nothing.
    assert!(
        tab_until(&mut app, 400, |app| gap_marks(app, &key).is_empty()),
        "the gap emptied: {:?}",
        gap_marks(&app, &key)
    );
    let shown = numbers(&app, &key);
    let mut sorted = shown.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(shown, sorted, "in order, none twice");
    let lowest = shown[0];
    assert_eq!(
        shown,
        (lowest..=200).collect::<Vec<_>>(),
        "none missing below the newest"
    );
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}
