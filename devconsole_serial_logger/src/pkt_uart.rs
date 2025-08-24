use tokio::sync::mpsc;

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
        self.rx.recv_many(&mut buf, len as usize).await;

        Some((dest_address, buf))
    }
}
