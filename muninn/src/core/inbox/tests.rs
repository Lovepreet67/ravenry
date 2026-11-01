use std::time::Duration;

use sqlx::PgPool;

use super::{InboxStore, NewInboxMessage, PgInboxStore};

fn msg(id: &str) -> NewInboxMessage {
    NewInboxMessage::valid(
        "events",
        "s1",
        id,
        "t",
        Some(1),
        serde_json::json!({ "a": 1 }),
    )
}

const LEASE: Duration = Duration::from_secs(60);

#[sqlx::test(migrations = "./migrations")]
async fn duplicate_inserts_are_ignored(pool: PgPool) {
    let store = PgInboxStore::new(pool);
    assert_eq!(
        store.insert_batch(&[msg("1-0"), msg("2-0")]).await.unwrap(),
        2
    );
    // 1-0 is a redelivery, 3-0 is new, and the in-batch duplicate is ignored.
    assert_eq!(
        store
            .insert_batch(&[msg("1-0"), msg("3-0"), msg("3-0")])
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn malformed_messages_are_kept_as_quarantined(pool: PgPool) {
    let store = PgInboxStore::new(pool.clone());
    let m = NewInboxMessage::malformed("events", "s1", "9-0", b"not json".to_vec(), "invalid JSON");
    assert_eq!(store.insert_batch(&[m]).await.unwrap(), 1);

    let (status, raw): (String, Option<Vec<u8>>) =
        sqlx::query_as("SELECT status, raw_payload FROM inbox_messages")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "quarantined");
    assert_eq!(raw.unwrap(), b"not json");
    // Never claimable.
    assert!(store
        .claim_batch("events", "w", 10, LEASE)
        .await
        .unwrap()
        .is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_claims_never_overlap(pool: PgPool) {
    let store = PgInboxStore::new(pool);
    let batch: Vec<_> = (0..20).map(|i| msg(&format!("{i}-0"))).collect();
    store.insert_batch(&batch).await.unwrap();

    let (a, b) = tokio::join!(
        store.claim_batch("events", "w1", 20, LEASE),
        store.claim_batch("events", "w2", 20, LEASE),
    );
    let (a, b) = (a.unwrap(), b.unwrap());

    assert_eq!(a.len() + b.len(), 20);
    assert!(a.iter().all(|x| !b.iter().any(|y| y.id == x.id)));
}

#[sqlx::test(migrations = "./migrations")]
async fn expired_lease_returns_to_pending_then_fails(pool: PgPool) {
    let store = PgInboxStore::new(pool.clone());
    store.insert_batch(&[msg("1-0")]).await.unwrap();

    // Claim with a lease that expires immediately, simulating a crash.
    let claimed = store
        .claim_batch("events", "w1", 1, Duration::from_millis(1))
        .await
        .unwrap();
    assert_eq!(claimed.len(), 1);
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(store.reap_stale(5).await.unwrap(), 1);
    let again = store
        .claim_batch("events", "w2", 1, Duration::from_millis(1))
        .await
        .unwrap();
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].attempts, 1);

    // With max_attempts = 2, the next expiry is fatal.
    tokio::time::sleep(Duration::from_millis(50)).await;
    store.reap_stale(2).await.unwrap();
    let (status,): (String,) = sqlx::query_as("SELECT status FROM inbox_messages")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "failed");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_worker_that_lost_its_lease_cannot_overwrite(pool: PgPool) {
    let store = PgInboxStore::new(pool.clone());
    store.insert_batch(&[msg("1-0")]).await.unwrap();
    let c = store.claim_batch("events", "w1", 1, LEASE).await.unwrap();
    let id = c[0].id;

    // Simulate reaper + new owner.
    sqlx::query("UPDATE inbox_messages SET status='pending', locked_by=NULL, locked_until=NULL")
        .execute(&pool)
        .await
        .unwrap();

    let mut conn = pool.acquire().await.unwrap();
    let err = super::mark_processed_tx(&mut conn, id).await.unwrap_err();
    assert!(matches!(err, crate::error::Error::LeaseLost(_)));
}
