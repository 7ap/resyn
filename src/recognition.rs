use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleFormat, SampleRate, StreamConfig};
use dasp::sample::{Sample, ToSample};
use regex::Regex;
use rtrb::{Producer, RingBuffer};
use webrtc_vad::{SampleRate as VadSampleRate, Vad, VadMode};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

use crate::CLI;

const WHISPER_SAMPLE_RATE: usize = 16000;
const THREAD_SLEEP_DURATION: Duration = Duration::from_millis(25);

#[rustfmt::skip]
const MIN_SAMPLES: usize = (WHISPER_SAMPLE_RATE / THREAD_SLEEP_DURATION.as_millis() as usize) + WHISPER_SAMPLE_RATE;

fn data_callback<T: Sample + ToSample<i16> + ToSample<f32>>(
    data: &[T],
    vad: &mut Vad,
    producer: &mut Producer<f32>,
) {
    let data = data.iter().map(|data| data.to_sample()).collect::<Vec<_>>();

    let mut is_voice_segment = false;

    for chunk in data.chunks_exact(160) {
        if vad.is_voice_segment(chunk).unwrap() {
            is_voice_segment = true;
        }
    }

    if !is_voice_segment {
        return;
    }

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

    let context = WhisperContext::new(&CLI.model_path.to_string_lossy()).unwrap();
    let mut state = context.create_state().unwrap();

    let mut vad = Vad::new_with_rate_and_mode(VadSampleRate::Rate16kHz, VadMode::VeryAggressive);

    let (mut producer, mut consumer) = RingBuffer::new(10 * WHISPER_SAMPLE_RATE);

    let stream = match input_device.default_input_config().unwrap().sample_format() {
        SampleFormat::I8 => input_device.build_input_stream(
            &config,
            move |data: &[f32], _| data_callback(data, &mut vad, &mut producer),
            |err| error!("stream error: {}", err),
            None,
        ),
        SampleFormat::I16 => input_device.build_input_stream(
            &config,
            move |data: &[f32], _| data_callback(data, &mut vad, &mut producer),
            |err| error!("stream error: {}", err),
            None,
        ),
        SampleFormat::I32 => input_device.build_input_stream(
            &config,
            move |data: &[f32], _| data_callback(data, &mut vad, &mut producer),
            |err| error!("stream error: {}", err),
            None,
        ),
        SampleFormat::F32 => input_device.build_input_stream(
            &config,
            move |data: &[f32], _| data_callback(data, &mut vad, &mut producer),
            |err| error!("stream error: {}", err),
            None,
        ),
        _ => panic!("unsupported sample format"),
    }
    .expect("could not build stream");

    stream.play().unwrap();

    let mut last = None;
    let mut data = Vec::new();

    loop {
        thread::sleep(THREAD_SLEEP_DURATION);

        if !consumer.is_empty() {
            if let Ok(chunk) = consumer.read_chunk(consumer.slots()) {
                if last.is_none() {
                    debug!("speech detected, waiting {}ms for silence...", CLI.delay);
                }

                last = Some(Instant::now());

                let (first, second) = chunk.as_slices();
                data.extend(first);
                data.extend(second);
                chunk.commit_all();
            }
        }

        if let Some(last) = last {
            trace!("last spoke {:?} ago ({})", last.elapsed(), data.len());

            if last.elapsed() < Duration::from_millis(CLI.delay as _) {
                continue;
            }
        }

        if data.is_empty() {
            continue;
        }

        if data.len() < MIN_SAMPLES {
            if let Some(threshold) = CLI.interpolate {
                #[rustfmt::skip]
                let min_samples = ((threshold as f32 * MIN_SAMPLES as f32) / 1000.0) as usize;

                if data.len() < min_samples {
                    debug!(
                        "speech with length of {}ms does not meet threshold of {}ms, skipping...",
                        ((data.len() as f32 / MIN_SAMPLES as f32) * 1000.0) as usize,
                        threshold
                    );

                    last = None;
                    data.clear();

                    continue;
                }

                let factor = MIN_SAMPLES as f32 / data.len() as f32;
                let mut interpolated_data = Vec::with_capacity(MIN_SAMPLES);

                for i in 0..MIN_SAMPLES {
                    interpolated_data.push(data[(i as f32 / factor) as usize]);
                }

                debug!(
                    "interpolated {} samples by a factor of {}, continuing...",
                    data.len(),
                    factor,
                );

                data = interpolated_data;
            } else {
                debug!(
                    "needed {} more milliseconds of speech to run recognition, skipping...",
                    1000 - ((data.len() as f32 / MIN_SAMPLES as f32) * 1000.0) as usize
                );

                last = None;
                data.clear();

                continue;
            }
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

        last = None;
        data.clear();

        if state.full_n_segments().unwrap() == 0 {
            debug!("no segments found in {:?}", now.elapsed());
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
