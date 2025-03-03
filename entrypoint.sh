#!/bin/bash
# entrypoint.sh: Update .env from config.toml and then run the application

# Run the update_env.sh script to update the .env file
/app/update_env.sh

# Optionally, print the updated config (for debugging)
echo "Updated environment:"
cat .env

# Ensure any necessary directories exist (like output directory)
mkdir -p /data/output

# Now, run application
# exec ./target/release/dribbling-detection-algorithm

# For development
exec cargo watch -x 'build --release' -x 'run'
