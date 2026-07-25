use std::panic;
use std::path::PathBuf;

fn temp_home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hestia-crash-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_panic_writes_a_report_carrying_the_log_tail() {
    let home = temp_home();
    let file = common::FileLog::for_binary("test", Some(&home), common::LogLevel::Debug);
    let _guard = common::init_logging(common::LogLevel::Off, Some(file));

    tracing::info!(target: "common", "a line the report should carry");

    let result = panic::catch_unwind(|| panic!("deliberate test panic"));
    assert!(result.is_err(), "the panic must still propagate");

    let reports = common::crash::list();
    assert_eq!(reports.len(), 1, "expected exactly one report: {reports:?}");

    let report = common::crash::read(&reports[0]).expect("report is readable");
    assert!(report.contains("deliberate test panic"), "{report}");
    assert!(report.contains("kind:     panic"), "{report}");
    assert!(report.contains("crates/common/tests/crash.rs"), "{report}");
    assert!(
        report.contains("a line the report should carry"),
        "{report}"
    );

    common::crash::clear().expect("reports are removable");
    assert!(common::crash::list().is_empty());

    let _ = std::fs::remove_dir_all(&home);
}
