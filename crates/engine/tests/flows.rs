//! The cross-subsystem flows, driven against fixture providers. What these pin
//! is ordering: which step runs before which, and what is left on disk when one
//! of them fails.

mod fixture;

use engine::{Cancel, EntryRef, Job, ServerCreateSpec};
use proto::content::{ContentAddItem, ContentAddSpec, ContentKind};
use proto::error::ErrorInfo;
use proto::minecraft::ProvisionProgress;

fn job<'a>(cancel: &'a Cancel, report: &'a (dyn Fn(&ProvisionProgress) + Send + Sync)) -> Job<'a> {
    Job::new(report, cancel)
}

fn add(projects: &[&str]) -> ContentAddSpec {
    ContentAddSpec {
        kind: ContentKind::Mod,
        items: projects
            .iter()
            .map(|p| ContentAddItem {
                project: (*p).to_string(),
                ..ContentAddItem::default()
            })
            .collect(),
        ..ContentAddSpec::default()
    }
}

fn spec(flavor: &str) -> ServerCreateSpec {
    ServerCreateSpec {
        name: "smp".into(),
        flavor: flavor.into(),
        version: "1.21".into(),
        eula: true,
        ..ServerCreateSpec::default()
    }
}

#[tokio::test]
async fn the_registry_the_engine_was_built_over_is_the_one_it_answers_from() {
    let home = tempfile::tempdir().expect("temp home");
    let engine = fixture::engine(
        home.path(),
        vec![fixture::Flavor::resolving("fixture")],
        vec![],
    );

    let flavors = engine.minecraft().server_flavors();
    assert_eq!(flavors.len(), 1, "only the injected flavor is offered");
    assert_eq!(flavors[0].id, "fixture");
    assert!(
        !flavors.iter().any(|f| f.id == "vanilla"),
        "the shipped registry must not leak into an engine built over another"
    );

    let versions = engine
        .minecraft()
        .server_versions("fixture")
        .await
        .expect("the fixture catalogue");
    assert_eq!(versions.len(), 2);
}

#[tokio::test]
async fn a_create_that_cannot_resolve_registers_nothing() {
    let home = tempfile::tempdir().expect("temp home");
    let engine = fixture::engine(
        home.path(),
        vec![fixture::Flavor::failing("fixture")],
        vec![],
    );
    let (cancel, report) = (Cancel::new(), |_: &ProvisionProgress| {});

    engine
        .provision_server(spec("fixture"), &job(&cancel, &report))
        .await
        .expect_err("the flavor cannot resolve");

    assert!(
        engine.servers().list().is_empty(),
        "resolve runs before the record exists, so a failure leaves no port claim behind"
    );
}

#[tokio::test]
async fn an_unknown_flavor_is_refused_by_name() {
    let home = tempfile::tempdir().expect("temp home");
    let engine = fixture::engine(
        home.path(),
        vec![fixture::Flavor::resolving("fixture")],
        vec![],
    );
    let (cancel, report) = (Cancel::new(), |_: &ProvisionProgress| {});

    let refused = engine
        .provision_server(spec("nosuch"), &job(&cancel, &report))
        .await
        .expect_err("no such flavor");
    assert!(
        matches!(engine::error_info(refused), ErrorInfo::Internal { .. }),
        "an unknown flavor is refused, not resolved against the first one"
    );
    assert!(engine.servers().list().is_empty());
}

#[tokio::test]
async fn a_cancelled_create_stops_before_it_claims_anything() {
    let home = tempfile::tempdir().expect("temp home");
    let engine = fixture::engine(
        home.path(),
        vec![fixture::Flavor::resolving("fixture")],
        vec![],
    );
    let cancel = Cancel::new();
    cancel.cancel();
    let report = |_: &ProvisionProgress| {};

    let stopped = engine
        .provision_server(spec("fixture"), &job(&cancel, &report))
        .await
        .expect_err("cancelled");

    assert!(
        engine::is_cancelled(&stopped),
        "a cancellation is not a failure"
    );
    assert!(
        engine.servers().list().is_empty(),
        "the checkpoint sits between resolve and create, so nothing is registered"
    );
}

/// An instance to install content into, built over the fixture flavor.
async fn instance(home: &std::path::Path, source: fixture::Source) -> (engine::Engine, String) {
    let engine = fixture::engine(
        home,
        vec![fixture::Flavor::resolving("fabric")],
        vec![source],
    );
    let record = engine
        .create_instance("modded", "fabric", "1.21", None, &[])
        .await
        .expect("create the instance");
    (engine, record.id)
}

