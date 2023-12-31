use cpal::traits::{DeviceTrait, HostTrait};
use rustfft::{num_complex::Complex, FftPlanner};
use std::{io::stdin, io::stdout, io::BufWriter, io::Write, str, sync::mpsc, thread};
use termion::terminal_size;

fn construct_frequency_vec(sample_rate: f32, fft_buffer_size: usize) -> Vec<f32> {
    // Only return up to buffer_size/2 since FFT is symmetric.
    (0..fft_buffer_size / 2)
        .into_iter()
        .map(|x| sample_rate * (x as f32) / (fft_buffer_size as f32))
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

fn print_spectrum(freq_vec: &[f32], complex_slice: &[Complex<f32>]) {
    let dimensions = terminal_size().expect("Failed to get terminal size.");
    let (width, height) = (dimensions.0 as usize, dimensions.1 as usize);

    // TODO(0xff): Determine a good mapping from fft result value to draw
    // height.
    let scale: f32 = 3.0;
    let chunk_size = complex_slice.len() / width;

    let mut writer = BufWriter::with_capacity((width + 1) * height + 1, stdout());
    let bins: Vec<f32> = complex_slice
        .chunks(chunk_size)
        .into_iter()
        .map(|x| x.iter().fold(0.0, |accum, x| accum + x.norm()))
        .collect();

    for i in (0..height).rev() {
        for j in 0..width {
            let c = if (i as f32 * scale) < bins[j] {
                b"#"
            } else {
                b" "
            };
            writer.write(c).expect("Failed to write space.");
        }
        writer.write(b"\n").expect("Failed to write new line.");
    }
    writer.write(b"\r").expect("Failed to write new line.");
    writer.flush().expect("Failed to flush.");
}

fn main() {
    let device = cpal::default_host()
        .default_input_device()
        .expect("no input device Plug in a mic.");

    let mut supported_configs_range = device
        .supported_input_configs()
        .expect("This device doesn't have an input device. Plug in a mic and try again.");

    let supported_config = supported_configs_range
        .next()
        .expect("The input device does not have any supported configurations.")
        .with_max_sample_rate();

    let (tx, rx) = mpsc::channel();

    // TODO(0xff): clean up stream elegantly when done.
    let stream = device.build_input_stream(
        &supported_config.config(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            tx.send(data.to_vec())
                .expect("Receiver hung up on sender :(");
        },
        // TOOD(0xff): Actually handle errors.
        |err| {},
        None,
    );

    let num_channels = supported_config.config().channels as usize;
    let sample_rate = supported_config.config().sample_rate.0 as f32;

    thread::spawn(move || {
        // TODO(0xff): Programmatically collect BUFFER_SIZE.
        const BUFFER_SIZE: usize = 2048;
        const NUM_BUFFERS: usize = 4;

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(BUFFER_SIZE * NUM_BUFFERS);
        let freq_vec = construct_frequency_vec(sample_rate, BUFFER_SIZE * NUM_BUFFERS);

        let mut buffer_num: usize = 0;

        let mut complex_buffer = [Complex {
            re: 0.0f32,
            im: 0.0f32,
        }; BUFFER_SIZE * NUM_BUFFERS];

        for audio_buffer in rx {
            for group in audio_buffer.chunks(BUFFER_SIZE) {
                if group.len() < BUFFER_SIZE {
                    continue;
                }
                let single_complex_buffer: Vec<Complex<f32>> = group
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
                    // TODO(0xff): Replace discrete max with interpolated value
                    let (max_index, max_norm) =
                        argmax_with_max(&complex_buffer[0..BUFFER_SIZE * NUM_BUFFERS / 2]);
                    // TODO(0xff): Replace this print output with visual feedback with
                    // respect to musical notes.
                    // print!("freq: {}\r", freq_vec[max_index]);
                    // std::io::stdout().flush().expect("Failed to Flush.");
                    let last_frequency_index = BUFFER_SIZE / 12;
                    print_spectrum(&freq_vec, &complex_buffer[0..last_frequency_index])
                }
            }
        }
    });
    std::io::stdout().flush().expect("Failed to Flush.");
    stdin()
        .read_line(&mut String::new())
        .expect("Degenerate input");
}
