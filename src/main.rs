use dribbling_detection_algorithm::domain::data::download_data::download_and_extract_dataset;
use dribbling_detection_algorithm::{
    config::Config, domain::data::dataset::Dataset, utils::visualizations::visualize_video
};
use std::fs;
use std::env;
use tokio::runtime::Runtime;


fn main() {
    // Check for command-line arguments
    let args: Vec<String> = env::args().collect();
    let should_download = args.contains(&"--download".to_string());

    if should_download {
        println!("Data download initiated...");
        let config_content = fs::read_to_string("config.toml").expect("Unable to read the config file");
        let config: Config = toml::from_str(&config_content).expect("Unable to parse the config file");

        // Use Tokio runtime to run async code
        let rt = Runtime::new().unwrap();
        rt.block_on(download_and_extract_dataset(&config));
        println!("Data download complete.");
    }

    println!("Continuing with regular execution...");
    
    // Load the configuration file
    let config_content = fs::read_to_string("config.toml").expect("Unable to read the config file");
    let config: Config = toml::from_str(&config_content).expect("Unable to parse the config file");

    // Example: Print loaded configuration
    println!("{:#?}", config);

    let dataset = Dataset::new(config);
    let train_iter = dataset.iter_subset(&"train");
    for (i, video_data) in train_iter.enumerate() {
        if i % 1 == 0 {
            println!("Processing {}", i);
        }
        let video_data = video_data.unwrap();
        visualize_video(
            &video_data.dir_path,
            video_data.annotations.as_slice()
        ).unwrap();
    }
}
