#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

readonly CLR_RESET=$'\033[0m'
readonly CLR_DIM=$'\033[2m'
readonly CLR_BLUE=$'\033[36m'
readonly CLR_GREEN=$'\033[32m'
readonly CLR_YELLOW=$'\033[33m'
readonly CLR_RED=$'\033[31m'
readonly CLR_BOLD=$'\033[1m'

readonly BANNER_FILL='                                                                      '
readonly BAR_WIDTH=28
readonly TARGET="aarch64-unknown-linux-gnu"

MODE="dev"
USE_UPX=0
RUN_AFTER_BUILD=0
BUILD_DIR="debug"
SHADER_TOTAL=1
TEXTURE_TOTAL=0
TEXTURE_CONVERTED=0

usage() {
	printf '%b\n' "${CLR_RED}Usage: ${0##*/} [--release or -r] [--upx] [--run]${CLR_RESET}"
}

help() {
	printf '%b\n' "${CLR_BLUE}Usage: ${0##*/} [--release or -r] [--upx] [--run]${CLR_RESET}"
}

log_info() {
	printf '%b\n' "${CLR_BLUE}$1${CLR_RESET}"
}

log_ok() {
	printf '%b\n' "${CLR_GREEN}$1${CLR_RESET}"
}

log_warn() {
	printf '%b\n' "${CLR_YELLOW}$1${CLR_RESET}"
}

log_error() {
	printf '%b\n' "${CLR_RED}$1${CLR_RESET}"
}

print_banner() {
	local title=$1
	printf '\n%b\n' "${CLR_BLUE}+==============================================================================+${CLR_RESET}"
	printf '%b\n' "${CLR_BLUE}| ${CLR_BOLD}$(printf '%-76s' "$title")${CLR_RESET}${CLR_BLUE} |${CLR_RESET}"
	printf '%b\n\n' "${CLR_BLUE}+==============================================================================+${CLR_RESET}"
}

render_progress() {
	local label=$1
	local done=$2
	local total=$3
	local filled count bar index

	if (( total <= 0 )); then
		total=1
	fi

	if (( done > total )); then
		done=$total
	fi

	filled=$(( done * BAR_WIDTH / total ))
	count="$done/$total"
	if (( done == total )); then
		count="${CLR_GREEN}${count}${CLR_RESET}"
	fi

	bar='['
	for ((index = 0; index < BAR_WIDTH; index++)); do
		if (( index < filled )); then
			bar+="${CLR_GREEN}=${CLR_RESET}"
		else
			bar+="${CLR_DIM}-${CLR_RESET}"
		fi
	done
	bar+=']'

	printf '\r\033[2K\033[1m%b\033[0m %b %b' "$label" "$bar" "$count"
}

progress_finish() {
	printf '\n'
}

get_exe_path() {
	BUILD_DIR="debug"
	if [[ $MODE == "release" ]]; then
		BUILD_DIR="release"
	fi
	EXE_PATH="$REPO_ROOT/target/$BUILD_DIR/vulkanalia_project"
}

get_metadata_exe_path() {
	BUILD_DIR="debug"
	if [[ $MODE == "release" ]]; then
		BUILD_DIR="release"
	fi
	METADATA_EXE_PATH="$REPO_ROOT/target/$BUILD_DIR/mesh_metadata_gen"
}

count_texture_inputs() {
	TEXTURE_TOTAL=0
	shopt -s nullglob
	local texture
	for texture in "$REPO_ROOT"/assets/textures/*.{jpg,jpeg,png,tga,bmp}; do
		if [[ -f "$texture" ]] && get_texture_encode "${texture##*/}" >/dev/null; then
			((TEXTURE_TOTAL++))
		fi
	done
	shopt -u nullglob
}

get_texture_encode() {
	local name_no_ext=${1%.*}
	ENCODE=()

	case "$name_no_ext" in
		*_albedo)
			if [[ $MODE == "release" ]]; then
				ENCODE=(--encode etc1s --qlevel 0 --clevel 5)
			else
				ENCODE=(--encode etc1s --qlevel 0 --clevel 1)
			fi
			;;
		*_normal)
			if [[ $MODE == "release" ]]; then
				ENCODE=(--encode uastc --uastc_quality 4 --uastc_rdo_l 10 --zcmp 16)
			else
				ENCODE=(--encode uastc --uastc_quality 4 --uastc_rdo_l 10 --zcmp 3)
			fi
			;;
		*_metallic)
			if [[ $MODE == "release" ]]; then
				ENCODE=(--encode etc1s --qlevel 128 --clevel 5)
			else
				ENCODE=(--encode etc1s --qlevel 128 --clevel 1)
			fi
			;;
		*_roughness)
			if [[ $MODE == "release" ]]; then
				ENCODE=(--encode etc1s --qlevel 128 --clevel 5)
			else
				ENCODE=(--encode etc1s --qlevel 128 --clevel 1)
			fi
			;;
		*_ao)
			if [[ $MODE == "release" ]]; then
				ENCODE=(--encode uastc --uastc_quality 2 --uastc_rdo_l 10 --zcmp 16)
			else
				ENCODE=(--encode uastc --uastc_quality 2 --uastc_rdo_l 10 --zcmp 3)
			fi
			;;
		*_emissive)
			if [[ $MODE == "release" ]]; then
				ENCODE=(--encode etc1s --qlevel 64 --clevel 5)
			else
				ENCODE=(--encode etc1s --qlevel 64 --clevel 1)
			fi
			;;
		*)
			ENCODE=()
			;;
	esac
}

