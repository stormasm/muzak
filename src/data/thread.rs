use std::{
    io::Cursor,
    sync::mpsc::{Receiver, Sender},
    thread::sleep,
};

use crate::util::rgb_to_bgr;

use super::{
    events::{DataCommand, DataEvent, ImageLayout, ImageType},
    interface::DataInterface,
};

pub struct DataThread {
    commands_rx: Receiver<DataCommand>,
    events_tx: Sender<DataEvent>,
}

impl DataThread {
    /// Starts the data thread and returns the created interface.
    pub fn start<T: DataInterface>() -> T {
        let (commands_tx, commands_rx) = std::sync::mpsc::channel();
        let (events_tx, events_rx) = std::sync::mpsc::channel();

        std::thread::Builder::new()
            .name("data".to_string())
            .spawn(move || {
                let mut thread = DataThread {
                    commands_rx,
                    events_tx,
                };

                thread.run();
            })
            .expect("could not start data thread");

        T::new(commands_tx, events_rx)
    }

    fn run(&mut self) {
        while let Ok(command) = self.commands_rx.recv() {
            match command {
                DataCommand::DecodeImage(data, image_type, layout) => {
                    if self.decode_image(data, image_type, layout).is_err() {
                        self.events_tx
                            .send(DataEvent::DecodeError(image_type))
                            .expect("could not send event");
                    }
                }
            }

            sleep(std::time::Duration::from_millis(10));
        }
    }

    // The only real possible error here is if the image format is unsupported, or the image is
    // corrupt. In either case, there's literally nothing we can do about it, and the only
    // required information is that there was an error. So, we just return `Result<(), ()>`.
    fn decode_image(
        &self,
        data: Box<[u8]>,
        image_type: ImageType,
        image_layout: ImageLayout,
    ) -> Result<(), ()> {
        let mut image = image::io::Reader::new(Cursor::new(data.clone()))
            .with_guessed_format()
            .map_err(|_| ())?
            .decode()
            .map_err(|_| ())?
            .into_rgba8();

        if image_layout == ImageLayout::BGR {
            rgb_to_bgr(&mut image);
        }

        self.events_tx
            .send(DataEvent::ImageDecoded(image, image_type))
            .expect("could not send event");

        Ok(())
    }
}