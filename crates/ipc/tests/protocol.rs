use vestigium_core::{EpisodeKind, LanguagePref, Reading, Settings};
use vestigium_ipc::{
    AgentStatus, ErrorCode, NowFrame, PROTOCOL_VERSION, Request, Response, TrayState,
};

fn roundtrip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    serde_json::from_str(&serde_json::to_string(value).unwrap()).unwrap()
}

#[test]
fn every_request_roundtrips_as_json() {
    let settings = Settings {
        language: LanguagePref::PtBr,
        ..Settings::default()
    };
    let requests = [
        Request::Hello {
            protocol: PROTOCOL_VERSION,
        },
        Request::Status,
        Request::Now,
        Request::Recent,
        Request::SetPaused { paused: true },
        Request::SetSensitive { on: true },
        Request::Settings,
        Request::SetSettings(settings),
        Request::SetAutostart { on: false },
    ];
    for request in requests {
        assert_eq!(roundtrip(&request), request);
    }
}

#[test]
fn every_response_roundtrips_as_json() {
    let responses = [
        Response::Hello {
            protocol: PROTOCOL_VERSION,
            agent_version: "0.0.0".to_string(),
        },
        Response::Status(AgentStatus {
            sensitive_until_ms: Some(42),
            state: TrayState::Attention,
            ..AgentStatus::default()
        }),
        Response::Now(Box::default()),
        Response::Now(Box::new(NowFrame {
            pressure: vec![
                EpisodeKind::CpuSaturated,
                EpisodeKind::DiskBusy {
                    disk: "0 C:".to_string(),
                },
            ],
            ..NowFrame::default()
        })),
        Response::Recent {
            readings: vec![Reading {
                ts_ms: 1_000,
                cpu_pct: 12.5,
                mem_used_pct: Some(40.0),
                disk_active_pct: 3.0,
                disk_latency_ms: 0.8,
                gpu_pct: None,
                probe_lag_ms: 1.2,
            }],
        },
        Response::Settings {
            settings: Settings::default(),
            autostart: true,
        },
        Response::Done,
        Response::Error {
            code: ErrorCode::InvalidSetting,
            message: "records kept 30 days".to_string(),
        },
    ];
    for response in responses {
        assert_eq!(roundtrip(&response), response);
    }
}

#[test]
fn requests_are_tagged_by_type() {
    let json = serde_json::to_string(&Request::SetPaused { paused: true }).unwrap();
    assert_eq!(json, r#"{"type":"set_paused","paused":true}"#);
}

#[test]
fn now_lists_the_open_pressure_episodes() {
    let frame = NowFrame {
        pressure: vec![EpisodeKind::MemoryHigh],
        ..NowFrame::default()
    };
    let json = serde_json::to_value(Response::Now(Box::new(frame))).unwrap();
    assert_eq!(json["type"], "now");
    assert_eq!(json["pressure"], serde_json::json!(["MemoryHigh"]));
}
