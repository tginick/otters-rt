#![cfg(test)]

use crate::otters::Otters;
use crate::conf::AudioConfig;

use std::path::PathBuf;

fn get_test_resources_directory() -> String {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("resources/test");

    d.display().to_string()
}

#[test]
fn test_load_basic() {
    let mut config_file = PathBuf::from(get_test_resources_directory());
    config_file.push("sample_config.json");

    let load_result = Otters::create_default(
        AudioConfig {
            sample_rate: 44100.0f32,
            max_block_size: 32,
        },
        &config_file.display().to_string(),
    );
    if let Err(err) = load_result {
        println!("Load Failed! {:?}", err);
        assert!(false);
    }
}
