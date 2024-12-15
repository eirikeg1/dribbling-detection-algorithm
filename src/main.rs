use dribbling_detection_algorithm::config::Config;
use std::fs;

fn main() {
    // Load the configuration file
    let config_content = fs::read_to_string("config.toml")
        .expect("Unable to read the config file");

    // Parse the TOML file into a Config struct
    let config: Config = toml::from_str(&config_content)
        .expect("Unable to parse the config file");

    // Print the configuration to verify
    println!("{:#?}", config);

    // Example of using config values
    println!("Data Path: {}", config.general.data_path);
    println!("Number of Cores: {}", config.general.num_cores);
}
