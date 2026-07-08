use bytes::Bytes;
use http::StatusCode;
use axum::http::HeaderMap;
use crate::proxy::hyper_client::ProxyResponse;
use crate::proxy::ProxyError;
use std::time::Duration;
use super::tests::test_forwarder;

#[tokio::test]
async fn non_streaming_success_is_buffered_before_marking_provider_successful() {
    let forwarder = test_forwarder(Duration::from_secs(1), Duration::from_secs(1));
    let response = ProxyResponse::streamed(
        StatusCode::OK,
        HeaderMap::new(),
        futures::stream::once(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<Bytes, std::io::Error>(Bytes::from_static(b"{\"ok\":true}"))
        }),
    );

    let prepared = forwarder
        .prepare_success_response_for_failover(response, false)
        .await
        .expect("response should be buffered");

    assert_eq!(
        prepared.bytes().await.unwrap(),
        Bytes::from_static(b"{\"ok\":true}")
    );
}

#[tokio::test]
async fn non_streaming_body_read_error_is_retryable_before_success_record() {
    let forwarder = test_forwarder(Duration::from_secs(1), Duration::from_secs(1));
    let response = ProxyResponse::streamed(
        StatusCode::OK,
        HeaderMap::new(),
        futures::stream::once(async {
            Err::<Bytes, std::io::Error>(std::io::Error::other("body boom"))
        }),
    );

    let err = match forwarder
        .prepare_success_response_for_failover(response, false)
        .await
    {
        Ok(_) => panic!("body read errors should fail the attempt"),
        Err(err) => err,
    };

    assert!(matches!(err, ProxyError::ForwardFailed(_)));
}

#[tokio::test]
async fn streaming_success_primes_first_chunk_and_replays_it() {
    let forwarder = test_forwarder(Duration::from_secs(1), Duration::from_secs(1));
    let response = ProxyResponse::streamed(
        StatusCode::OK,
        HeaderMap::new(),
        futures::stream::iter(vec![
            Ok::<Bytes, std::io::Error>(Bytes::from_static(b"first")),
            Ok::<Bytes, std::io::Error>(Bytes::from_static(b"second")),
        ]),
    );

    let prepared = forwarder
        .prepare_success_response_for_failover(response, true)
        .await
        .expect("stream should be primed");

    assert_eq!(
        prepared.bytes().await.unwrap(),
        Bytes::from_static(b"firstsecond")
    );
}

#[tokio::test]
async fn streaming_first_chunk_error_is_retryable_before_success_record() {
    let forwarder = test_forwarder(Duration::from_secs(1), Duration::from_secs(1));
    let response = ProxyResponse::streamed(
        StatusCode::OK,
        HeaderMap::new(),
        futures::stream::once(async {
            Err::<Bytes, std::io::Error>(std::io::Error::other("first chunk boom"))
        }),
    );

    let err = match forwarder
        .prepare_success_response_for_failover(response, true)
        .await
    {
        Ok(_) => panic!("first chunk errors should fail the attempt"),
        Err(err) => err,
    };

    assert!(matches!(err, ProxyError::ForwardFailed(_)));
}
