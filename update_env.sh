#!/bin/bash
# update_env.sh: Extract DATA_PATH and OUTPUT_PATH from config.toml and write them to .env

CONFIG_FILE="config.toml"
ENV_FILE=".env"

# Check if config file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "Error: $CONFIG_FILE not found!"
    exit 1
fi

# Extract the value for data_path.
# It ignores lines where the first non-whitespace character is a '#' (comment)
DATA_PATH=$(grep -E '^[[:space:]]*[^#]' "$CONFIG_FILE" | grep -E 'data_path\s*=' | head -n 1 | sed 's/.*=\s*"\(.*\)".*/\1/')
# Extract the value for output_path.
OUTPUT_PATH=$(grep -E '^[[:space:]]*[^#]' "$CONFIG_FILE" | grep -E 'output_path\s*=' | head -n 1 | sed 's/.*=\s*"\(.*\)".*/\1/')

# Check if values were found
if [ -z "$DATA_PATH" ] || [ -z "$OUTPUT_PATH" ]; then
    echo "Error: Could not extract DATA_PATH or OUTPUT_PATH from $CONFIG_FILE"
    exit 1
fi

# Write the values to the .env file
echo "DATA_PATH=$DATA_PATH" > "$ENV_FILE"
echo "OUTPUT_PATH=$OUTPUT_PATH" >> "$ENV_FILE"

echo ".env file updated:"
cat "$ENV_FILE\n"
