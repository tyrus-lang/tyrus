use super::*;
use crate::codec::compress;

fn header(name: &str, value: &str) -> (String, String) {
    (name.to_string(), value.to_string())
}

#[test]
fn codings_are_offered_unless_the_program_chose_its_own() {
    let mut headers = vec![header("accept", "application/json")];
    offer_codings(&mut headers);
    assert!(headers.contains(&header("accept-encoding", OFFERED_CODINGS)));
    let mut chosen = vec![header("Accept-Encoding", "identity")];
    offer_codings(&mut chosen);
    assert_eq!(chosen, vec![header("Accept-Encoding", "identity")]);
}

#[test]
fn only_a_single_known_coding_is_decoded() {
    for (value, coding) in [
        ("gzip", Some(Coding::Gzip)),
        ("X-GZIP", Some(Coding::Gzip)),
        (" deflate ", Some(Coding::Deflate)),
        ("br", Some(Coding::Brotli)),
        ("identity", None),
        ("gzip, br", None),
        ("zstd", None),
    ] {
        assert_eq!(
            response_coding(&[header("Content-Encoding", value)]),
            coding,
            "{value}"
        );
    }
    assert_eq!(response_coding(&[]), None);
}

#[test]
fn the_encoded_length_and_coding_headers_are_dropped() {
    let mut headers = vec![
        header("Content-Encoding", "gzip"),
        header("content-length", "42"),
        header("content-type", "text/plain"),
    ];
    forget_encoded_length(&mut headers);
    assert_eq!(headers, vec![header("content-type", "text/plain")]);
}

#[test]
fn every_coding_decodes_including_raw_deflate_sent_as_deflate() {
    let body = b"decoded body ".repeat(1_000);
    for (coding, format) in [
        (Coding::Gzip, Format::Gzip),
        (Coding::Deflate, Format::Deflate),
        (Coding::Deflate, Format::DeflateRaw),
        (Coding::Brotli, Format::Brotli),
    ] {
        let encoded = compress(&body, format, None).unwrap();
        assert_eq!(
            decode_body(&encoded, coding, u64::MAX).unwrap(),
            body,
            "{format:?}"
        );
    }
    assert_eq!(decode_body(b"", Coding::Gzip, 10).unwrap(), b"");
}

#[test]
fn a_body_that_is_not_what_it_claims_or_too_large_is_refused() {
    assert_eq!(
        decode_body(b"plain text", Coding::Gzip, u64::MAX),
        Err(DECODE_FAILED)
    );
    let bomb = compress(&vec![0_u8; 8 * 1024 * 1024], Format::Gzip, Some(9)).unwrap();
    assert_eq!(
        decode_body(&bomb, Coding::Gzip, u64::MAX),
        Err(RATIO_EXCEEDED)
    );
    let text = compress(&b"x".repeat(4_000), Format::Brotli, None).unwrap();
    assert_eq!(decode_body(&text, Coding::Brotli, 1_000), Err(TOO_LARGE));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_streamed_body_decodes_across_chunk_boundaries() {
    let body = b"streamed response body ".repeat(5_000);
    for (coding, format) in [
        (Coding::Gzip, Format::Gzip),
        (Coding::Deflate, Format::Deflate),
        (Coding::Deflate, Format::DeflateRaw),
        (Coding::Brotli, Format::Brotli),
    ] {
        let encoded = compress(&body, format, None).unwrap();
        let (readable, producer) = crate::stream::byte_readable_stream(64 * 1024);
        let readable = unsafe { crate::stream::take_owned_readable_stream(readable) };
        let reader = tokio::spawn(async move {
            let mut output = Vec::new();
            while let Some(chunk) = crate::stream::read_owned_byte_stream(&readable)
                .await
                .unwrap()
            {
                output.extend(chunk);
            }
            output
        });
        let mut decoder = StreamDecoder::new(coding, u64::MAX);
        for chunk in encoded.chunks(1) {
            decoder.push(chunk, &producer).await.unwrap();
        }
        decoder.finish(&producer).await.unwrap();
        producer.close();
        assert_eq!(reader.await.unwrap(), body, "{format:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_streamed_body_that_does_not_decode_fails_the_stream() {
    let (readable, producer) = crate::stream::byte_readable_stream(64 * 1024);
    let _readable = unsafe { crate::stream::take_owned_readable_stream(readable) };
    let mut decoder = StreamDecoder::new(Coding::Gzip, u64::MAX);
    let error = decoder
        .push(b"not gzip at all", &producer)
        .await
        .unwrap_err();
    assert_eq!(error.code, DECODE_FAILED.0);
    let mut empty = StreamDecoder::new(Coding::Brotli, u64::MAX);
    assert!(empty.finish(&producer).await.is_ok());
}
