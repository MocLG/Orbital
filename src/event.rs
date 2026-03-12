use crossterm::event::{self, Event, KeyEvent};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::sync::mpsc;

pub enum AppEvent {
    Key(KeyEvent),
    Tick,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
    paused: Arc<AtomicBool>,
    #[allow(dead_code)]
    tick_rate: u64,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let paused = Arc::new(AtomicBool::new(false));

        let tick_rate = tick_rate_ms;
        let tick_dur = Duration::from_millis(tick_rate_ms);
        let paused_flag = Arc::clone(&paused);

        tokio::spawn(async move {
            loop {
                if paused_flag.load(Ordering::Relaxed) {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    continue;
                }

                if event::poll(tick_dur).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        if tx.send(AppEvent::Key(key)).is_err() {
                            break;
                        }
                    }
                }
                if tx.send(AppEvent::Tick).is_err() {
                    break;
                }
            }
        });

        Self {
            rx,
            paused,
            tick_rate,
        }
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }
}
