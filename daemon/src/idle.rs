//! User idle detection over the Wayland `ext-idle-notify-v1` protocol.
//!
//! Same protocol Omarchy's own idle service uses, so "idle" here means the same
//! thing as for the screensaver: no keyboard or pointer input from any device
//! (including an external keyboard). A background thread owns the connection;
//! the rest of the daemon asks `is_idle()` and can wait for a resume event.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::{
    self, ExtIdleNotificationV1,
};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1;

struct State {
    idle: Arc<AtomicBool>,
    resumed: mpsc::Sender<()>,
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtIdleNotifierV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ExtIdleNotifierV1,
        _: <ExtIdleNotifierV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtIdleNotificationV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => state.idle.store(true, Ordering::SeqCst),
            ext_idle_notification_v1::Event::Resumed => {
                state.idle.store(false, Ordering::SeqCst);
                let _ = state.resumed.send(());
            }
            _ => {}
        }
    }
}

pub struct IdleWatcher {
    conn: Connection,
    qh: QueueHandle<State>,
    seat: wl_seat::WlSeat,
    notifier: ExtIdleNotifierV1,
    current: Option<(ExtIdleNotificationV1, Duration)>,
    idle: Arc<AtomicBool>,
}

fn err(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::Other, msg.into())
}

impl IdleWatcher {
    /// Connect to the compositor. Returns the watcher and a channel that gets a
    /// message every time the user becomes active again after being idle.
    pub fn start() -> io::Result<(Self, mpsc::Receiver<()>)> {
        let conn = Connection::connect_to_env()
            .map_err(|e| err(format!("cannot connect to the Wayland compositor: {e}")))?;
        let (globals, mut queue) = registry_queue_init::<State>(&conn)
            .map_err(|e| err(format!("Wayland registry: {e}")))?;
        let qh = queue.handle();
        let seat: wl_seat::WlSeat = globals
            .bind(&qh, 1..=1, ())
            .map_err(|e| err(format!("no wl_seat: {e}")))?;
        let notifier: ExtIdleNotifierV1 = globals
            .bind(&qh, 1..=2, ())
            .map_err(|e| err(format!("the compositor lacks ext-idle-notify-v1: {e}")))?;

        let idle = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let mut state = State { idle: idle.clone(), resumed: tx };

        // The dispatch thread owns the event queue for the life of the process.
        std::thread::Builder::new()
            .name("wayland-idle".into())
            .spawn(move || loop {
                if queue.blocking_dispatch(&mut state).is_err() {
                    // Compositor went away: stop reporting idle so lights stay sane.
                    state.idle.store(false, Ordering::SeqCst);
                    break;
                }
            })?;

        Ok((
            Self { conn, qh, seat, notifier, current: None, idle },
            rx,
        ))
    }

    /// Start (or change) the idle threshold. No-op when it is unchanged.
    pub fn watch(&mut self, timeout: Duration) -> io::Result<()> {
        if matches!(&self.current, Some((_, t)) if *t == timeout) {
            return Ok(());
        }
        if let Some((old, _)) = self.current.take() {
            old.destroy();
        }
        self.idle.store(false, Ordering::SeqCst);
        let ms = u32::try_from(timeout.as_millis().max(1)).unwrap_or(u32::MAX);
        // Version 2 has an "input idle" notification that ignores idle inhibitors
        // (a playing video, etc.). We want "no input from the user", not "the
        // screen may sleep", so use it whenever the compositor offers it.
        let n = if self.notifier.version() >= 2 {
            self.notifier.get_input_idle_notification(ms, &self.seat, &self.qh, ())
        } else {
            self.notifier.get_idle_notification(ms, &self.seat, &self.qh, ())
        };
        self.current = Some((n, timeout));
        self.conn
            .flush()
            .map_err(|e| err(format!("Wayland flush: {e}")))
    }

    /// Protocol version the compositor offers (2 = inhibitor-proof input idle).
    pub fn protocol_version(&self) -> u32 {
        self.notifier.version()
    }

    pub fn is_idle(&self) -> bool {
        self.idle.load(Ordering::SeqCst)
    }
}
