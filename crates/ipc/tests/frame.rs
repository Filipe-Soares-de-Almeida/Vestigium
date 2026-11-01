use std::io::Cursor;

use vestigium_ipc::{
    AgentStatus, IpcError, MAX_FRAME, PROTOCOL_VERSION, Request, Response, read_frame,
    user_only_sddl, write_frame,
};

#[test]
fn frame_round_trip() {
    let status = Response::Status(AgentStatus {
        paused: true,
        started_at_ms: 1_700_000_000_000,
        last_sample_ms: Some(1_700_000_005_000),
        ..AgentStatus::default()
    });
    let mut buf = Vec::new();
    write_frame(&mut buf, &status).unwrap();
    write_frame(
        &mut buf,
        &Request::Hello {
            protocol: PROTOCOL_VERSION,
        },
    )
    .unwrap();

    let mut cursor = Cursor::new(buf);
    assert_eq!(read_frame::<_, Response>(&mut cursor).unwrap(), status);
    assert_eq!(
        read_frame::<_, Request>(&mut cursor).unwrap(),
        Request::Hello {
            protocol: PROTOCOL_VERSION
        }
    );
}

#[test]
fn oversized_frame_is_rejected_without_alloc() {
    let mut cursor = Cursor::new(u32::MAX.to_le_bytes().to_vec());

    let err = read_frame::<_, Request>(&mut cursor).unwrap_err();

    assert!(matches!(err, IpcError::FrameTooLarge(u32::MAX)));
    const { assert!(MAX_FRAME < u32::MAX) };
}

#[test]
fn truncated_frame_is_an_error() {
    let mut bytes = 10u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(b"{\"t");

    let err = read_frame::<_, Request>(&mut Cursor::new(bytes)).unwrap_err();

    assert!(matches!(err, IpcError::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof));
}

#[test]
fn malformed_json_is_an_error() {
    let body = b"{\"type\":\"nope\"}";
    let mut bytes = (body.len() as u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(body);

    let err = read_frame::<_, Request>(&mut Cursor::new(bytes)).unwrap_err();

    assert!(matches!(err, IpcError::Json(_)));
}

#[test]
fn sddl_grants_only_the_user() {
    assert_eq!(
        user_only_sddl("S-1-5-21-1-2-3-1001"),
        "D:P(A;;GA;;;S-1-5-21-1-2-3-1001)"
    );
}
