use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use serde::Serialize;
use vestigium_core::{EpisodeKind, PcState, Reading, pc_state};
use vestigium_ipc::{NowFrame, Request, Response};

use crate::agent::UiError;

/// How often the live screen asks for the latest second.
pub const LIVE_PERIOD: Duration = Duration::from_secs(1);

/// The one running live stream: its id and the sender whose drop ends it.
#[derive(Default)]
pub struct LiveSlot {
    current: Option<(u64, Sender<()>)>,
    last_id: u64,
}

impl LiveSlot {
    /// Makes `stop` the current stream, ending the one before it; returns the new stream's id.
    pub fn replace(&mut self, stop: Sender<()>) -> u64 {
        self.last_id = self.last_id.wrapping_add(1);
        self.current = Some((self.last_id, stop));
        self.last_id
    }

    /// Ends the stream `id` if it is still the current one; a stale id does nothing.
    pub fn stop(&mut self, id: u64) {
        if self.current.as_ref().is_some_and(|(held, _)| *held == id) {
            self.current = None;
        }
    }
}

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
    /// The watcher is running but did not answer in time; nothing follows.
    Silent,
}

/// What ends the stream when a request fails: silence from a running watcher, or a stop.
fn ending(err: &UiError) -> LiveMessage {
    if err.is_silent() {
        LiveMessage::Silent
    } else {
        LiveMessage::Stopped
    }
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
    /// `pressure` lists the pressure episodes that second falls in.
    pub fn of(reading: Reading, pressure: &[EpisodeKind]) -> LivePoint {
        let none = Reading {
            cpu_pct: 0.0,
            mem_used_pct: None,
            disk_active_pct: 0.0,
            gpu_pct: None,
            ..reading
        };
        let open = |kind: fn(&EpisodeKind) -> bool| pressure.iter().any(kind);
        let alone = |only: Reading, open: bool| pc_state(Some(&only), open, false);
        LivePoint {
            reading,
            state: pc_state(Some(&reading), !pressure.is_empty(), false),
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
    let (readings, pressure) = match call(&Request::Recent) {
        Ok(Response::Recent { readings, pressure }) => (readings, pressure),
        Err(err) => {
            send(ending(&err));
            return;
        }
        Ok(_) => {
            send(LiveMessage::Stopped);
            return;
        }
    };
    let points = readings
        .into_iter()
        .map(|r| {
            let open: Vec<EpisodeKind> = pressure
                .iter()
                .filter(|span| span.covers(r.ts_ms))
                .map(|span| span.kind.clone())
                .collect();
            LivePoint::of(r, &open)
        })
        .collect();
    if !send(LiveMessage::Recent { points }) {
        return;
    }
    loop {
        let frame = match call(&Request::Now) {
            Ok(Response::Now(frame)) => frame,
            Err(err) => {
                send(ending(&err));
                return;
            }
            Ok(_) => {
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

    use vestigium_core::{EpisodeKind, PressureSpan, Sample};
    use vestigium_ipc::IpcError;

    use super::*;

    fn quiet_second() -> Sample {
        Sample {
            ts_ms: 1_000,
            cpu_pct: 20.0,
            dpc_pct: 0.2,
            isr_pct: 0.1,
            perf_limit_pct: Some(100.0),
            proc_perf_pct: Some(100.0),
            mem_total_bytes: 16 << 30,
            mem_avail_bytes: 8 << 30,
            commit_bytes: 8 << 30,
            commit_limit_bytes: 32 << 30,
            hard_faults_per_s: Some(0.0),
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
                pressure: Vec::new(),
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

    fn timed_out() -> UiError {
        UiError::Ipc(IpcError::Io(std::io::ErrorKind::TimedOut.into()))
    }

    #[test]
    fn a_watcher_that_stops_answering_ends_the_stream_as_silent() {
        let (_keep, stop) = mpsc::channel();
        let mut sent = Vec::new();
        let mut asked = 0;

        run_live(
            |request| {
                asked += 1;
                if asked > 2 {
                    return Err(timed_out());
                }
                agent(request)
            },
            |message| {
                sent.push(message);
                true
            },
            &stop,
            Duration::from_millis(1),
        );

        assert!(matches!(sent.last(), Some(LiveMessage::Silent)));
    }

    #[test]
    fn a_silent_first_request_ends_the_stream_as_silent() {
        let (_keep, stop) = mpsc::channel();
        let mut sent = Vec::new();

        run_live(
            |_| Err(timed_out()),
            |message| {
                sent.push(message);
                true
            },
            &stop,
            Duration::from_millis(1),
        );

        assert_eq!(sent, [LiveMessage::Silent]);
    }

    #[test]
    fn a_gone_watcher_ends_the_stream_as_stopped() {
        let (_keep, stop) = mpsc::channel();
        let mut sent = Vec::new();

        run_live(
            |_| Err(UiError::AgentStopped),
            |message| {
                sent.push(message);
                true
            },
            &stop,
            Duration::from_millis(1),
        );

        assert_eq!(sent, [LiveMessage::Stopped]);
    }

    #[test]
    fn slot_stop_for_an_old_stream_keeps_the_new_one() {
        let mut slot = LiveSlot::default();
        let (a_tx, a_rx) = mpsc::channel::<()>();
        let (b_tx, b_rx) = mpsc::channel::<()>();

        let a = slot.replace(a_tx);
        let b = slot.replace(b_tx);
        slot.stop(a);

        assert_ne!(a, b);
        assert!(matches!(
            a_rx.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
        assert!(matches!(b_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        slot.stop(b);
        assert!(matches!(
            b_rx.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
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
    fn recent_seconds_inside_an_episode_are_under_pressure() {
        let (_keep, stop) = mpsc::channel();
        let mut sent = Vec::new();
        let disk = EpisodeKind::DiskBusy {
            disk: "0 C:".to_string(),
        };

        run_live(
            |request| {
                Ok(match request {
                    Request::Recent => Response::Recent {
                        readings: (1..=6)
                            .map(|s| Reading {
                                ts_ms: s * 1_000,
                                ..reading(20.0, Some(50.0), 5.0, None)
                            })
                            .collect(),
                        pressure: vec![
                            PressureSpan {
                                kind: EpisodeKind::CpuSaturated,
                                start_ms: 2_000,
                                end_ms: Some(3_000),
                            },
                            PressureSpan {
                                kind: disk.clone(),
                                start_ms: 5_000,
                                end_ms: None,
                            },
                        ],
                    },
                    other => agent(other)?,
                })
            },
            |message| {
                sent.push(message);
                false
            },
            &stop,
            Duration::from_millis(1),
        );

        let Some(LiveMessage::Recent { points }) = sent.first() else {
            panic!("the recent minutes come first");
        };
        let states: Vec<PcState> = points.iter().map(|p| p.state).collect();
        assert_eq!(
            states,
            [
                PcState::Calm,
                PcState::Pressure,
                PcState::Pressure,
                PcState::Calm,
                PcState::Pressure,
                PcState::Pressure,
            ]
        );
        assert_eq!(points[1].lanes.cpu, PcState::Pressure);
        assert_eq!(points[1].lanes.disk, PcState::Calm);
        assert_eq!(points[5].lanes.disk, PcState::Pressure);
        assert_eq!(points[5].lanes.cpu, PcState::Calm);
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
