use deadpool_redis::redis::cmd;

use super::common::local_server::connect_local_server;

#[tokio::test]
async fn root_returns_ok() {
    let server = connect_local_server().await;
    let res = server
        .http
        .get(format!("{}/", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(
        (res.status().as_u16(), res.text().await.unwrap()),
        (200, String::from("OK"))
    );
}

#[tokio::test]
async fn database_is_reachable() {
    let server = connect_local_server().await;
    let one: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&server.db)
        .await
        .unwrap();
    assert_eq!(one, 1);
}

#[tokio::test]
async fn redis_is_reachable() {
    let server = connect_local_server().await;
    let mut conn = server.redis.get().await.unwrap();
    let pong: String = cmd("PING").query_async(&mut conn).await.unwrap();
    assert_eq!(pong, "PONG");
}
