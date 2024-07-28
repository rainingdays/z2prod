use crate::helpers::*;
#[tokio::test]
async fn health_check_works() {
    // spawn_app().await.expect("Failed to spawn our app.");
    let app = spawn_app().await;
    let url = format!("{}/health_check", app.address);
    let client = reqwest::ClientBuilder::new().no_proxy().build().unwrap();
    // let client = reqwest::Client::new();
    let response = client
        // .get("http://127.0.0.1:8000/health_check")
        .get(&url)
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    // assert_eq!(Some(0), response.content_length());
}
