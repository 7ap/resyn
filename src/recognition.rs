use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use dasp::sample::{Sample, ToSample};
use vosk::{DecodingState, Model, Recognizer};

use crate::CLI;

fn data_callback<T: Sample + ToSample<i16>>(
    data: &[T],
    recognizer: &mut Recognizer,
    tx: Sender<String>,
) {
    let data = data.iter().map(|data| data.to_sample()).collect::<Vec<_>>();
    let state = recognizer.accept_waveform(&data);

    match state {
        DecodingState::Finalized => {
            let result = recognizer.final_result();
            trace!("result: {:?}", result);

            if let Some(result) = result.single() {
                debug!("result: {:#?}", result);

                if result.text.is_empty() {
                    return;
                }

                info!("recognized {:?}", result.text);

                if let Err(e) = tx.send(String::from(result.text)) {
                    error!("send error: {}", e);
                }
            }
        }
        DecodingState::Running => {
            let result = recognizer.partial_result();
            trace!("result: {:?}", result);
        }
        DecodingState::Failed => {
            error!("failed to decode waveform");
        }
    }
}

pub fn run(tx: Sender<String>) {
    let input_device = cpal::default_host()
        .default_input_device()
        .expect("failed to get default input device");

    let input_config = input_device
        .default_input_config()
        .expect("failed to get default input config");

    let mut config = input_config.config();
    config.channels = 1;

    let model = Model::new(CLI.model_path.to_string_lossy()).unwrap();
    let mut recognizer = Recognizer::new(&model, input_config.sample_rate().0 as f32).unwrap();

    recognizer.set_max_alternatives(0);
    recognizer.set_words(true);
    recognizer.set_partial_words(true);

    let recognizer = Arc::new(Mutex::new(recognizer));

    let stream = match input_config.sample_format() {
        SampleFormat::I8 => input_device.build_input_stream(
            &config,
            move |data: &[i8], _| {
                data_callback(data, &mut recognizer.clone().lock().unwrap(), tx.clone())
            },
            |err| error!("stream error: {}", err),
            None,
        ),
        SampleFormat::I16 => input_device.build_input_stream(
            &config,
            move |data: &[i16], _| {
                data_callback(data, &mut recognizer.clone().lock().unwrap(), tx.clone())
            },
            |err| error!("stream error: {}", err),
            None,
        ),
        SampleFormat::I32 => input_device.build_input_stream(
            &config,
            move |data: &[i32], _| {
                data_callback(data, &mut recognizer.clone().lock().unwrap(), tx.clone())
            },
            |err| error!("stream error: {}", err),
            None,
        ),
        SampleFormat::F32 => input_device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                data_callback(data, &mut recognizer.clone().lock().unwrap(), tx.clone())
            },
            |err| error!("stream error: {}", err),
            None,
        ),
        _ => panic!("unsupported sample format"),
    }
    .expect("could not build stream");

    stream.play().unwrap();

    info!("listening...");

    loop {}
}
