extern crate clap;
extern crate hound;
extern crate otters_rt;

use hound::{SampleFormat, WavReader, WavWriter};
use otters_rt::conf::AudioConfig;
use otters_rt::Otters;

use std::io::Read;
use std::time;

const MAX_BLOCK_SIZE: usize = 1024;

fn main() {
    let matches = clap::App::new("otters_runner")
        .version("0.1")
        .author("Nick W. <tginick93@gmail.com>")
        .about("Test bed for Otters RT effect units")
        .arg(clap::Arg::with_name("PRINT_AVAILABLE_UNITS")
            .short("p")
            .long("print-available-effects")
            .help("Do nothing except print available effects"))
        .arg(clap::Arg::with_name("CONFIG_FILE")
            .short("c")
            .long("config")
            .help("Otters RT Board Configuration File")
            .takes_value(true)
            .required_unless("PRINT_AVAILABLE_UNITS"))
        .arg(clap::Arg::with_name("WAVE_FILE")
            .short("f")
            .long("wavfile")
            .help("Wave File to process")
            .takes_value(true)
            .required_unless("PRINT_AVAILABLE_UNITS"))
        .arg(clap::Arg::with_name("OUTPUT_FILE")
            .short("o")
            .long("outfile")
            .help("Output wave file")
            .takes_value(true)
            .required_unless("PRINT_AVAILABLE_UNITS"))
    .get_matches();

    if matches.is_present("PRINT_AVAILABLE_UNITS") {
        print_avail_units();
        return;
    }

    let input_wav_name = matches.value_of("WAVE_FILE").unwrap();
    let wavfile = WavReader::open(&input_wav_name);
    if let Err(err) = wavfile {
        println!(
            "ERROR: Failed to open file {}. Err {:?}",
            &input_wav_name, &err
        );
        std::process::exit(1);
    }

    let wavfile = wavfile.unwrap();
    println!("Loading input wav {}", &input_wav_name);

    let input_wav_spec = wavfile.spec();
    if !check_wav_spec(&input_wav_spec) {
        std::process::exit(1);
    }

    let input_wav_samples = load_wav_into_mem(wavfile);

    let otters_conf_name = matches.value_of("CONFIG_FILE").unwrap();
    let otters = Otters::create_default(
        AudioConfig {
            sample_rate: input_wav_spec.sample_rate as f32,
            max_block_size: MAX_BLOCK_SIZE,
        },
        otters_conf_name,
    );

    if let Err(err) = otters {
        println!("ERROR: Failed to initialize otters. {:?}", err);
        std::process::exit(1);
    }

    let otters = otters.unwrap();

    let output_samples = do_processing(otters, input_wav_samples);

    let out_wav_name = matches.value_of("OUTPUT_FILE").unwrap();
    write_wav_to_file(output_samples, &out_wav_name, &input_wav_spec);
}

fn print_avail_units() {
    println!("Effect Info\n");

    let infos = Otters::get_effect_info_json(true);
    println!("{}", infos);

    println!("\n");
}

fn check_wav_spec(spec: &hound::WavSpec) -> bool {
    println!(
        "Sample Rate: {}\nChannels: {}\nType: {:?}, Bits per Sample: {}\n",
        spec.sample_rate,
        spec.channels,
        spec.sample_format,
        spec.bits_per_sample
    );

    if spec.channels > 1 {
        println!("ERROR: Currently only 1 channel is supported");
        return false;
    }

    if spec.sample_format != SampleFormat::Float {
        println!("ERROR: Currently only 32-bit FLOAT is supported");
        return false;
    }

    return true;
}

fn load_wav_into_mem<T: Read>(reader: WavReader<T>) -> Vec<f32> {
    let itr = reader.into_samples();
    let mut dest_vec = Vec::with_capacity(itr.len());

    for sample in itr {
        dest_vec.push(sample.unwrap());
    }

    dest_vec
}

fn write_wav_to_file(data: Vec<f32>, file_name: &str, spec: &hound::WavSpec) {
    let outfile = WavWriter::create(&file_name, *spec);
    if let Err(err) = outfile {
        println!(
            "ERROR: Failed to open output file {}. Err {:?}",
            &file_name, &err
        );
        std::process::exit(1);
    }
    let mut outfile = outfile.unwrap();
    println!("Outputting to {}", &file_name);

    for sample in data.iter() {
        outfile.write_sample(*sample).unwrap();
    }
}

fn do_processing(mut otters: Otters, input_wav_samples: Vec<f32>) -> Vec<f32> {
    let mut input_buf: Vec<f32> = Vec::with_capacity(MAX_BLOCK_SIZE);
    let mut output_buf: Vec<f32> = Vec::with_capacity(MAX_BLOCK_SIZE);

    for _ in 0..MAX_BLOCK_SIZE {
        input_buf.push(0.0f32);
        output_buf.push(0.0f32);
    }

    otters.bind_input(0, input_buf.as_ptr());
    otters.bind_output(0, output_buf.as_mut_ptr());

    let mut samples_processed = 0_usize;
    let mut samples_itr = input_wav_samples.iter();
    let total_samples = input_wav_samples.len();
    let mut output_samples = Vec::with_capacity(total_samples);

    let now = time::Instant::now();
    while samples_processed < total_samples {
        // try to take MAX_BLOCK_SIZE samples
        // if there aren't enough samples left, take whatever's left
        let samples_to_take = MAX_BLOCK_SIZE.min(total_samples - samples_processed);
        for i in 0..samples_to_take {
            input_buf[i] = *samples_itr.next().unwrap();
        }

        otters.frolic(samples_to_take);

        for i in 0..samples_to_take {
            output_samples.push(output_buf[i]);
        }

        samples_processed += samples_to_take;
    }
    let elapsed = now.elapsed();

    let elapsed_seconds = elapsed.as_secs_f32();
    let samples_per_second = if elapsed_seconds == 0.0f32 {
        -1
    } else {
        ((samples_processed as f32) / elapsed_seconds) as i32
    };
    println!(
        "Processing {} samples took: {} ms. Expected samples per second {}",
        total_samples,
        elapsed.as_millis(),
        samples_per_second
    );

    output_samples
}

