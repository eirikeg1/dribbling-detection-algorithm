use dribbling_detection_algorithm::{config::Config, domain::{dataset::Dataset, models::VideoData}, utils::visualizations::visualize_video};
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
    println!("Data Path: {}", config.data.data_path);
    println!("Subsets: {:?} ", config.data.subsets);
    println!("Number of Cores: {}", config.general.num_cores);

    let dataset = Dataset::new(config);
    // let valid_data = dataset
    //     .load_subset(&"valid")
    //     .expect("Error when loading valid");

    let valid_iter = dataset.iter_subset(&"train");
    for (i, video_data) in valid_iter.enumerate() {
        if i % 1 == 0 {
            println!("Processing {}", i);
        }
        let video_data = video_data.unwrap();
        visualize_video(&video_data.dir_path, video_data.annotations.as_slice()).unwrap();
        // println!("VideoData: {:#?}", video_data);
    }

    // visualize
    
}

