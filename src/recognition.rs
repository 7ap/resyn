use std::env;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleFormat, SampleRate, StreamConfig};
use dasp::sample::{Sample, ToSample};
use regex::Regex;
use rtrb::{Producer, RingBuffer};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

const WHISPER_SAMPLE_RATE: usize = 16000;
const THREAD_SLEEP_DURATION: Duration = Duration::from_secs(1);

fn data_callback<T: Sample + ToSample<f32>>(data: &[T], producer: &mut Producer<f32>) {
    let data = data.iter().map(|data| data.to_sample()).collect::<Vec<_>>();

    if let Ok(chunk) = producer.write_chunk_uninit(data.len()) {
        chunk.fill_from_iter(data.iter().copied());
    }
}

pub fn run(tx: Sender<String>) {
    let input_device = cpal::default_host()
        .default_input_device()
        .expect("input device should be connected");

    let config = StreamConfig {
        channels: 1,
        sample_rate: SampleRate(WHISPER_SAMPLE_RATE as _),
        buffer_size: BufferSize::Fixed(1024),
    };

    let context = WhisperContext::new(
        env::var("WHISPER_MODEL_PATH")
            .expect("env variable `WHISPER_MODEL_PATH` is not set")
            .as_str(),
    )
    .unwrap();

    let mut state = context.create_state().unwrap();

    let (mut producer, mut consumer) = RingBuffer::new(10 * WHISPER_SAMPLE_RATE);

    let stream = match input_device.default_input_config().unwrap().sample_format() {
        SampleFormat::I8 => input_device.build_input_stream(
            &config,
            move |data: &[f32], _| data_callback(data, &mut producer),
            |err| error!("stream error: {}", err),
            None,
        ),
        SampleFormat::I16 => input_device.build_input_stream(
            &config,
            move |data: &[f32], _| data_callback(data, &mut producer),
            |err| error!("stream error: {}", err),
            None,
        ),
        SampleFormat::I32 => input_device.build_input_stream(
            &config,
            move |data: &[f32], _| data_callback(data, &mut producer),
            |err| error!("stream error: {}", err),
            None,
        ),
        SampleFormat::F32 => input_device.build_input_stream(
            &config,
            move |data: &[f32], _| data_callback(data, &mut producer),
            |err| error!("stream error: {}", err),
            None,
        ),
        _ => panic!("unsupported sample format"),
    }
    .expect("could not build stream");

    stream.play().unwrap();

    loop {
        thread::sleep(THREAD_SLEEP_DURATION);

        let mut data = Vec::new();

        if let Ok(chunk) = consumer.read_chunk(consumer.slots()) {
            let (first, second) = chunk.as_slices();
            data.extend(first);
            data.extend(second);
            chunk.commit_all();
        }

        let mut params = FullParams::new(SamplingStrategy::default());

        params.set_n_threads(12);
        params.set_no_context(true);
        params.set_single_segment(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_language(Some("en"));
        params.set_suppress_blank(true);
        // params.set_no_speech_thold(0.3);

        let now = Instant::now();

        if state.full(params, &data).unwrap() != 0 {
            error!("failed to run the model");
            continue; // TODO: Should we really continue in this case?
        }

        if state.full_n_segments().unwrap() == 0 {
            warn!("no segments found in {:?}", now.elapsed());
            continue;
        }

        let text = state.full_get_segment_text(0).unwrap();
        trace!("full: {:?}", text);

        #[rustfmt::skip]
        let speech = {
            let mut speech = String::from(text.trim());
            speech = String::from(Regex::new(r"\[.*?\]").unwrap().replace_all(&speech, ""));
            speech = String::from(Regex::new(r"\(.*?\)").unwrap().replace_all(&speech, ""));
            speech = String::from(Regex::new(r"[^a-zA-Z0-9\.,\?!\s\:\'\-]").unwrap().replace_all(&speech, ""));
            String::from(speech.trim())
        };

        if speech.is_empty() {
            debug!("recognized no speech in {:?}", now.elapsed());
            continue;
        }

        info!("recognized {:?} in {:?}", speech, now.elapsed());

        if let Err(e) = tx.send(speech) {
            error!("send error: {}", e);
        }
    }
}
