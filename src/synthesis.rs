use std::sync::mpsc::Receiver;
use std::thread;

pub fn run(rx: Receiver<String>) {
    while let Some(_speech) = rx.recv().ok() {
        thread::spawn(move || {
            // TODO: Synthesize speech.
        });
    }
}
