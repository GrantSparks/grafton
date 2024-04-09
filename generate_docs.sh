#!/bin/bash

# Default target directory
target_directory="./target"

# Function to display usage information
usage() {
    echo "Usage: $0 [target_directory]"
    echo "  target_directory  Optional. The target directory to process. Defaults to './target'."
    exit 1
}

# Parse command-line options
if [ "$#" -eq 1 ]; then
    target_directory=$1
elif [ "$#" -gt 1 ]; then
    usage
fi

# Start time
start_time=$(date +%s)

# Delete the pdf directory if it exists
if [ -d "$target_directory/pdf" ]; then
    echo "Deleting existing PDF directory..."
    rm -rf "$target_directory/pdf"
fi

# Ensure the pdf directory exists
mkdir -p "$target_directory/pdf"

# Check if the doc directory exists, and run cargo doc if it doesn't
if [ ! -d "$target_directory/doc" ]; then
    echo "Doc directory does not exist, generating documentation..."
    cargo doc
fi

# Function to convert HTML files to PDF directly
convert_and_merge() {
    local directory=$1
    local output_pdf=$2

    # Initialize an array to hold all file paths
    local files=()

    # Find priority and regular HTML files and add them to the array
    while IFS= read -r file; do
        files+=("$file")
    done < <(find "$directory" -type f \( -name 'index.html' -o -name 'all.html' \) | sort -V)
    
    while IFS= read -r file; do
        files+=("$file")
    done < <(find "$directory" -type f -name '*.html' ! -name 'index.html' ! -name 'all.html' | sort -V)

    # Call wkhtmltopdf with all files, properly handling filenames with periods
    wkhtmltopdf --disable-javascript --no-images --grayscale --load-error-handling ignore --margin-top 5mm --margin-bottom 5mm --margin-left 5mm --margin-right 5mm --dpi 96 -q "${files[@]}" "$output_pdf"
}

# Array of directories to exclude
exclude_dirs=("libc" "digest" "futures" "openssl" "linux_raw_sys" "itertools" "openssl_sys" "futures_util" "regex_automata" "src" "typenum")

# Process each subdirectory in [target]/doc
for doc_dir in "$target_directory"/doc/*; do
    if [ -d "$doc_dir" ]; then
        # Extract the name of the subdirectory to use in the output PDF filename
        dir_name=$(basename "$doc_dir")

        # Check if the directory is in the list of directories to exclude
        if [[ " ${exclude_dirs[@]} " =~ " ${dir_name} " ]]; then
            echo "Skipping excluded directory: $doc_dir"
            continue # Skip this directory
        fi

        output_pdf="$target_directory/pdf/${dir_name}.pdf"
        echo "Converting documentation in $doc_dir to $output_pdf"
        convert_and_merge "$doc_dir" "$output_pdf"
    fi
done

echo "All documentation has been converted to PDF."

# End time
end_time=$(date +%s)

# Calculate and print the time taken
time_taken=$((end_time - start_time))
echo "Time taken: $time_taken seconds"