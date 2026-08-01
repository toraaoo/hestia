//! The cross-subsystem flows, driven against fixture providers. What these pin
//! is ordering: which step runs before which, and what is left on disk when one
//! of them fails.

mod fixture;

use engine::{Cancel, Job, ServerCreateSpec};
use proto::error::ErrorInfo;
use proto::minecraft::ProvisionProgress;

fn job<'a>(cancel: &'a Cancel, report: &'a (dyn Fn(&ProvisionProgress) + Send + Sync)) -> Job<'a> {
    Job::new(report, cancel)
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
