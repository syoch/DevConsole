use std::{sync::mpsc::Sender, thread::sleep, time::Duration};

use srobo_base::{
    communication::{AsyncReadableStream, AsyncSerial, SerialDevice},
    utils::lined::Lined,
};

enum Event {
    LineReceipt(String),
    Error(),
}
// mermaid
// graph LR
//   A[Start] --> B{x > y}
//   B -->|Yes| C[x]
//   B -->|No| D[y]
// end-mermaid
fn monitor_thread(path: OsString, tx: Sender<Event>) {
    let serial = SerialDevice::new(path.to_string(), 961200);

    let lined = Box::new(Lined::new());

    let (rd, td) = serial.open().expect("Failed to open serial device");
    rd.on_data(|data: &[u8]| {
        lined
            .feed(data)
            .expect("Failed to feed data to lined buffer");
    });

    loop {
        let line = lined.get_line();
        if let None = line {
            sleep(Duration::from_millis(10));
            continue;
        }
        let line = line.unwrap();

        if let Err(e) = tx.send(Event::LineReceipt(line)) {
            error!("Failed to send line: {}", e);
            break;
        }
    }
}
