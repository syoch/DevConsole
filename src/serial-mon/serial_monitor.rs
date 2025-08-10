use std::sync::mpsc::{self, Sender};

use srobo_base::{
    communication::{AsyncReadableStream, AsyncSerial, SerialDevice},
    utils::fifo::Spsc,
};

pub enum Event {
    LineReceipt(String, String),
    Closed(String),
}

pub fn monitor_thread(path: String, tx: Sender<Event>) {
    let dev = SerialDevice::new(path.clone(), 961200);
    let (mut rd, _td) = dev.open().expect("Failed to open serial device");

    let (line_tx, line_rx) = Spsc::<char, 512>::new();

    let (live_t, live_r) = mpsc::channel();

    let tx2 = tx.clone();
    let path2 = path.clone();
    rd.on_data(Box::new(move |data: &[u8]| {
        for ch in data {
            if *ch == '\r' as u8 {
                continue;
            } else if *ch == '\n' as u8 {
                let mut line = String::new();
                while let Some(c) = line_rx.dequeue() {
                    if *c == '\x1b' || (0x20u8 <= *c as u8 && *c as u8 <= 0x7e) {
                        line.push(*c);
                    } else {
                        line.push_str(format!("\\x{:02X}", *c as u8).as_str());
                    }
                }
                if !line.is_empty() {
                    tx.send(Event::LineReceipt(path.clone(), line))
                        .expect("Failed to send line receipt event");
                }
                continue;
            }
            line_tx.enqueue(*ch as char).unwrap();
        }

        // tx.send(Event::LineReceipt(path.clone(), dump_bytes(x))) .expect("Failed to send line receipt event")
    }))
    .expect("Failed to set data callback");

    rd.on_closed(Box::new(move || {
        live_t.send(()).expect("Failed to send live event");
    }))
    .expect("Failed to set closed callback");

    live_r.recv().expect("Failed to receive live event");

    tx2.send(Event::Closed(path2.clone()))
        .expect("Failed to send closed event");
}
