use std::{fs, sync::mpsc, thread, time::Duration};

#[derive(Debug)]
pub enum Event {
    DeviceFound(String),
}

pub fn watcher_thread(tx: mpsc::Sender<Event>) {
    let mut found_devices = Vec::new();

    loop {
        let entries = match fs::read_dir("/dev") {
            Ok(entries) => entries,
            Err(e) => {
                error!("Failed to read /dev: {}", e);
                thread::sleep(Duration::from_secs(1));
                continue;
            }
        };

        for entry in entries {
            if let Err(e) = entry {
                error!("Failed to read entry in /dev: {}", e);
                continue;
            }
            let entry = entry.unwrap();

            let fname = entry.file_name();
            let fname = fname.clone().to_string_lossy().to_string();
            if found_devices.contains(&fname) {
                continue; // Already found this device
            }
            if fname.starts_with("ttyACM") || fname.starts_with("ttyUSB") {
                found_devices.push(fname.clone());
                let _ = tx.send(Event::DeviceFound(
                    entry.path().to_string_lossy().to_string(),
                ));
            }
        }

        for device in found_devices.clone() {
            if !fs::metadata(format!("/dev/{}", device)).is_ok() {
                found_devices.retain(|d| d != &device);
            }
        }

        thread::sleep(Duration::from_secs(1));
    }
}
