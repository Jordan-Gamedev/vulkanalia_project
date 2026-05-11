@echo off
setlocal EnableDelayedExpansion

REM ===========================================================================
REM builders\build.bat
REM Windows-only helper to build assets and the `vulkanalia_project` binary.
REM
REM Usage: build.bat [--release | -r] [--upx] [--run]
REM   --release | -r   Build in release mode (default: dev)
REM   --upx             Attempt to compress the final release exe with UPX
REM   --run             Run the built executable after a successful build
REM
REM Notes:
REM  - This script assumes a Windows environment and appends .exe to targets
REM  - Keep this file small and readable; heavy logic is moved to short labels
REM ===========================================================================

for /f %%A in ('echo prompt $E ^| cmd') do set "ESC=%%A"
set "CLR_RESET=%ESC%[0m"
set "CLR_DIM=%ESC%[2m"
set "CLR_BLUE=%ESC%[36m"
set "CLR_GREEN=%ESC%[32m"
set "CLR_YELLOW=%ESC%[33m"
set "CLR_RED=%ESC%[31m"
set "CLR_BOLD=%ESC%[1m"
set "BANNER_FILL=                                                                      "

:: ------------------------- Defaults ---------------------------------------
set "MODE=dev"
set "USE_UPX=0"
set "RUN_AFTER_BUILD=0"
set "SHADER_TOTAL=1"
set "TEXTURE_TOTAL=0"
set "SHADER_DONE=0"
set "TEXTURE_DONE=0"
set "BAR_WIDTH=28"
set "BUILD_DIR=debug"

:: ------------------------- Arg parsing ------------------------------------
if "%~1"=="" goto build

:parse_args
if "%~1"=="" goto build

if /I "%~1"=="--release" (
	set "MODE=release"
	shift
	goto parse_args
)

if /I "%~1"=="-r" (
	set "MODE=release"
	shift
	goto parse_args
)

if /I "%~1"=="--upx" (
	set "USE_UPX=1"
	shift
	goto parse_args
)

if /I "%~1"=="--run" (
	set "RUN_AFTER_BUILD=1"
	shift
	goto parse_args
)

if /I "%~1"=="-h" goto help
if /I "%~1"=="--help" goto help
goto usage

:usage
call :log_error "Usage: %~nx0 [--release or -r] [--upx] [--run]"
exit /b 1

:help
call :log_info "Usage: %~nx0 [--release or -r] [--upx] [--run]"
exit /b 0

:: ------------------------- Helpers ---------------------------------------
:get_exe_path
if /I "%MODE%"=="release" set "BUILD_DIR=release"
set "EXE_PATH=target\%BUILD_DIR%\vulkanalia_project.exe"
exit /b 0

:get_metadata_exe_path
if /I "%MODE%"=="release" set "BUILD_DIR=release"
set "METADATA_EXE_PATH=target\%BUILD_DIR%\mesh_metadata_gen.exe"
exit /b 0

:log_info
echo %CLR_BLUE%%~1%CLR_RESET%
exit /b 0

:log_ok
echo %CLR_GREEN%%~1%CLR_RESET%
exit /b 0

:log_warn
echo %CLR_YELLOW%%~1%CLR_RESET%
exit /b 0

:log_error
echo %CLR_RED%%~1%CLR_RESET%
exit /b 0

:render_progress
set "PROGRESS_LABEL=%~1"
set "PROGRESS_DONE=%~2"
set "PROGRESS_TOTAL=%~3"
if "%PROGRESS_TOTAL%"=="0" set "PROGRESS_TOTAL=1"
set /a PROGRESS_FILLED=PROGRESS_DONE*BAR_WIDTH/PROGRESS_TOTAL
set "PROGRESS_COUNT=%PROGRESS_DONE%/%PROGRESS_TOTAL%"
if "%PROGRESS_DONE%"=="%PROGRESS_TOTAL%" set "PROGRESS_COUNT=%CLR_GREEN%%PROGRESS_DONE%/%PROGRESS_TOTAL%%CLR_RESET%"
set "PROGRESS_BAR=["
for /l %%I in (1,1,%BAR_WIDTH%) do (
	if %%I LEQ !PROGRESS_FILLED! (
		set "PROGRESS_BAR=!PROGRESS_BAR!%CLR_GREEN%=!CLR_RESET!"
	) else (
		set "PROGRESS_BAR=!PROGRESS_BAR!%CLR_DIM%-%CLR_RESET%"
	)
)
set "PROGRESS_BAR=!PROGRESS_BAR!]"
<nul set /p "=%ESC%[2K%ESC%[G%CLR_BOLD%%PROGRESS_LABEL%%CLR_RESET% !PROGRESS_BAR! !PROGRESS_COUNT!"
exit /b 0

