use cpal::traits::{DeviceTrait, HostTrait};
use cpal::Data;
use std::io::stdin;

fn main() {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("no input device");
    let mut supported_configs_range = device
        .supported_input_configs()
        .expect("error query config.");
    let supported_config = supported_configs_range
        .next()
        .expect("no supported config")
        .with_max_sample_rate();

    let stream = device.build_input_stream(
        &supported_config.config(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            println!("{}", data.iter().fold(0.0f32, |acc, x| acc + x));
        },
        |err| {},
        None,
    );

    let mut input = String::new();
    stdin().read_line(&mut input).expect("Degenerate input");
}
