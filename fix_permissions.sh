#!/bin/bash

# Get all files that are not ignored by git
all_files=$(git ls-files && git ls-files --exclude-standard --others)

total_files=$(echo "$all_files" | wc -l)
current_file=0

# Loop over all files
while IFS= read -r file; do
    let "current_file++"
    echo "Processing $file [$current_file/$total_files]..."
    if [ -L "$file" ]; then
        echo "Skipping symbolic link $file"
        continue
    fi
    case "${file,,}" in
        *.rs | *.toml | *.json | *.yml | *.config | *.md | *.html | */.devcontainer/* | */dockerfile | *.css | *.js | *.gitignore)
            chmod 644 "$file" && echo "Updated permissions for $file to 644" || echo "Failed to update permissions for $file"
            ;;
        *.sh)
            chmod 700 "$file" && echo "Updated permissions for $file to 700" || echo "Failed to update permissions for $file"
            ;;
        *.pem | *.polar | *.sql)
            chmod 600 "$file" && echo "Updated permissions for $file to 600" || echo "Failed to update permissions for $file"
            ;;
        *license*)
            chmod 644 "$file" && echo "Updated permissions for $file to 644" || echo "Failed to update permissions for $file"
            ;;
        *)
            echo "No action taken for $file"
            ;;
    esac
done <<< "$all_files"

echo "Permissions have been updated!"
