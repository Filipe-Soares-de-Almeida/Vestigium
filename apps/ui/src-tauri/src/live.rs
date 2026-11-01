use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use serde::Serialize;
use vestigium_core::{EpisodeKind, PcState, Reading, pc_state};
use vestigium_ipc::{NowFrame, Request, Response};

use crate::agent::UiError;

/// How often the live screen asks for the latest second.
pub const LIVE_PERIOD: Duration = Duration::from_secs(1);

/// What the live screen receives, in order.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LiveMessage {
    /// The last ten minutes, sent once before the first second.
    Recent { points: Vec<LivePoint> },
    /// The latest second; `point` is `None` until the first sample.
    Now {
        point: Option<LivePoint>,
        frame: Box<NowFrame>,
    },
    /// The watcher stopped answering; nothing follows.
    Stopped,
}

/// One second of the live chart: the reading, the state of the PC and of each lane.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct LivePoint {
    pub reading: Reading,
    pub state: PcState,
    pub lanes: Lanes,
}

/// The state each lane would give the PC on its own; a lane with no reading is suspended.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Lanes {
    pub cpu: PcState,
    pub memory: PcState,
    pub disk: PcState,
    pub gpu: PcState,
}

impl LivePoint {
    /// `pressure` lists the episodes open at that second; the recent minutes carry none.
    pub fn of(reading: Reading, pressure: &[EpisodeKind]) -> LivePoint {
        let none = Reading {
            cpu_pct: 0.0,
            mem_used_pct: None,
            disk_active_pct: 0.0,
            gpu_pct: None,
            ..reading
        };
        let open = |kind: fn(&EpisodeKind) -> bool| pressure.iter().any(kind);
        let alone = |only: Reading, open: bool| pc_state(Some(&only), open);
        LivePoint {
            reading,
            state: pc_state(Some(&reading), !pressure.is_empty()),
            lanes: Lanes {
                cpu: alone(
                    Reading {
                        cpu_pct: reading.cpu_pct,
                        ..none
                    },
                    open(|k| matches!(k, EpisodeKind::CpuSaturated)),
                ),
                memory: match reading.mem_used_pct {
                    Some(_) => alone(
                        Reading {
                            mem_used_pct: reading.mem_used_pct,
                            ..none
                        },
                        open(|k| matches!(k, EpisodeKind::MemoryHigh)),
                    ),
                    None => PcState::Suspended,
                },
                disk: alone(
                    Reading {
                        disk_active_pct: reading.disk_active_pct,
                        ..none
                    },
                    open(|k| matches!(k, EpisodeKind::DiskBusy { .. })),
                ),
                gpu: match reading.gpu_pct {
                    Some(_) => alone(
                        Reading {
                            gpu_pct: reading.gpu_pct,
                            ..none
                        },
                        false,
                    ),
                    None => PcState::Suspended,
                },
            },
        }
    }
}