:progress_finish
echo(
exit /b 0

:count_texture_inputs
set "TEXTURE_TOTAL=0"
for %%F in (assets\textures\*.jpg assets\textures\*.jpeg assets\textures\*.png assets\textures\*.tga assets\textures\*.bmp) do (
	if exist "%%~fF" (
		set "BASENAME=%%~nF"
		call :get_texture_encode "!BASENAME!"
		if defined ENCODE set /a TEXTURE_TOTAL+=1
	)
)
exit /b 0

:get_texture_encode
set "BASENAME=%~1"
set "ENCODE="

if /I "!BASENAME:~-7!"=="_albedo" (
	set "ENCODE=--encode etc1s --qlevel 0 --clevel"
	if /I "%MODE%"=="release" (
		set "ENCODE=!ENCODE! 5"
	) else (
		set "ENCODE=!ENCODE! 1"
	)
	goto :eof
)

if /I "!BASENAME:~-7!"=="_normal" (
	set "ENCODE=--encode uastc --uastc_quality 4 --uastc_rdo_l 10 --zcmp"
	if /I "%MODE%"=="release" (
		set "ENCODE=!ENCODE! 16"
	) else (
		set "ENCODE=!ENCODE! 3"
	)
	goto :eof
)

if /I "!BASENAME:~-9!"=="_metallic" (
	set "ENCODE=--encode etc1s --qlevel 128 --clevel"
	if /I "%MODE%"=="release" (
		set "ENCODE=!ENCODE! 5"
	) else (
		set "ENCODE=!ENCODE! 1"
	)
	goto :eof
)

if /I "!BASENAME:~-9!"=="_roughness" (
	set "ENCODE=--encode etc1s --qlevel 128 --clevel"
	if /I "%MODE%"=="release" (
		set "ENCODE=!ENCODE! 5"
	) else (
		set "ENCODE=!ENCODE! 1"
	)
	goto :eof
)

if /I "!BASENAME:~-3!"=="_ao" (
	set "ENCODE=--encode uastc --uastc_quality 2 --uastc_rdo_l 10 --zcmp"
	if /I "%MODE%"=="release" (
		set "ENCODE=!ENCODE! 16"
	) else (
		set "ENCODE=!ENCODE! 3"
	)
	goto :eof
)

if /I "!BASENAME:~-9!"=="_emissive" (
	set "ENCODE=--encode etc1s --qlevel 64 --clevel"
	if /I "%MODE%"=="release" (
		set "ENCODE=!ENCODE! 5"
	) else (
		set "ENCODE=!ENCODE! 1"
	)
)
exit /b 0

:compile_shader
set "SHADER_INDEX=%~1"
set "SHADER_TOTAL=%~2"
set "SHADER_SOURCE=%~3"
set "SHADER_OUTPUT=%~4"
C:/VulkanSDK/1.4.335.0/Bin/slangc.exe "%SHADER_SOURCE%" ^
	-target spirv ^
	-profile spirv_1_3 ^
	-emit-spirv-directly ^
	-fvk-use-entrypoint-name ^
	-entry vertMain ^
	-entry fragMain ^
	-o "%SHADER_OUTPUT%"
exit /b %errorlevel%

:convert_texture
set "TEXTURE_INDEX=%~1"
set "TEXTURE_TOTAL=%~2"
set "TEXTURE_INPUT=%~3"
set "TEXTURE_OUTPUT=%~4"
set "TEXTURE_NAME=%~5"
set "TEXTURE_ENCODE=%~6"
toktx --genmipmap --filter lanczos --t2 %TEXTURE_ENCODE% "%TEXTURE_OUTPUT%" "%TEXTURE_INPUT%"
exit /b %errorlevel%

:print_banner
set "BANNER_TITLE=%~1"
echo.
echo %CLR_BLUE%+==============================================================================+%CLR_RESET%
echo %CLR_BLUE%^| %CLR_BOLD%%BANNER_TITLE%%CLR_RESET%%CLR_BLUE%%BANNER_FILL% ^|%CLR_RESET%
echo %CLR_BLUE%+==============================================================================+%CLR_RESET%
echo.
exit /b 0

:: ------------------------- Main build ------------------------------------
:build

REM --- 1) Create and compress vertex data (mesher) -------------------------
call :print_banner "1/4  Mesher / Model Compression"
call :log_ok "Building mesher (generates compressed model buffers)..."
pushd builders
cargo build --target-dir mesher
if errorlevel 1 exit /b %errorlevel%

