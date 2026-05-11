@echo off
setlocal EnableDelayedExpansion


REM ===============================================================
REM Arguments
REM ===============================================================


set "MODE=dev"
set "USE_UPX=0"
set "RUN_AFTER_BUILD=0"
set "TARGET=x86_64-pc-windows-msvc"

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

if /I "%~1"=="--target" (
	shift
	if "%~1"=="" goto usage
	set "TARGET=%~1"
	shift
	goto parse_args
)

if /I "%~1"=="-h" goto help
if /I "%~1"=="--help" goto help

:usage
echo Usage: %~nx0 [--release ^| -r] [--upx] [--run] [--target ^<triple^>]
exit /b 1

:help
echo Usage: %~nx0 [--release ^| -r] [--upx] [--run] [--target ^<triple^>]
exit /b 0

:get_exe_path
set "BUILD_DIR=debug"
if /I "%MODE%"=="release" set "BUILD_DIR=release"
set "EXE_PATH=target\%TARGET%\%BUILD_DIR%\vulkanalia_project"
echo %TARGET% | findstr /I /C:"windows" >nul
if not errorlevel 1 set "EXE_PATH=!EXE_PATH!.exe"
exit /b 0

:get_metadata_exe_path
set "BUILD_DIR=debug"
if /I "%MODE%"=="release" set "BUILD_DIR=release"
set "METADATA_EXE_PATH=target\%TARGET%\%BUILD_DIR%\mesh_metadata_gen"
echo %TARGET% | findstr /I /C:"windows" >nul
if not errorlevel 1 set "METADATA_EXE_PATH=!METADATA_EXE_PATH!.exe"
exit /b 0


:build

REM ===============================================================
REM Create and Compress Vertex Data
REM ===============================================================


REM Create mesher.exe
cd builders
cargo build --target-dir mesher

REM Run mesher.exe
.\mesher\debug\mesher.exe ..\assets\models ..\assets\models_compressed
cd ..


REM ===============================================================
REM Compile shaders
REM ===============================================================


echo Compiling shaders. . .
C:/VulkanSDK/1.4.335.0/Bin/slangc.exe assets/shaders/shader.slang ^
	-target spirv ^
	-profile spirv_1_3 ^
	-emit-spirv-directly ^
	-fvk-use-entrypoint-name ^
	-entry vertMain ^
	-entry fragMain ^
	-o assets/shaders/shader.spv

if errorlevel 1 exit /b %errorlevel%


REM ===============================================================
REM Create and Compress Texture Data
REM ===============================================================


set "TEXTURE_CONVERTED=0"
for %%F in (assets\textures\*.jpg assets\textures\*.jpeg assets\textures\*.png assets\textures\*.tga assets\textures\*.bmp) do (
	if exist "%%~fF" (
		set "BASENAME=%%~nF"
		set "ENCODE="

		REM Albedo Compression Settings
		if /I "!BASENAME:~-7!"=="_albedo" (
			set "ENCODE=--encode etc1s --qlevel 0 --clevel"

			if /I "%MODE%"=="release" (
				set "ENCODE=!ENCODE! 5"
			) else (
				set "ENCODE=!ENCODE! 1"
			)
		)
		
		REM Normal Compression Settings
		if /I "!BASENAME:~-7!"=="_normal" (
			set "ENCODE=--encode uastc --uastc_quality 4 --uastc_rdo_l 10 --zcmp"

			if /I "%MODE%"=="release" (
				set "ENCODE=!ENCODE! 16"
			) else (
				set "ENCODE=!ENCODE! 3"
			)
		)

		REM Metallic Compression Settings
		if /I "!BASENAME:~-9!"=="_metallic" (
			set "ENCODE=--encode etc1s --qlevel 128 --clevel"

			if /I "%MODE%"=="release" (
				set "ENCODE=!ENCODE! 5"
			) else (
				set "ENCODE=!ENCODE! 1"
			)
		)

		REM Roughness Compression Settings
		if /I "!BASENAME:~-9!"=="_roughness" (
			set "ENCODE=--encode etc1s --qlevel 128 --clevel"

			if /I "%MODE%"=="release" (
				set "ENCODE=!ENCODE! 5"
			) else (
				set "ENCODE=!ENCODE! 1"
			)
		)

		REM Ambient Occlusion Compression Settings
		if /I "!BASENAME:~-3!"=="_ao" (
			set "ENCODE=--encode uastc --uastc_quality 2 --uastc_rdo_l 10 --zcmp"

			if /I "%MODE%"=="release" (
				set "ENCODE=!ENCODE! 16"
			) else (
				set "ENCODE=!ENCODE! 3"
			)
		)

		REM Emissive Compression Settings
		if /I "!BASENAME:~-9!"=="_emissive" (
			set "ENCODE=--encode etc1s --qlevel 64 --clevel"

			if /I "%MODE%"=="release" (
				set "ENCODE=!ENCODE! 5"
			) else (
				set "ENCODE=!ENCODE! 1"
			)
		)

		if defined ENCODE (
			echo Converting %%~nxF with !ENCODE!...
			toktx --genmipmap --filter lanczos --t2 !ENCODE! "assets/textures/!BASENAME!.ktx2" "%%~fF"
			if errorlevel 1 exit /b !errorlevel!
			set "TEXTURE_CONVERTED=1"
		) else (
			echo Skipping %%~nxF ^(no recognized texture suffix^).
		)
	)
)

if "!TEXTURE_CONVERTED!"=="0" echo No matching textures found for conversion.


REM ===============================================================
REM Build Main Program
REM ===============================================================


if /I "%MODE%"=="release" (
	echo Building release...
    set "RUSTFLAGS=-C link-arg=/OPT:REF,ICF -C link-arg=/INCREMENTAL:NO -Zlocation-detail=none -Zfmt-debug=none"
    cargo +nightly build -Zbuild-std=std,core,panic_abort -Zunstable-options -Zno-embed-metadata --target %TARGET% --release
	
	if errorlevel 1 exit /b %errorlevel%

	call :get_exe_path

	if "%USE_UPX%"=="1" (
		if exist ".\builders\upx.exe" (
			if exist "!EXE_PATH!" (
				echo Compressing release executable with UPX...
				.\builders\upx.exe --best "!EXE_PATH!"
				if errorlevel 1 exit /b %errorlevel%
			) else (
				echo Release binary not found at !EXE_PATH!. Skipping compression.
			)
		) else (
			if exist ".\builders\upx" (
				if exist "!EXE_PATH!" (
					echo Compressing release executable with UPX...
					.\builders\upx --best "!EXE_PATH!"
					if errorlevel 1 exit /b %errorlevel%
				) else (
					echo Release binary not found at !EXE_PATH!. Skipping compression.
				)
			) else (
				echo UPX not found. Skipping compression.
			)
		)
	) else (
		echo UPX disabled. Skipping compression.
	)
) else (
    set "RUSTFLAGS="
	echo Building dev...
	cargo build --target %TARGET%
	if errorlevel 1 exit /b %errorlevel%
	call :get_exe_path
)

echo Done.


REM ===============================================================
REM Run Executable
REM ===============================================================


if "%RUN_AFTER_BUILD%"=="1" (
	if exist "!EXE_PATH!" (
		echo Running !EXE_PATH!...
		"!EXE_PATH!"
		if errorlevel 1 exit /b %errorlevel%
	) else (
		echo Built executable not found at !EXE_PATH!. Skipping run.
	)
)

exit /b 0