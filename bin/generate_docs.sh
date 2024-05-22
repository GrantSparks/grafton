#!/bin/bash

# Navigate to the parent directory of the script's location
cd "$(dirname "$0")/.."

# Default target directory and verbosity
target_directory="./target"
verbose=0

# Function to display usage information
usage() {
    echo "Usage: $0 [-v] [target_directory]"
    echo "  -v                Verbose mode. Prints more about script operations."
    echo "  target_directory  Optional. The target directory to process. Defaults to './target'."
    exit 1
}

# Check for required commands
check_dependencies() {
    local deps=("wkhtmltopdf" "cargo")
    for dep in "${deps[@]}"; do
        command -v "$dep" >/dev/null 2>&1 || { echo "$dep is not installed. Aborting." >&2; exit 1; }
    done
}

# Parse command-line options
while getopts ":v" opt; do
  case ${opt} in
    v )
      verbose=1
      ;;
    \? )
      usage
      ;;
  esac
done
shift $((OPTIND -1))

# Setting the target directory
if [ "$#" -eq 1 ]; then
    target_directory=$1
elif [ "$#" -gt 1 ]; then
    usage
fi

# Log message function
log_message() {
    if [[ $verbose -eq 1 ]]; then
        echo "$1"
    fi
}

start_time=$(date +%s)
check_dependencies

# Function to delete and recreate directories
reset_directory() {
    local dir=$1
    if [ -d "$dir" ]; then
        log_message "Resetting directory: $dir"
        rm -rf "$dir"
    fi
    mkdir -p "$dir"
}

# Reset PDF directory
reset_directory "$target_directory/pdf"

# Generate documentation if needed
if [ ! -d "$target_directory/doc" ]; then
    log_message "Generating documentation..."
    cargo doc
fi

# Function to convert HTML files to PDF
convert_and_merge() {
    local directory=$1
    local output_pdf=$2

    # Find and prioritize specific files, then add remaining HTML files
    local files=(
        $(find "$directory" -type f \( -name 'index.html' -o -name 'all.html' \) -print0 | xargs -0 ls -v)
        $(find "$directory" -type f -name '*.html' ! -name 'index.html' ! -name 'all.html' -print0 | xargs -0 ls -v)
    )

    # Check if any files were found
    if [ ${#files[@]} -eq 0 ]; then
        log_message "No HTML files found in $directory. Skipping PDF generation."
        return
    fi

    log_message "Converting ${#files[@]} files from $directory to $output_pdf"

    # Convert HTML files to PDF using wkhtmltopdf
    wkhtmltopdf --disable-javascript --no-images --grayscale --load-error-handling ignore --margin-top 5mm --margin-bottom 5mm --margin-left 5mm --margin-right 5mm --dpi 96 -q "${files[@]}" "$output_pdf"
}


exclude_dirs=("libc" "digest" "futures" "openssl" "linux_raw_sys" "itertools" "openssl_sys" "futures_util" "regex_automata" "src" "typenum" "hashbrown")

# Process each subdirectory in [target]/doc
for doc_dir in "$target_directory"/doc/*; do
    if [ -d "$doc_dir" ]; then
        dir_name=$(basename "$doc_dir")

        if [[ " ${exclude_dirs[@]} " =~ " ${dir_name} " ]]; then
            log_message "Skipping excluded directory: $doc_dir"
            continue
        fi

        output_pdf="$target_directory/pdf/${dir_name}.pdf"
        log_message "Converting documentation in $doc_dir to $output_pdf"
        convert_and_merge "$doc_dir" "$output_pdf"
    fi
done

log_message "All documentation has been converted to PDF."

end_time=$(date +%s)
time_taken=$((end_time - start_time))
log_message "Total time taken: $time_taken seconds"
