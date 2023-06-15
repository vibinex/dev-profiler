import re
import fileinput
import subprocess

# Read the current version from the Cargo.toml file
current_version = ""
with open("Cargo.toml", "r") as cargo_file:
    for line in cargo_file:
        match = re.search(r'^version\s*=\s*"(.*?)"', line)
        if match:
            current_version = match.group(1)
            break

# Generate a new version number (increment the patch version)
version_parts = current_version.split('.')
new_patch = int(version_parts[2]) + 1
new_version = f"{version_parts[0]}.{version_parts[1]}.{new_patch}"

# Update the Cargo.toml file with the new version number
for line in fileinput.input("Cargo.toml", inplace=True):
    line = re.sub(r'^version\s*=\s*".*?"', f'version = "{new_version}"', line.rstrip())
    print(line)

# Update the link to the CLI in the GitHub Action/Bitbucket pipeline file
# for line in fileinput.input(".github/workflows/main.yml", inplace=True):
#     line = re.sub(r'https://github.com/Alokit-Innovations/dev-profiler/releases/download/v.*?',
#                   f'https://github.com/Alokit-Innovations/dev-profiler/releases/download/v{new_version}', line.rstrip())
#     print(line)

# # Commit the changes
# subprocess.run(["git", "add", "Cargo.toml", ".github/workflows/main.yml"])
# subprocess.run(["git", "commit", "-m", f"Update version to {new_version}"])

# # Push the changes
# subprocess.run(["git", "push", "origin", "main"])
