mod helpers;
mod lookup;
mod test_case;
mod todo;
mod user;

use crate::{
    helpers::load_app_settings_for_testing,
    test_case::{EnableTracing, InsertTestData, TestCase},
};

#[tokio::test]
#[ignore]
async fn health_check() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let uri = format!("{}/health-check", test_case.origin());
    let response = test_case.http_client.get(&uri).send().await.unwrap();
    assert!(
        response.status().is_success(),
        "Health check failed: {}",
        response.status()
    );
    assert!(
        response
            .text()
            .await
            .unwrap()
            .contains("Ok, the server is running!"),
        "Health check response did not contain 'Ok, the server is running!'"
    );

    test_case.end().await;
}
