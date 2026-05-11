#!/usr/bin/env bash

set -euo pipefail

MODE="dev"
RUN_AFTER_BUILD=0
TARGET="aarch64-unknown-linux-gnu"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
	echo "Usage: ${0##*/} [--release | -r] [--run] [--target <triple>]"
}

get_exe_path() {
	local build_dir="debug"
	if [[ "$MODE" == "release" ]]; then
		build_dir="release"
	fi
	printf 'target/%s/%s/vulkanalia_project' "$TARGET" "$build_dir"
}

while [[ $# -gt 0 ]]; do
	case "$1" in
		--release|-r)
			MODE="release"
			shift
			;;
		--run)
			RUN_AFTER_BUILD=1
			shift
			;;
		--target)
			shift
			if [[ $# -eq 0 ]]; then
				usage
				exit 1
			fi
			TARGET="$1"
			shift
			;;
		-h|--help)
			usage
			exit 0
			;;
		*)
			usage
			exit 1
			;;
	esac
done

cd "$REPO_ROOT"

echo "Creating and compressing vertex data..."
cd "$SCRIPT_DIR"
cargo build --target-dir mesher
"$SCRIPT_DIR/mesher/debug/mesher" "$REPO_ROOT/assets/models" "$REPO_ROOT/assets/models_compressed"
cd "$REPO_ROOT"

echo "Compiling shaders..."
slangc assets/shaders/shader.slang \
	-target spirv \
	-profile spirv_1_3 \
	-emit-spirv-directly \
	-fvk-use-entrypoint-name \
	-entry vertMain \
	-entry fragMain \
	-o assets/shaders/shader.spv

shopt -s nullglob
texture_converted=0
for texture in assets/textures/*.{jpg,jpeg,png,tga,bmp}; do
	base_name="$(basename "$texture")"
	name_no_ext="${base_name%.*}"
	encode=()

	case "$name_no_ext" in
		*_albedo)
			if [[ "$MODE" == "release" ]]; then
				encode=(--encode etc1s --qlevel 0 --clevel 5)
			else
				encode=(--encode etc1s --qlevel 0 --clevel 1)
			fi
			;;
		*_normal)
			if [[ "$MODE" == "release" ]]; then
				encode=(--encode uastc --uastc_quality 4 --uastc_rdo_l 10 --zcmp 16)
			else
				encode=(--encode uastc --uastc_quality 4 --uastc_rdo_l 10 --zcmp 3)
			fi
			;;
		*_metallic)
			if [[ "$MODE" == "release" ]]; then
				encode=(--encode etc1s --qlevel 128 --clevel 5)
			else
				encode=(--encode etc1s --qlevel 128 --clevel 1)
			fi
			;;
		*_roughness)
			if [[ "$MODE" == "release" ]]; then
				encode=(--encode etc1s --qlevel 128 --clevel 5)
			else
				encode=(--encode etc1s --qlevel 128 --clevel 1)
			fi
			;;
		*_ao)
			if [[ "$MODE" == "release" ]]; then
				encode=(--encode uastc --uastc_quality 2 --uastc_rdo_l 10 --zcmp 16)
			else
				encode=(--encode uastc --uastc_quality 2 --uastc_rdo_l 10 --zcmp 3)
			fi
			;;
		*_emissive)
			if [[ "$MODE" == "release" ]]; then
				encode=(--encode etc1s --qlevel 64 --clevel 5)
			else
				encode=(--encode etc1s --qlevel 64 --clevel 1)
			fi
			;;
	esac

	if [[ ${#encode[@]} -gt 0 ]]; then
		echo "Converting ${base_name}..."
		toktx --genmipmap --filter lanczos --t2 "${encode[@]}" "assets/textures/${name_no_ext}.ktx2" "$texture"
		texture_converted=1
	else
		echo "Skipping ${base_name} (no recognized texture suffix)."
	fi
done

if [[ "$texture_converted" -eq 0 ]]; then
	echo "No matching textures found for conversion."
fi

if [[ "$MODE" == "release" ]]; then
	echo "Building release..."
	cargo +nightly build -Zbuild-std=std,core,panic_abort -Zunstable-options -Zno-embed-metadata --target "$TARGET" --release
else
	echo "Building dev..."
	cargo build --target "$TARGET"
fi

exe_path="$(get_exe_path)"
echo "Done."

if [[ "$RUN_AFTER_BUILD" -eq 1 ]]; then
	if [[ -x "$exe_path" ]]; then
		echo "Running ${exe_path}..."
		"./$exe_path"
	else
		echo "Built executable not found at ${exe_path}. Skipping run."
	fi
fi