/// Streams the recent minutes, then one second per `period`, until `stop` is dropped or
/// `send` reports the receiver gone.
pub fn run_live(
    mut call: impl FnMut(&Request) -> Result<Response, UiError>,
    mut send: impl FnMut(LiveMessage) -> bool,
    stop: &Receiver<()>,
    period: Duration,
) {
    let readings = match call(&Request::Recent) {
        Ok(Response::Recent { readings }) => readings,
        _ => {
            send(LiveMessage::Stopped);
            return;
        }
    };
    let points = readings
        .into_iter()
        .map(|r| LivePoint::of(r, &[]))
        .collect();
    if !send(LiveMessage::Recent { points }) {
        return;
    }
    loop {
        let frame = match call(&Request::Now) {
            Ok(Response::Now(frame)) => frame,
            _ => {
                send(LiveMessage::Stopped);
                return;
            }
        };
        let point = frame
            .sample
            .as_ref()
            .map(|sample| LivePoint::of(Reading::of_sample(sample), &frame.pressure));
        let message = LiveMessage::Now { point, frame };
        if !send(message) {
            return;
        }
        match stop.recv_timeout(period) {
            Err(RecvTimeoutError::Timeout) => {}
            Ok(()) | Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use vestigium_core::{EpisodeKind, Sample};

    use super::*;

    fn quiet_second() -> Sample {
        Sample {
            ts_ms: 1_000,
            cpu_pct: 20.0,
            dpc_pct: 0.2,
            isr_pct: 0.1,
            perf_limit_pct: 100.0,
            proc_perf_pct: 100.0,
            mem_total_bytes: 16 << 30,
            mem_avail_bytes: 8 << 30,
            commit_bytes: 8 << 30,
            commit_limit_bytes: 32 << 30,
            hard_faults_per_s: 0.0,
            gpu_pct: None,
            probe_max_lag_ms: 1.0,
            sampler_lag_ms: 1.0,
            cores: Vec::new(),
            disks: Vec::new(),
        }
    }

    fn agent(request: &Request) -> Result<Response, UiError> {
        Ok(match request {
            Request::Recent => Response::Recent {
                readings: Vec::new(),
            },
            Request::Now => Response::Now(Box::default()),
            _ => Response::Done,
        })
    }

    #[test]
    fn live_stops_when_the_channel_closes() {
        let (_keep, stop) = mpsc::channel();
        let mut sent = Vec::new();

        run_live(
            agent,
            |message| {
                sent.push(message);
                sent.len() < 3
            },
            &stop,
            Duration::from_millis(1),
        );

        assert_eq!(sent.len(), 3);
        assert!(matches!(sent[0], LiveMessage::Recent { .. }));
        assert!(matches!(sent[2], LiveMessage::Now { point: None, .. }));
    }

    #[test]
    fn live_stops_when_asked() {
        let (keep, stop) = mpsc::channel::<()>();
        drop(keep);
        let mut sent = 0;

        run_live(
            agent,
            |_| {
                sent += 1;
                true
            },
            &stop,
            Duration::from_secs(60),
        );

        assert_eq!(sent, 2);
    }

    #[test]
    fn live_is_under_pressure_only_inside_an_episode() {
        let (_keep, stop) = mpsc::channel();
        let mut sent = Vec::new();
        let mut open = false;

        run_live(
            |request| {
                Ok(match request {
                    Request::Now => {
                        open = !open;
                        Response::Now(Box::new(NowFrame {
                            sample: Some(quiet_second()),
                            pressure: if open {
                                vec![EpisodeKind::CpuSaturated]
                            } else {
                                Vec::new()
                            },
                            ..NowFrame::default()
                        }))
                    }
                    other => agent(other)?,
                })
            },
            |message| {
                sent.push(message);
                sent.len() < 3
            },
            &stop,
            Duration::from_millis(1),
        );

        let states: Vec<Option<PcState>> = sent
            .iter()
            .filter_map(|m| match m {
                LiveMessage::Now { point, .. } => Some(point.map(|p| p.state)),
                _ => None,
            })
            .collect();
        assert_eq!(states, [Some(PcState::Pressure), Some(PcState::Calm)]);
    }

    fn reading(cpu: f32, memory: Option<f32>, disk: f32, gpu: Option<f32>) -> Reading {
        Reading {
            ts_ms: 1_000,
            cpu_pct: cpu,
            mem_used_pct: memory,
            disk_active_pct: disk,
            disk_latency_ms: 1.0,
            gpu_pct: gpu,
            probe_lag_ms: 1.0,
        }
    }

    #[test]
    fn each_lane_is_judged_on_its_own() {
        let point = LivePoint::of(reading(60.0, Some(50.0), 5.0, Some(10.0)), &[]);

        assert_eq!(point.state, PcState::Busy);
        assert_eq!(
            point.lanes,
            Lanes {
                cpu: PcState::Busy,
                memory: PcState::Calm,
                disk: PcState::Calm,
                gpu: PcState::Calm,
            }
        );
    }

    #[test]
    fn only_the_lane_of_an_open_episode_is_under_pressure() {
        let disk = EpisodeKind::DiskBusy {
            disk: "0 C:".to_string(),
        };
        let point = LivePoint::of(reading(20.0, Some(50.0), 70.0, Some(10.0)), &[disk]);

        assert_eq!(point.state, PcState::Pressure);
        assert_eq!(point.lanes.disk, PcState::Pressure);
        assert_eq!(point.lanes.cpu, PcState::Calm);
    }

    #[test]
    fn a_lane_without_a_reading_is_suspended() {
        let point = LivePoint::of(reading(5.0, None, 1.0, None), &[]);

        assert_eq!(point.lanes.memory, PcState::Suspended);
        assert_eq!(point.lanes.gpu, PcState::Suspended);
        assert_eq!(point.state, PcState::Calm);
    }

    #[test]
    fn live_says_when_the_watcher_stops() {
        let (_keep, stop) = mpsc::channel();
        let mut answers = 0;
        let mut sent = Vec::new();

        run_live(
            |request| {
                answers += 1;
                if answers > 2 {
                    Err(UiError::AgentStopped)
                } else {
                    agent(request)
                }
            },
            |message| {
                sent.push(message);
                true
            },
            &stop,
            Duration::from_millis(1),
        );

        assert_eq!(sent.last(), Some(&LiveMessage::Stopped));
        assert_eq!(sent.len(), 3);
    }
}
