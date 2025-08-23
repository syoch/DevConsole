use tokio::{
    select,
    sync::mpsc::{self, Receiver, Sender},
};

use srobo_base::communication::{AsyncReadableStream, AsyncSerial, SerialDevice, WritableStream};

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
    let dev = SerialDevice::new(path.clone(), 961200);
    let (mut rd, mut td) = dev.open().expect("Failed to open serial device");

    let (live_t, mut live_r) = mpsc::channel(64);

    let tx2 = tx.clone();
    let path2 = path.clone();
    rd.on_data(Box::new(move |data: &[u8]| {
        if !data.is_empty() {
            tx.blocking_send(Event::LineReceipt(path.clone(), data.to_vec()))
                .expect("Failed to send line event");
        }
    }))
    .expect("Failed to set data callback");

    rd.on_closed(Box::new(move || {
        live_t
            .blocking_send(())
            .expect("Failed to send closed signal");
    }))
    .expect("Failed to set closed callback");

    let path3 = path2.clone();
    loop {
        let mut should_continue_loop = false;
        select! {
            val = req_rx.recv() => {
                match val {
                    Some(RequestToDevice::Data(data)) => {
                        info!("Writing data to {}: {:?}", path2, data);
                        td.write(&data)
                            .expect("Failed to write data to serial device");
                        should_continue_loop  =true
                    }
                    None => {
                        warn!("Error receiving request to device");

                    }
                }
            }
            val = live_r.recv() => {
                match val {
                Some(()) => {}
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

    tx2.send(Event::Closed(path3.clone()))
        .await
        .expect("Failed to send closed event");
}