compile_shader() {
	local shader_index=$1
	local shader_total=$2
	local shader_source=$3
	local shader_output=$4
	:
	slangc "$shader_source" \
		-target spirv \
		-profile spirv_1_3 \
		-emit-spirv-directly \
		-fvk-use-entrypoint-name \
		-entry vertMain \
		-entry fragMain \
		-o "$shader_output"
}

convert_texture() {
	local texture_index=$1
	local texture_total=$2
	local texture_input=$3
	local texture_output=$4
	local texture_name=$5
	shift 5
	local -a texture_encode=("$@")
	:
	ktx --genmipmap --filter lanczos --t2 "${texture_encode[@]}" "$texture_output" "$texture_input"
}

maybe_compress_upx() {
	if [[ $USE_UPX -ne 1 ]]; then
		log_warn "UPX disabled. Skipping compression."
		return 0
	fi

	if [[ ! -f "$EXE_PATH" ]]; then
		log_warn "Release binary not found at $EXE_PATH. Skipping compression."
		return 0
	fi

	log_info "Compressing $EXE_PATH with upx..."
	upx --best "$EXE_PATH"
}

maybe_run_exe() {
	if [[ $RUN_AFTER_BUILD -eq 1 ]]; then
		if [[ -f "$EXE_PATH" ]]; then
			log_info "Running $EXE_PATH..."
			"$EXE_PATH"
		else
			log_warn "Built executable not found at $EXE_PATH. Skipping run."
		fi
	fi
}

main() {
	cd "$REPO_ROOT"

	while [[ $# -gt 0 ]]; do
		case "$1" in
			--release|-r)
				MODE="release"
				shift
				;;
			--upx)
				USE_UPX=1
				shift
				;;
			--run)
				RUN_AFTER_BUILD=1
				shift
				;;
			-h|--help)
				help
				exit 0
				;;
			*)
				usage
				exit 1
				;;
		esac
	done

	print_banner "1/4  Mesher / Model Compression"
	log_ok "Building mesher (generates compressed model buffers)..."
	pushd "$SCRIPT_DIR" >/dev/null
	cargo build --target-dir mesher
	./mesher/debug/mesher "$REPO_ROOT/assets/models" "$REPO_ROOT/assets/models_compressed"
	popd >/dev/null

	print_banner "2/4  Shader Compilation"
	log_ok "Compiling shaders..."
	render_progress "Shader compiling" 0 "$SHADER_TOTAL"
	compile_shader 1 "$SHADER_TOTAL" "$REPO_ROOT/assets/shaders/shader.slang" "$REPO_ROOT/assets/shaders/shader.spv"
	render_progress "Shader compiling" 1 "$SHADER_TOTAL"
	progress_finish
	log_ok "  shader compilation complete"

	count_texture_inputs
	TEXTURE_CONVERTED=0

	if [[ $TEXTURE_TOTAL -eq 0 ]]; then
		log_warn "No matching textures found for conversion."
	else
		print_banner "3/4  Texture Conversion"
		log_ok "Converting textures..."
		render_progress "Texture conversion" 0 "$TEXTURE_TOTAL"
		local_index=0
		shopt -s nullglob
		local texture
		for texture in "$REPO_ROOT"/assets/textures/*.{jpg,jpeg,png,tga,bmp}; do
			if [[ -f "$texture" ]]; then
				((local_index++))
				get_texture_encode "${texture##*/}"
				if (( ${#ENCODE[@]} > 0 )); then
					convert_texture "$local_index" "$TEXTURE_TOTAL" "$texture" "${texture%.*}.ktx2" "${texture##*/}" "${ENCODE[@]}"
					render_progress "Texture conversion" "$local_index" "$TEXTURE_TOTAL"
					TEXTURE_CONVERTED=1
				fi
			fi
		done
		shopt -u nullglob
		progress_finish
		if [[ $TEXTURE_CONVERTED -eq 0 ]]; then
			log_warn "No matching textures found for conversion."
		fi
	fi

	if [[ $MODE == "release" ]]; then
		print_banner "4/4  Final Build (Release)"
		log_ok "Building release..."
		cargo +nightly build -Zbuild-std=std,core,panic_abort -Zunstable-options -Zno-embed-metadata --target "$TARGET" --release
		get_exe_path
		maybe_compress_upx
	else
		print_banner "4/4  Final Build (Dev)"
		log_ok "Building dev..."
		cargo build --target "$TARGET"
		get_exe_path
	fi

	log_ok "Build finished."
	maybe_run_exe
}

main "$@"