#[tokio::test]
async fn a_batch_installs_its_roots_and_their_required_dependencies_once() {
    let home = tempfile::tempdir().expect("temp home");
    let files = fixture::Files::serve().await;
    let source = fixture::Source {
        projects: vec![
            fixture::project("sodium", ContentKind::Mod),
            fixture::project("iris", ContentKind::Mod),
            fixture::project("fabric-api", ContentKind::Mod),
        ],
        versions: vec![
            fixture::version("sodium", &files, "1.21", &["fabric-api"]),
            fixture::version("iris", &files, "1.21", &["fabric-api"]),
            fixture::version("fabric-api", &files, "1.21", &[]),
        ],
    };
    let (engine, id) = instance(home.path(), source).await;
    let (cancel, report) = (Cancel::new(), |_: &ProvisionProgress| {});

    let (installed, failures) = engine
        .add_entry_content(
            EntryRef::Instance(&id),
            &add(&["sodium", "iris"]),
            &job(&cancel, &report),
        )
        .await
        .expect("the batch runs");

    assert!(failures.is_empty(), "no item should fail: {failures:?}");
    let names: Vec<&str> = installed.iter().map(|i| i.project_id.as_str()).collect();
    assert_eq!(
        names.iter().filter(|n| **n == "fabric-api").count(),
        1,
        "a dependency shared across the batch installs once, got {names:?}"
    );
    assert_eq!(installed.len(), 3, "two roots plus the shared dependency");
}

#[tokio::test]
async fn a_root_that_fails_does_not_stop_the_batch() {
    let home = tempfile::tempdir().expect("temp home");
    let files = fixture::Files::serve().await;
    let source = fixture::Source {
        projects: vec![fixture::project("sodium", ContentKind::Mod)],
        versions: vec![fixture::version("sodium", &files, "1.21", &[])],
    };
    let (engine, id) = instance(home.path(), source).await;
    let (cancel, report) = (Cancel::new(), |_: &ProvisionProgress| {});

    let (installed, failures) = engine
        .add_entry_content(
            EntryRef::Instance(&id),
            &add(&["ghost", "sodium"]),
            &job(&cancel, &report),
        )
        .await
        .expect("the batch runs past the failure");

    assert_eq!(installed.len(), 1, "the good root still installs");
    assert_eq!(failures.len(), 1, "the unknown project is reported, once");
    assert_eq!(failures[0].item, "ghost");
}

#[tokio::test]
async fn what_a_batch_installed_is_what_the_entry_then_lists() {
    let home = tempfile::tempdir().expect("temp home");
    let files = fixture::Files::serve().await;
    let source = fixture::Source {
        projects: vec![fixture::project("sodium", ContentKind::Mod)],
        versions: vec![fixture::version("sodium", &files, "1.21", &[])],
    };
    let (engine, id) = instance(home.path(), source).await;
    let (cancel, report) = (Cancel::new(), |_: &ProvisionProgress| {});

    engine
        .add_entry_content(
            EntryRef::Instance(&id),
            &add(&["sodium"]),
            &job(&cancel, &report),
        )
        .await
        .expect("install");

    let (listed, _) = engine
        .entry_content(EntryRef::Instance(&id), ContentKind::Mod)
        .expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].project_id, "sodium");
    assert!(listed[0].enabled);

    // The index and the disk agree: removing it takes the file with it.
    assert!(engine
        .remove_entry_content(EntryRef::Instance(&id), ContentKind::Mod, "sodium", &[])
        .expect("remove"));
    let (after, _) = engine
        .entry_content(EntryRef::Instance(&id), ContentKind::Mod)
        .expect("list");
    assert!(after.is_empty(), "the index follows the removal");
}

#[tokio::test]
async fn disabling_an_item_keeps_it_installed() {
    let home = tempfile::tempdir().expect("temp home");
    let files = fixture::Files::serve().await;
    let source = fixture::Source {
        projects: vec![fixture::project("sodium", ContentKind::Mod)],
        versions: vec![fixture::version("sodium", &files, "1.21", &[])],
    };
    let (engine, id) = instance(home.path(), source).await;
    let (cancel, report) = (Cancel::new(), |_: &ProvisionProgress| {});

    engine
        .add_entry_content(
            EntryRef::Instance(&id),
            &add(&["sodium"]),
            &job(&cancel, &report),
        )
        .await
        .expect("install");

    let matched = engine
        .enable_entry_content(
            EntryRef::Instance(&id),
            ContentKind::Mod,
            "sodium",
            false,
            &[],
        )
        .expect("disable");
    assert_eq!(matched, 1);

    let (listed, _) = engine
        .entry_content(EntryRef::Instance(&id), ContentKind::Mod)
        .expect("list");
    assert_eq!(listed.len(), 1, "a disabled item is still installed");
    assert!(!listed[0].enabled);
}
