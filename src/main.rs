use dribbling_detection_algorithm::domain::data::download_data::download_and_extract_dataset;
use dribbling_detection_algorithm::{
    config::Config, domain::data::dataset::Dataset, utils::visualizations::visualize_or_store_video,
};
use std::env;
use std::fs;
use tokio::runtime::Runtime;

fn main() {
    // Check for command-line arguments
    let args: Vec<String> = env::args().collect();
    let should_download = args.contains(&"--download".to_string());

    if should_download {
        println!("Data download initiated...");
        let config_content =
            fs::read_to_string("config.toml").expect("Unable to read the config file");
        let config: Config =
            toml::from_str(&config_content).expect("Unable to parse the config file");

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

    let video_mode: &String = &config.general.video_mode;
    let output_path: &String = &config.data.output_path;

    let dataset = Dataset::new(config.clone());
    let train_iter = dataset.iter_subset(&"train");

    for (i, video_data) in train_iter.enumerate() {
        if i % 1 == 0 {
            println!("Processing {}", i);
        }
        let video_data = video_data.unwrap();
        let image_dir = video_data.labels.info.im_dir.clone();

        visualize_or_store_video(
            std::path::Path::new(&format!("{}/{}", video_data.dir_path.display(), image_dir.unwrap_or("img1".to_string()))),
            video_data.labels.annotations.as_slice(),
            video_data.labels.images.as_slice(),
            video_mode.as_str(),
            output_path.as_str(),
            &format!("video_{}", i),
            &config,
        )
        .unwrap();
    }
}
