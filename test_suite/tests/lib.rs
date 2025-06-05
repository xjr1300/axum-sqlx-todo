use crate::helpers::TestCase;

mod helpers;

#[tokio::test]
async fn run_test_skeleton() {
    // Initialize the test case
    let test_case = TestCase::begin(true).await;
    println!("Test application started on port: {}", test_case.port());

    /************************************************************

            Implement integration test logic here

    *************************************************************/

    // Next lines simulate a graceful shutdown, so real test logic should not be included next lines
    println!("Waiting for 3 seconds before sending graceful shutdown signal...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Terminate the test case gracefully
    test_case.end().await;
}
