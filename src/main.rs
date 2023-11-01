use cpal::traits::{DeviceTrait, HostTrait};
use cpal::StreamConfig;
use rustfft::{num_complex::Complex, FftPlanner};
use std::{io::stdin, sync::mpsc, thread, time};

fn construct_frequency_vec(sample_rate: f32, buffer_size: usize) -> Vec<f32> {
    // Only return up to buffer_size/2 since FFT is symmetric.
    (0..buffer_size / 2)
        .into_iter()
        .map(|x| sample_rate * (x as f32) / (buffer_size as f32))
        .collect()
}

fn argmax_with_max(complex_slice: &[Complex<f32>]) -> (usize, f32) {
    complex_slice.iter().enumerate().fold(
        (0, complex_slice[0].norm()),
        |(idx_max, norm_max), (idx, val)| {
            if norm_max > val.norm() {
                (idx_max, norm_max)
            } else {
                (idx, val.norm())
            }
        },
    )
}

fn main() {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("no input device");

    let mut supported_configs_range = device
        .supported_input_configs()
        .expect("This device doesn't have an input device. Plug in a mic and try again.");

    let supported_config = supported_configs_range
        .next()
        .expect("The input device does not have any supported configurations.")
        .with_max_sample_rate();

    let num_channels = supported_config.config().channels as usize;
    let sample_rate = supported_config.config().sample_rate.0 as f32;

    let (tx, rx) = mpsc::channel();

    let stream = device.build_input_stream(
        &supported_config.config(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            tx.send(data.to_vec())
                .expect("Receiver hung up on sender :(");
        },
        |err| {},
        None,
    );

    thread::spawn(move || {
        const BUFFER_SIZE: usize = 512;
        const NUM_BUFFERS: usize = 8;

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(BUFFER_SIZE * NUM_BUFFERS);
        let freq_vec = construct_frequency_vec(sample_rate, BUFFER_SIZE * NUM_BUFFERS);

        let mut buffer_num: usize = 0;

        let mut complex_buffer = [Complex {
            re: 0.0f32,
            im: 0.0f32,
        }; BUFFER_SIZE * NUM_BUFFERS];

        for audio_buffer in rx {
            let single_complex_buffer: Vec<Complex<f32>> = audio_buffer
                .iter()
                .step_by(num_channels)
                .map(|x| Complex { re: *x, im: 0.0f32 })
                .collect();
            complex_buffer[BUFFER_SIZE * buffer_num..BUFFER_SIZE * (buffer_num + 1)]
                .copy_from_slice(&single_complex_buffer);
            buffer_num = (buffer_num + 1) % NUM_BUFFERS;

            if buffer_num == 0 {
                fft.process(&mut complex_buffer);
                // Symmetric, only need to take first half of buffer.
                let (max_index, max_norm) =
                    argmax_with_max(&complex_buffer[0..BUFFER_SIZE * NUM_BUFFERS / 2]);
                println!(
                    "freq: {}, max_index: {}, max_norm: {}",
                    freq_vec[max_index], max_index, max_norm
                );
            }
        }
    });
    let mut input = String::new();
    stdin().read_line(&mut input).expect("Degenerate input");
}
