use crate::helpers::TestCase;

mod helpers;

#[tokio::test]
async fn health_check() {
    let test_case = TestCase::begin(false).await;

    let uri = format!("{}/health-check", test_case.origin());
    let response = test_case.client.get(&uri).send().await.unwrap();
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
