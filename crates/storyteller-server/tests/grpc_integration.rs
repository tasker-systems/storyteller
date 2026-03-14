//! Integration tests for gRPC services.
//!
//! These tests start a real gRPC server and verify RPCs via tonic client stubs.
//! Requires STORYTELLER_DATA_PATH to be set.

use storyteller_server::grpc::composer_service::ComposerServiceImpl;
use storyteller_server::proto::composer_service_client::ComposerServiceClient;
use storyteller_server::proto::composer_service_server::ComposerServiceServer;
use storyteller_server::proto::*;

use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::transport::{Channel, Server};

async fn start_test_server() -> Option<String> {
    let data_path = std::env::var("STORYTELLER_DATA_PATH").ok()?;
    let composer = Arc::new(
        storyteller_composer::SceneComposer::load(std::path::Path::new(&data_path))
            .expect("load descriptors"),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}");

    let service = ComposerServiceImpl::new(composer);

    tokio::spawn(async move {
        Server::builder()
            .add_service(ComposerServiceServer::new(service))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Some(url)
}

#[tokio::test]
async fn list_genres_returns_non_empty() {
    let Some(url) = start_test_server().await else {
        eprintln!("STORYTELLER_DATA_PATH not set — skipping");
        return;
    };

    let channel = Channel::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = ComposerServiceClient::new(channel);

    let response = client.list_genres(()).await.unwrap();
    let genres = response.into_inner().genres;

    assert!(!genres.is_empty(), "should return at least one genre");
    assert!(
        genres.iter().any(|g| g.slug == "low_fantasy_folklore"),
        "should contain low_fantasy_folklore"
    );
}

#[tokio::test]
async fn profiles_for_genre_returns_results() {
    let Some(url) = start_test_server().await else {
        eprintln!("STORYTELLER_DATA_PATH not set — skipping");
        return;
    };

    let channel = Channel::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = ComposerServiceClient::new(channel);

    let response = client
        .get_profiles_for_genre(GenreRequest {
            genre_id: "low_fantasy_folklore".to_string(),
        })
        .await
        .unwrap();

    let profiles = response.into_inner().profiles;
    assert!(!profiles.is_empty(), "should return profiles for genre");
}

#[tokio::test]
async fn invalid_genre_returns_empty_profiles() {
    let Some(url) = start_test_server().await else {
        eprintln!("STORYTELLER_DATA_PATH not set — skipping");
        return;
    };

    let channel = Channel::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = ComposerServiceClient::new(channel);

    let response = client
        .get_profiles_for_genre(GenreRequest {
            genre_id: "nonexistent_genre".to_string(),
        })
        .await
        .unwrap();

    assert!(response.into_inner().profiles.is_empty());
}
