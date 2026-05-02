use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    select,
    sync::mpsc::{Receiver, Sender},
};
use tokio_serial::SerialPortBuilderExt;

#[derive(Debug)]
pub enum Event {
    LineReceipt(String, Vec<u8>),
    Closed(String),
}

#[derive(Debug)]
pub enum RequestToDevice {
    Data(Vec<u8>),
}

pub async fn monitor_thread(
    path: String,
    tx: Sender<Event>,
    mut req_rx: Receiver<RequestToDevice>,
) {
    let mut dev = tokio_serial::new(path.clone(), 921600)
        .timeout(std::time::Duration::from_millis(100))
        .open_native_async()
        .expect("Failed to open serial device");

    let mut buf = [0u8; 1024];

    let tx2 = tx.clone();
    let path2 = path.clone();

    loop {
        let mut should_continue_loop = false;
        select! {
            res = dev.read(&mut buf) => {
                if res.is_err() {
                    warn!("Error reading from serial device");
                    break;
                }

                let buf = &buf[..res.unwrap()];
                tx.blocking_send(Event::LineReceipt(path.clone(), buf.to_vec()))
                    .expect("Failed to send line event");
            }
            val = req_rx.recv() => {
                match val {
                    Some(RequestToDevice::Data(data)) => {
                        dev.write(&data).await
                            .expect("Failed to write data to serial device");
                        should_continue_loop  =true
                    }
                    None => {
                        warn!("Error receiving request to device");

                    }
                }
            }
        }

        if !should_continue_loop {
            break;
        }
    }

    tx2.send(Event::Closed(path2.clone()))
        .await
        .expect("Failed to send closed event");
}
