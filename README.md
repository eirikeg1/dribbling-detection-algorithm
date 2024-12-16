# Dribbling Detection Algorithm

A Rust-based implementation for detecting dribbling events on the [SoccerNet Game State Reconstruction 2024](https://www.soccer-net.org/) dataset.

## Dataset

* This tool is designed for the SoccerNet GSR 2024 dataset.
* A simple way to download the data is through the official [Hugging Face dataset](https://huggingface.co/datasets/SoccerNet/SN-GSR-2025).
* The official challenge repository, with other instructions of how to download the data can be found at [sn-gamestate](https://github.com/SoccerNet/sn-gamestate)
* Refer to the official [SoccerNet website](https://www.soccer-net.org/) for more information about soccernet and their challenges/datasets

## Configurations
Adjust paths, parallelism, and other runtime parameters in ```config.toml```.

## Requirements

- **Rust** (latest stable recommended)
- **OpenCV** (3.4 or 4.x)  
  Ensure OpenCV and its development headers are installed on your system. If automatic detection fails, the project’s `build.rs` ensures correct linking of OpenCV (including `stdc++`).


## Installation

1. **Install Rust**:  
   Follow the official [Rust installation guide](https://www.rust-lang.org/tools/install) to set up Rust and Cargo.

2. **Install OpenCV**:  
   Ensure OpenCV (3.4 or 4.x) is installed. For example, on Ubuntu/Debian:
   ```bash
   sudo apt-get update
   sudo apt-get install libopencv-dev pkg-config cmake
   ```
3. **Clone the repository**
    ```bash
    git clone https://github.com/yourusername/dribbling-detection-algorithm.git
    cd dribbling-detection-algorithm
    ```
    At this point, the project should be ready to build and run.


## Building & Running

Simply run:
```bash
cargo run
```
