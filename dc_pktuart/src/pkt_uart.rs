use log::debug;
use tokio::sync::mpsc;

// RD16: パケットのチェックサム計算用構造体
#[derive(Clone, Copy, Debug)]
pub struct RD16 {
    current: u16,
}

impl RD16 {
    pub fn new() -> Self {
        Self { current: 36683 }
    }

    pub fn reset(&mut self) {
        self.current = 36683;
    }

    pub fn update(&mut self, x: u8) {
        self.current ^= x as u16;
        self.current = self.current.wrapping_mul(37003);
    }

    pub fn update_slice<T: AsRef<[u8]>>(&mut self, data: T) {
        for &x in data.as_ref() {
            self.update(x);
        }
    }

    pub fn from_data<T: AsRef<[u8]>>(data: T) -> Self {
        let mut rd16 = Self::new();
        rd16.update_slice(data);
        rd16
    }

    pub fn copy_and_append<T: AsRef<[u8]>>(&self, data: T) -> Self {
        let mut rd16 = *self;
        rd16.update_slice(data);
        rd16
    }

    pub fn get(&self) -> u16 {
        self.current
    }

    pub fn set(&mut self, x: u16) {
        self.current = x;
    }
}

pub struct PktUARTRx {
    rx: mpsc::Receiver<u8>,
}

impl PktUARTRx {
    pub fn new(rx: mpsc::Receiver<u8>) -> Self {
        Self { rx }
    }

    async fn read_u16(&mut self) -> Option<u16> {
        let mut buf = [0u8; 2];
        for byte in buf.iter_mut() {
            *byte = self.rx.recv().await?;
        }
        Some(u16::from_be_bytes(buf))
    }

    async fn read_header(&mut self) -> Option<()> {
        loop {
            if self.rx.recv().await? != 0x55 {
                continue;
            }
            if self.rx.recv().await? != 0xaa {
                continue;
            }
            if self.rx.recv().await? != 0x5a {
                continue;
            }

            break;
        }
        Some(())
    }

    pub async fn read_pkt(&mut self) -> Option<(u8, Vec<u8>)> {
        self.read_header().await?;

        let dest_address = self.rx.recv().await?;
        let len = self.read_u16().await?;
        let _rd16 = self.read_u16().await?;

        let mut buf = Vec::with_capacity(len as usize);
        let mut read_len = 0;
        while read_len < len {
            let b = self.rx.recv().await?;
            buf.push(b);
            read_len += 1;
        }

        Some((dest_address, buf))
    }
}

pub struct PktUARTTx {
    tx: mpsc::Sender<Vec<u8>>,
}

impl PktUARTTx {
    pub fn new(tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self { tx }
    }

    pub async fn send(&self, addr: u8, data: Vec<u8>) {
        let mut packet = Vec::new();
        packet.push(0x55);
        packet.push(0xaa);
        packet.push(0x5a);
        packet.push(addr);

        let len = data.len() as u16;
        packet.extend_from_slice(&len.to_be_bytes());

        let rd16 = RD16::from_data(&data);
        packet.extend_from_slice(&rd16.get().to_be_bytes());

        packet.extend_from_slice(&data);

        self.tx.send(packet).await.unwrap();
    }
}
