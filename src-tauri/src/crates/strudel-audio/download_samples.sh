#!/bin/bash
# Download essential drum samples from Dirt-Samples repository

set -e

SAMPLES_DIR="assets/samples"
BASE_URL="https://raw.githubusercontent.com/tidalcycles/Dirt-Samples/master"

# Essential drum samples to download
SAMPLES=(
    "bd/BT0A0A7.wav"
    "bd/BT0A0D0.wav"
    "bd/BT0A0D3.wav"
    "sd/rytm-00-hard.wav"
    "sd/rytm-01-classic.wav"
    "sd/ST0T0S3.wav"
    "hh/000_hh3closedhh.wav"
    "hh/001_hh3crash.wav"
    "hh/002_hh3hit1.wav"
    "cp/HANDCLP0.wav"
    "cp/HANDCLPA.wav"
)

echo "Downloading essential drum samples..."
echo "======================================"

# Create directories
mkdir -p "$SAMPLES_DIR/bd"
mkdir -p "$SAMPLES_DIR/sd"
mkdir -p "$SAMPLES_DIR/hh"
mkdir -p "$SAMPLES_DIR/cp"

# Download each sample
for sample in "${SAMPLES[@]}"; do
    output_path="$SAMPLES_DIR/$sample"
    url="$BASE_URL/$sample"

    echo "Downloading $sample..."
    curl -L -f "$url" -o "$output_path" || echo "Failed to download $sample"
done

echo ""
echo "Download complete!"
echo "Samples saved to: $SAMPLES_DIR/"
echo ""
echo "Sample count:"
ls -R "$SAMPLES_DIR" | grep -E '\.wav$|\.WAV$' | wc -l