REM run mesher to generate compressed model assets
.\mesher\debug\mesher.exe ..\assets\models ..\assets\models_compressed
if errorlevel 1 exit /b %errorlevel%
popd

REM --- 2) Compile shaders --------------------------------------------------
set "SHADER_TOTAL=1"
call :print_banner "2/4  Shader Compilation"
call :log_ok "Compiling shaders..."
call :render_progress "Shader compiling" 0 %SHADER_TOTAL%
call :compile_shader 1 1 "assets/shaders/shader.slang" "assets/shaders/shader.spv"
if errorlevel 1 exit /b %errorlevel%
call :render_progress "Shader compiling" 1 %SHADER_TOTAL%
call :progress_finish
call :log_ok "  shader compilation complete"

REM --- 3) Create and compress texture data --------------------------------
call :count_texture_inputs
set "TEXTURE_CONVERTED=0"

if "%TEXTURE_TOTAL%"=="0" (
	call :log_warn "No matching textures found for conversion."
) else (
	call :print_banner "3/4  Texture Conversion"
	call :log_ok "Converting textures..."
	call :render_progress "Texture conversion" 0 %TEXTURE_TOTAL%
	set "TEXTURE_INDEX=0"
	for %%F in (assets\textures\*.jpg assets\textures\*.jpeg assets\textures\*.png assets\textures\*.tga assets\textures\*.bmp) do (
		if exist "%%~fF" (
			set /a TEXTURE_INDEX+=1
			set "BASENAME=%%~nF"
			call :get_texture_encode "!BASENAME!"

			if defined ENCODE (
				call :convert_texture !TEXTURE_INDEX! %TEXTURE_TOTAL% "%%~fF" "assets/textures/!BASENAME!.ktx2" "%%~nxF" "!ENCODE!"
				if errorlevel 1 exit /b !errorlevel!
				call :render_progress "Texture conversion" !TEXTURE_INDEX! %TEXTURE_TOTAL%
				set "TEXTURE_CONVERTED=1"
			)
		)
	)
	call :progress_finish
	if "!TEXTURE_CONVERTED!"=="0" call :log_warn "No matching textures found for conversion."
)

:build_main
REM --- 4) Build main program ------------------------------------------------
if /I "%MODE%"=="release" (
	call :print_banner "4/4  Final Build (Release)"
	call :log_ok "Building release..."
	set "RUSTFLAGS=-C link-arg=/OPT:REF,ICF -C link-arg=/INCREMENTAL:NO -Zlocation-detail=none -Zfmt-debug=none"
	cargo +nightly build -Zbuild-std=std,core,panic_abort -Zunstable-options -Zno-embed-metadata --release
	if errorlevel 1 exit /b %errorlevel%
	call :get_exe_path
	call :maybe_compress_upx
) else (
	call :print_banner "4/4  Final Build (Dev)"
	set "RUSTFLAGS="
	call :log_ok "Building dev..."
	cargo build
	if errorlevel 1 exit /b %errorlevel%
	call :get_exe_path
)

call :log_ok "Build finished."

:: ------------------------- Post-build actions -----------------------------
call :maybe_run_exe

exit /b 0

:: ---------------------------------------------------------------------------
:: Compression helper - attempts to compress using UPX in .\builders
:: ---------------------------------------------------------------------------
:maybe_compress_upx
if /I "%USE_UPX%" NEQ "1" (
	call :log_warn "UPX disabled. Skipping compression."
	goto :eof
)

if not exist ".\builders\upx.exe" if not exist ".\builders\upx" (
	call :log_warn "UPX not found in .\builders. Skipping compression."
	goto :eof
)

if not exist "!EXE_PATH!" (
	call :log_warn "Release binary not found at !EXE_PATH!. Skipping compression."
	goto :eof
)

if exist ".\builders\upx.exe" (
	call :log_info "Compressing !EXE_PATH! with .\builders\upx.exe..."
	.\builders\upx.exe --best "!EXE_PATH!"
	if errorlevel 1 exit /b %errorlevel%
	goto :eof
)

call :log_info "Compressing !EXE_PATH! with .\builders\upx..."
.\builders\upx --best "!EXE_PATH!"
if errorlevel 1 exit /b %errorlevel%
goto :eof

:: ---------------------------------------------------------------------------
:: Run helper - runs the built exe if requested
:: ---------------------------------------------------------------------------
:maybe_run_exe
if "%RUN_AFTER_BUILD%"=="1" (
	if exist "!EXE_PATH!" (
		call :log_info "Running !EXE_PATH!..."
		"!EXE_PATH!"
		if errorlevel 1 exit /b %errorlevel%
	) else (
		call :log_warn "Built executable not found at !EXE_PATH!. Skipping run."
	)
)
goto :eof