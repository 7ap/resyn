use std::io::Cursor;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use reqwest::blocking::Client;
use rodio::{Decoder, OutputStream, Sink};

use crate::CLI;

pub fn run(rx: Receiver<String>) {
    let client = Arc::new(Client::new());

    while let Some(speech) = rx.recv().ok() {
        let client = client.clone();
        let speaker_id = CLI.speaker_id.clone();

        thread::spawn(move || {
            let now = Instant::now();
            let speech = client
                .get("http://[::1]:5002/api/tts")
                .query(&[("text", speech), ("speaker_id", speaker_id)])
                .send();

            if speech.is_err() {
                error!("failed to synthesize speech, is `tts-server` running?");
                return;
            }

            let speech = speech.unwrap().bytes().unwrap().to_vec();
            let source = Decoder::new(Cursor::new(speech.clone()));

            if source.is_err() {
                error!("failed to synthesize speech, is `COQUI_SPEAKER_ID` valid?");
                return;
            }

            info!("synthesized {} bytes in {:?}", speech.len(), now.elapsed());

            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();

            sink.append(source.unwrap());
            sink.sleep_until_end();
        });
    }
}
