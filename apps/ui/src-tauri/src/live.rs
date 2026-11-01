use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use serde::Serialize;
use vestigium_core::{PcState, Reading, pc_state};
use vestigium_ipc::{NowFrame, Request, Response};

use crate::agent::UiError;

/// How often the live screen asks for the latest second.
pub const LIVE_PERIOD: Duration = Duration::from_secs(1);

/// What the live screen receives, in order.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LiveMessage {
    /// The last ten minutes, sent once before the first second.
    Recent { readings: Vec<Reading> },
    /// The latest second; `reading` and `state` are `None` until the first sample.
    Now {
        reading: Option<Reading>,
        state: Option<PcState>,
        frame: Box<NowFrame>,
    },
    /// The watcher stopped answering; nothing follows.
    Stopped,
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
    if !send(LiveMessage::Recent { readings }) {
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
        let reading = frame.sample.as_ref().map(Reading::of_sample);
        let message = LiveMessage::Now {
            reading,
            state: reading.map(|r| pc_state(Some(&r), false)),
            frame: Box::new(frame),
        };
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

    use super::*;

    fn agent(request: &Request) -> Result<Response, UiError> {
        Ok(match request {
            Request::Recent => Response::Recent {
                readings: Vec::new(),
            },
            Request::Now => Response::Now(NowFrame::default()),
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
        assert!(matches!(sent[2], LiveMessage::Now { state: None, .. }));
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
