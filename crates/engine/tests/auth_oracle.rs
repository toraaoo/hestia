//! Real-endpoint auth oracle. `#[ignore]` by default because it reaches
//! Microsoft's live device-code endpoint and needs network; run it deliberately
//! to validate the sign-in flow against production:
//!
//!   cargo test -p engine --test auth_oracle -- --ignored --nocapture
//!
//! It exercises only the pre-consent leg (requesting a device code), so it
//! completes without a human approving anything — the part that can regress
//! silently (endpoint, request shape, response parsing) without a browser step.

use engine::Accounts;
use proto::accounts::LoginMethod;

#[tokio::test]
#[ignore = "hits live Microsoft endpoints; run with --ignored"]
async fn device_code_begin_returns_a_user_code() {
    let dir = std::env::temp_dir().join(format!("hestia-auth-oracle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let accounts = Accounts::new(dir.join("accounts.json"));

    let challenge = accounts
        .begin_login(LoginMethod::DeviceCode)
        .await
        .expect("device-code begin should reach Microsoft and return a challenge");

    assert!(
        !challenge.user_code.is_empty(),
        "the device-code flow must surface a user code"
    );
    assert!(
        !challenge.verification_uri.is_empty(),
        "the device-code flow must surface a verification URI"
    );
    eprintln!(
        "enter code {} at {}",
        challenge.user_code, challenge.verification_uri
    );

    std::fs::remove_dir_all(&dir).ok();
}
