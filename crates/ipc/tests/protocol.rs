use vestigium_core::{
    EpisodeKind, LanguagePref, PCIE_ASPM, PlanItem, PressureSpan, ProgramPoint, Reading,
    SUB_PCIEXPRESS, Setting, SettingChange, SettingResult, Settings, TweakChange, TweakId, Value,
};

fn aspm_item() -> PlanItem {
    PlanItem {
        tweak: TweakId::PcieAspm,
        subject: None,
        undo_of: None,
        undo_journal: None,
        changes: vec![SettingChange {
            setting: Setting::PowerAc {
                subgroup: SUB_PCIEXPRESS,
                setting: PCIE_ASPM,
                scheme: None,
            },
            from: Value::Index(2),
            to: Value::Index(0),
        }],
    }
}
use vestigium_core::HistoryReset;
use vestigium_core::health::FindingKind;
use vestigium_ipc::{
    AgentStatus, ErrorCode, MonitorSource, MonitorState, MonitorStatus, NowFrame, PROTOCOL_VERSION,
    Request, Response, SourceFailure, TrayState,
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
        Request::ProgramRecent {
            name: "build.exe".to_string(),
        },
        Request::SetPaused {
            paused: true,
            until_ms: Some(1_700_000_000_000),
        },
        Request::SetSensitive { on: true },
        Request::Settings,
        Request::SetSettings(settings),
        Request::SetAutostart { on: false },
        Request::ClearHistory,
        Request::ImportResults,
        Request::ApplyUserTweak { item: aspm_item() },
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
            suggest_capture: true,
            suspended: true,
            monitor: MonitorStatus {
                state: MonitorState::Degraded,
                failing: vec![SourceFailure {
                    source: MonitorSource::Counters,
                    since_ms: 1_700_000_000_000,
                    error: "counter query failed".to_string(),
                }],
                no_reading_since_ms: Some(1_700_000_100_000),
                resumed_ms: Some(1_699_999_000_000),
                health_attempt_ms: Some(1_700_000_200_000),
                health_ok_ms: Some(1_700_000_000_000),
                health_unread: vec![FindingKind::TwoAntivirus, FindingKind::DiskSmart],
                history_reset: Some(HistoryReset {
                    file: "vestigium.db.corrupt-1700000000000".to_string(),
                    at_ms: 1_700_000_300_000,
                    tweaks: true,
                    settings: false,
                    machine: true,
                }),
            },
            ..AgentStatus::default()
        }),
        Response::Now(Box::default()),
        Response::ProgramRecent {
            points: vec![ProgramPoint {
                ts_ms: 5_000,
                cpu_pct: 3.5,
                private_bytes: 64 << 20,
                io_bps: 2048.0,
            }],
        },
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
            pressure: vec![
                PressureSpan {
                    kind: EpisodeKind::MemoryHigh,
                    start_ms: 400,
                    end_ms: Some(900),
                },
                PressureSpan {
                    kind: EpisodeKind::CpuSaturated,
                    start_ms: 1_000,
                    end_ms: None,
                },
            ],
        },
        Response::Settings {
            settings: Settings::default(),
            autostart: true,
        },
        Response::Imported {
            captures: vec![3, 4],
            scans: vec![5],
            dumps: vec![6],
            deleted: 1,
            refused: 2,
            failed: 3,
            tweaks: vec![7],
            tests: vec![9],
        },
        Response::TweakApplied {
            id: 8,
            change: TweakChange {
                tweak: TweakId::PcieAspm,
                subject: None,
                undo_of: None,
                results: aspm_item()
                    .changes
                    .into_iter()
                    .map(|change| SettingResult {
                        change,
                        error: None,
                    })
                    .collect(),
                restore_point: None,
                journal: None,
            },
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
    let json = serde_json::to_string(&Request::SetPaused {
        paused: true,
        until_ms: None,
    })
    .unwrap();
    assert_eq!(
        json,
        r#"{"type":"set_paused","paused":true,"until_ms":null}"#
    );
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

#[test]
fn recent_carries_the_pressure_spans() {
    let response = Response::Recent {
        readings: Vec::new(),
        pressure: vec![PressureSpan {
            kind: EpisodeKind::CpuSaturated,
            start_ms: 1_000,
            end_ms: None,
        }],
    };
    let json = serde_json::to_value(response).unwrap();
    assert_eq!(json["type"], "recent");
    assert_eq!(
        json["pressure"],
        serde_json::json!([{ "kind": "CpuSaturated", "start_ms": 1_000, "end_ms": null }])
    );
}

#[test]
fn status_without_suggest_capture_still_decodes() {
    let json = r#"{"type":"status","paused":false,"sensitive_until_ms":null,"state":"normal","started_at_ms":1,"last_sample_ms":null,"last_error":null}"#;
    let Response::Status(status) = serde_json::from_str(json).unwrap() else {
        panic!("not a status");
    };
    assert!(!status.suggest_capture);
}

#[test]
fn a_status_without_the_monitor_fields_still_reads() {
    let old = r#"{"paused":false,"sensitive_until_ms":null,"state":"normal","started_at_ms":1,
        "last_sample_ms":null,"last_error":null}"#;

    let status: AgentStatus = serde_json::from_str(old).unwrap();

    assert_eq!(status.monitor, MonitorStatus::default());
    assert!(!status.suspended);
}

#[test]
fn a_monitor_without_a_set_aside_history_still_reads() {
    let old = r#"{"state":"watching","failing":[]}"#;

    let monitor: MonitorStatus = serde_json::from_str(old).unwrap();

    assert_eq!(monitor.history_reset, None);
}
