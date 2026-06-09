use meshopt::{quantize_half, quantize_snorm, quantize_unorm};
use std::ffi::OsStr;
use std::fs;
use std::io::{stdout, Result, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const BAR_WIDTH: usize = 28;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
struct QuantizedVertex {
    position: [u16; 3],
    color: [u8; 3],
    normal: [i8; 3],
    uv: [u16; 2],
}

fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        if current.join("Cargo.toml").is_file() && current.join("assets").is_dir() {
            return Some(current);
        }

        if !current.pop() {
            return None;
        }
    }
}

fn set_project_root_cwd() -> Result<()> {
    let start = std::env::current_dir()?;
    if let Some(root) = find_project_root(&start) {
        std::env::set_current_dir(root)?;
        return Ok(());
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(root) = find_project_root(&exe_path) {
            std::env::set_current_dir(root)?;
            return Ok(());
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Unable to locate project root (expected Cargo.toml and assets directory)",
    ))
}

fn main() -> Result<()> {
    set_project_root_cwd()?;

    // Print help info if requested by user
    if check_help() {
        return Ok(())
    }

    // Compile shaders to spirv bytecode
    compile_shaders()?;

    // Convert textures to ktx2 files
    convert_textures()?;

    // Extract and compress vertex and index buffers from meshes
    mesher()?;

    // Create resources
    create_resources()?;

    // Build the program
    build()?;

    // Optionally compress with upx
    if std::env::args().any(|a| a.eq_ignore_ascii_case("--upx")) {
        let exe_path = find_executable()?;
        upx_compress(&exe_path)?;
    }

    // Optionally run the executable
    if std::env::args().any(|a| a.eq_ignore_ascii_case("--run")) {
        if let Ok(exe_path) = find_executable() {
            log_color(&format!("Running {}...", exe_path.display()), ColorType::Blue);
            let _ = Command::new(&exe_path).status();
        }
    }

    Ok(())
}

enum ColorType { Blue, Green, Yellow, Red }
fn log_color(message: &str, color_type: ColorType) {
    let esc_seq = match color_type {
        ColorType::Blue => { "\x1b[36m" },
        ColorType::Green => { "\x1b[32m" },
        ColorType::Yellow => { "\x1b[33m" },
        ColorType::Red => { "\x1b[31m" },
    };
    println!("{}{}", esc_seq, message);
    print!("\x1b[0m");
}

fn print_banner(title: &str) {
    println!();
    println!("+==============================================================================+");
    println!("| {:<76} |", title);
    println!("+==============================================================================+");
    println!();
}

fn render_progress(label: &str, done: usize, total: usize) -> Result<()> {
    let total = total.max(1);
    let done = done.min(total);
    let filled = done * BAR_WIDTH / total;

    let mut bar = String::with_capacity(BAR_WIDTH + 2);
    bar.push('[');
    for index in 0..BAR_WIDTH {
        if index < filled {
            bar.push('=');
        } else {
            bar.push('-');
        }
    }
    bar.push(']');

    let count = if done == total {
        format!("\x1b[32m{done}/{total}\x1b[0m")
    } else {
        format!("{done}/{total}")
    };

    print!("\r\x1b[2K\x1b[1m{label}\x1b[0m {bar} {count}");
    stdout().flush()?;
    Ok(())
}

fn finish_progress() {
    println!();
}

fn check_help() -> bool {
    let mut args  = std::env::args();

    if args.any(|a| a.eq_ignore_ascii_case("--help") || a.eq_ignore_ascii_case("-h")) {
        log_color("Usage: build <Options>\n\t
        --help | -h\tList usage info
        --release\tBuild in release mode (default: dev)\n\t
        --upx\tAttempt to compress the final release exe with UPX\n\t
        --run\tRun the built executable after a successful build", ColorType::Blue);
        return true
    }

    return false
}

fn find_executable() -> Result<PathBuf> {
    let target_dir = if std::env::args().any(|a| a.eq_ignore_ascii_case("--release")) {
        "target/release"
    } else {
        "target/debug"
    };

    let mut exe_name = String::from("vulkanalia_project");
    if cfg!(windows) {
        exe_name.push_str(".exe");
    }

    let exe_path = PathBuf::from(format!("{}/{}", target_dir, exe_name));

    if exe_path.exists() {
        Ok(exe_path)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Executable not found at {}", exe_path.display()),
        ))
    }
}

fn compile_shaders() -> Result<()> {
    print_banner("1/5  Shader Compilation");
    let shader_paths = traverse_directory("assets/shaders", vec!["slang"])?;
    if shader_paths.is_empty() {
        log_color("No shader files found", ColorType::Yellow);
        return Ok(())
    }

    render_progress("Shader compiling", 0, shader_paths.len())?;

    let mut failed_shaders: Vec<&str> = Vec::new();
    let slang_command = cfg_select! {
        windows => { "C:/VulkanSDK/1.4.335.0/Bin/slangc.exe" },
        _ => { "slangc" }
    };

    for (index, shader_path) in shader_paths.iter().enumerate() {
        match Command::new(slang_command)
            .args([
                "-target", "spirv",
                "-profile", "spirv_1_3",
                "-emit-spirv-directly",
                "-fvk-use-entrypoint-name",
                "-entry", "vertMain",
                "-entry", "fragMain",
                "-o",
            ])
            .arg(shader_path.with_extension("spv"))
            .arg(shader_path)
            .output() {
                Ok(m) =>
                {
                    if m.status.code().map_or(false, |c| c != 0) {
                        let msg = String::from_utf8_lossy(&m.stderr).to_string();
                        log_color(&msg, ColorType::Red);
                        failed_shaders.push(shader_path.to_str().unwrap())
                    }
                },
                Err(e) => { log_color(&format!("\n{}", e), ColorType::Red); failed_shaders.push(shader_path.to_str().unwrap()) }
            };

        render_progress("Shader compiling", index + 1, shader_paths.len())?;
    }
    finish_progress();

    if failed_shaders.len() == 0 {
        log_color("Shader compilation finished successfully.", ColorType::Green);
    } else {
        log_color(&format!("{}/{} shaders failed!", failed_shaders.len(), shader_paths.len()), ColorType::Red);
        for (i, failed) in failed_shaders.iter().enumerate() {
            log_color(&format!("\t[{}]: {}", {i}, {failed}), ColorType::Red);
        }
        println!();
    }

    Ok(())
}

fn convert_textures() -> Result<()> {
    print_banner("2/5  Texture Conversion");
    let texture_paths = traverse_directory("assets/textures", vec!["png", "jpg", "jpeg", "tga", "bmp"])?;
    if texture_paths.is_empty() {
        log_color("No texture files found", ColorType::Yellow);
        return Ok(())
    }

    render_progress("Texture conversion", 0, texture_paths.len())?;

    let ktx_command = cfg_select! {
        windows => { "toktx" },
        _ => { "ktx" }
    };
    let base_args = vec![
        "--genmipmap",
        "--filter", "mitchell",
        "--t2"
    ];

    let etc1s_quality = if std::env::args().any(|a| a.eq_ignore_ascii_case("--release")) { vec!["--clevel", "5"] } else { vec!["--clevel", "1"] };
    let uastc_quality = if std::env::args().any(|a| a.eq_ignore_ascii_case("--release")) { vec!["--zcmp", "16"] } else { vec!["--zcmp", "3"] };

    let mut failed_conversions: Vec<&str> = Vec::new();

    for (index, tex_path) in texture_paths.iter().enumerate() {
        let tex_path = tex_path.to_str().unwrap();

        let encode_args = match tex_path {
            s if s.contains("_albedo") => {
                vec!["--encode", "uastc", "--uastc_quality", "3", "--uastc_rdo_l", "1.0", uastc_quality[0], uastc_quality[1]]
            },
            s if s.contains("_normal") => {
                vec!["--encode", "uastc", "--uastc_quality", "4", "--uastc_rdo_l", "0.75", uastc_quality[0], uastc_quality[1]]
            },
            s if s.contains("_metallic") || s.contains("_roughness") => {
                vec!["--encode", "etc1s", "--qlevel", "128", etc1s_quality[0], etc1s_quality[1]]
            },
            s if s.contains("_ao") => {
                vec!["--encode", "uastc", "--uastc_quality", "2", "--uastc_rdo_l", "2.0", uastc_quality[0], uastc_quality[1]]
            },
            s if s.contains("_emissive") => {
                vec!["--encode", "etc1s", "--qlevel", "64", etc1s_quality[0], etc1s_quality[1]]
            },
            _ => {
                vec![]
            },
        };

        let mut final_args = base_args.clone();
        final_args.extend(encode_args);
        let file_output = format!("{}.ktx2", tex_path.split(".").next().unwrap());
        final_args.push(file_output.as_str());
        final_args.push(tex_path);

        match Command::new(ktx_command).args(final_args).output() {
                Ok(m) => {
                    if m.status.code().map_or(false, |c| c != 0) {
                        let msg = String::from_utf8_lossy(&m.stderr).to_string();
                        log_color(&msg, ColorType::Red);
                        failed_conversions.push(tex_path)
                    }
                },
                Err(e) => { log_color(&format!("\n{}", e), ColorType::Red); failed_conversions.push(tex_path) }
            };

        render_progress("Texture conversion", index + 1, texture_paths.len())?;
    }
    finish_progress();

    if failed_conversions.len() == 0 {
        log_color("Texture conversion finished successfully.", ColorType::Green);
    } else {
        log_color(&format!("{}/{} textures failed!", failed_conversions.len(), texture_paths.len()), ColorType::Red);
        for (i, failed) in failed_conversions.iter().enumerate() {
            log_color(&format!("\t[{}]: {}", i, failed), ColorType::Red);
        }
        println!();
    }

    Ok(())
}

fn mesher() -> Result<()> {
    print_banner("3/5  Mesh Extraction");

    let source_root_dir = "assets/models";
    let paths = traverse_directory(&source_root_dir, vec!["glb"])?;
    if paths.is_empty() {
        log_color("No model files found", ColorType::Yellow);
        return Ok(());
    }

    let dest_root_dir = "assets/models_compressed";
    if let Err(e) = fs::remove_dir_all(&dest_root_dir) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!("{e}");
        }
    }

    render_progress("Meshing models", 0, paths.len())?;

    let mut fail_count = 0;

    for (index, path) in paths.iter().enumerate() {
        match gltf::import(path) {
            Ok((glb, buffers, _)) => {
                let (verts, inds) = compress_glb(glb, buffers)?;
                write_mesh_to_file(
                    &source_root_dir.to_string(),
                    &dest_root_dir.to_string(),
                    path.parent().unwrap().as_os_str(),
                    &path.file_stem().unwrap(),
                    verts,
                    inds,
                )?;
            }
            Err(e) => {
                eprintln!("Failed to load {}: {e}", path.display()); fail_count += 1;
            }
        }

        render_progress("Meshing models", index + 1, paths.len())?;
    }
    finish_progress();
    
    if fail_count == 0 {
        log_color("Mesh extraction finished successfully.", ColorType::Green);
    } else {
        log_color(&format!("Mesh extraction finished with {} errors", fail_count), ColorType::Red);
    }

    Ok(())
}

fn create_resources() -> Result<()> {
    fn file_size_as_string(size: &usize) -> String {
        if *size >= 1024_usize.pow(3) {
            format!("{:.2} GiB", (*size as f32 / 1024.0_f32.powf(3.0)) as f32)
        } else if *size >= 1024_usize.pow(2) {
            format!("{:.2} MiB", (*size as f32 / 1024.0_f32.powf(2.0)) as f32)
        } else if *size >= 1024_usize {
            format!("{:.2} KiB", (*size as f32 / 1024.0_f32.powf(1.0)) as f32)
        } else {
            format!("{} B", *size)
        }
    }

    let mut resource_file_contents: String ="// |                                            |\n".to_string();
    resource_file_contents.push_str("// | This file is auto-generated by the builder |\n");
    resource_file_contents.push_str("// |                                            |\n\n");
    resource_file_contents.push_str("#![allow(dead_code)]\n\n");
    resource_file_contents.push_str("#[repr(align(4096))]\n");
    resource_file_contents.push_str("pub struct AlignedAsset(pub &'static[u8]);\n\n");
   
    resource_file_contents.push_str("// ------------KTX2 Textures------------\n\n");

    let texture_resources = traverse_directory("assets", vec!["ktx2"])?;
    let mut texture_file_names: Vec<String> = Vec::new();

    for resource in texture_resources {
        let resource_path = resource.to_string_lossy().replace("\\", "/");
        let resource_path = resource_path.as_str();
        let file_name = format!("{}_T", resource.file_stem().unwrap().to_ascii_uppercase().into_string().unwrap());
        let byte_size = fs::read(resource_path).unwrap().len();
        resource_file_contents.push_str(&format!("// {}\n", &file_size_as_string(&byte_size)));
        resource_file_contents.push_str(format!("pub const {}: AlignedAsset = AlignedAsset(include_bytes!(\"../{}\").as_slice());\n", file_name, resource_path).as_str());
        texture_file_names.push(file_name);
    }

    resource_file_contents.push_str("\n// ------------Model Vertices-----------\n\n");

    let vertex_resources = traverse_directory("assets", vec!["vertbuff"])?;
    let mut vertex_file_names: Vec<String> = Vec::new();
    
    for resource in vertex_resources {
        let resource_path = resource.to_string_lossy().replace("\\", "/");
        let resource_path = resource_path.as_str();
        let file_name = format!("{}_V", resource.file_stem().unwrap().to_ascii_uppercase().into_string().unwrap());
        let byte_size = fs::read(resource_path).unwrap().len();
        resource_file_contents.push_str(&format!("// {}\n", &file_size_as_string(&byte_size)));
        resource_file_contents.push_str(format!("pub const {}: AlignedAsset = AlignedAsset(include_bytes!(\"../{}\").as_slice());\n", file_name, resource_path).as_str());
        vertex_file_names.push(file_name);
    }

    resource_file_contents.push_str("\n// ------------Model Indices------------\n\n");

    let index_resources = traverse_directory("assets", vec!["indbuff"])?;
    let mut index_file_names: Vec<String> = Vec::new();

    for resource in index_resources {
        let resource_path = resource.to_string_lossy().replace("\\", "/");
        let resource_path = resource_path.as_str();
        let file_name = format!("{}_I", resource.file_stem().unwrap().to_ascii_uppercase().into_string().unwrap());
        let byte_size = fs::read(resource_path).unwrap().len();
        resource_file_contents.push_str(&format!("// {}\n", &file_size_as_string(&byte_size)));
        resource_file_contents.push_str(format!("pub const {}: AlignedAsset = AlignedAsset(include_bytes!(\"../{}\").as_slice());\n", file_name, resource_path).as_str());
        index_file_names.push(file_name);
    }

    // Create asset ids
    resource_file_contents.push_str("\n#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]\n");
    resource_file_contents.push_str("pub enum AssetId {\n");
    resource_file_contents.push_str("\t#[default] None,\n");

    let mut enum_texture_names: Vec<String> = Vec::new();
    for tex_name in texture_file_names.clone() {
        // Split name into individual words
        let mut name_words: Vec<String> = tex_name.split("_").map(|s| s.to_ascii_lowercase().to_string()).collect();
        
        // Remove asset type suffix
        name_words.remove(name_words.len() - 1);

        // Capitalize first letter of every word
        for name in &mut name_words {
            name.get_mut(0..1).unwrap().make_ascii_uppercase();
        }

        // Add asset type suffix
        name_words.push("Texture".to_string());

        let tex_name = name_words.concat();

        // Add the enum name
        resource_file_contents.push_str(&format!("\t{},\n", tex_name));
        enum_texture_names.push(tex_name);
    }

    let mut enum_vertex_names: Vec<String> = Vec::new();
    for vert_name in vertex_file_names.clone() {
        // Split name into individual words
        let mut name_words: Vec<String> = vert_name.split("_").map(|s| s.to_ascii_lowercase().to_string()).collect();
        
        // Remove asset type suffix
        name_words.remove(name_words.len() - 1);

        // Capitalize first letter of every word
        for name in &mut name_words {
            name.get_mut(0..1).unwrap().make_ascii_uppercase();
        }

        // Add asset type suffix
        name_words.push("Vertices".to_string());

        let vert_name = name_words.concat();

        // Add the enum name
        resource_file_contents.push_str(&format!("\t{},\n", vert_name));
        enum_vertex_names.push(vert_name);
    }

    let mut enum_index_names: Vec<String> = Vec::new();
    for index_name in index_file_names.clone() {
        // Split name into individual words
        let mut name_words: Vec<String> = index_name.split("_").map(|s| s.to_ascii_lowercase().to_string()).collect();
        
        // Remove asset type suffix
        name_words.remove(name_words.len() - 1);

        // Capitalize first letter of every word
        for name in &mut name_words {
            name.get_mut(0..1).unwrap().make_ascii_uppercase();
        }

        // Add asset type suffix
        name_words.push("Indices".to_string());

        let index_name = name_words.concat();
        
        // Add the enum name
        resource_file_contents.push_str(&format!("\t{},\n", index_name));
        enum_index_names.push(index_name);
    }

    resource_file_contents.push_str("}\n\n");
    resource_file_contents.push_str("pub fn get_asset_from_id(id: AssetId) -> AlignedAsset {\n");
    resource_file_contents.push_str("\tmatch id {\n");
    resource_file_contents.push_str("\t\tAssetId::None => {\n");
    resource_file_contents.push_str("\t\t\tAlignedAsset(&[])\n");
    resource_file_contents.push_str("\t\t},\n");

    for i in 0..texture_file_names.len() {
        resource_file_contents.push_str(&format!("\t\tAssetId::{} => {{\n", enum_texture_names[i]));
        resource_file_contents.push_str(&format!("\t\t\t{}\n", texture_file_names[i]));
        resource_file_contents.push_str("\t\t},\n");
    }

    for i in 0..vertex_file_names.len() {
        resource_file_contents.push_str(&format!("\t\tAssetId::{} => {{\n", enum_vertex_names[i]));
        resource_file_contents.push_str(&format!("\t\t\t{}\n", vertex_file_names[i]));
        resource_file_contents.push_str("\t\t},\n");
    }

    for i in 0..index_file_names.len() {
        resource_file_contents.push_str(&format!("\t\tAssetId::{} => {{\n", enum_index_names[i]));
        resource_file_contents.push_str(&format!("\t\t\t{}\n", index_file_names[i]));
        resource_file_contents.push_str("\t\t},\n");
    }
    
    resource_file_contents.push_str("\t}\n");
    resource_file_contents.push_str("}\n");

    fs::write("src/resources.rs", resource_file_contents)
}

fn build() -> Result<()> {
    let cargo_command = "build";

    if std::env::args().any(|a| a.eq_ignore_ascii_case("--release")) {
        print_banner("4/5  Build App (Release)");
        log_color("Building release...", ColorType::Blue);

        match Command::new("cargo")
            .args(vec![
                "+nightly",
                cargo_command,
                "-Zbuild-std=std,core,panic_abort",
                "-Zunstable-options",
                "-Zno-embed-metadata",
                "--release",
            ])
            .status() {
                Ok(_) => { log_color("Build finished.", ColorType::Green) },
                Err(_) => { log_color("Build failed!", ColorType::Red); },
            }
    } else {
        print_banner("4/5  Build App (Dev)");
        log_color("Building dev...", ColorType::Blue);

        match Command::new("cargo")
            .arg(cargo_command)
            .status() {
                Ok(_) => { log_color("Build finished.", ColorType::Green) },
                Err(_) => { log_color("Build failed!", ColorType::Red); },
            }
    }

    Ok(())
}

fn upx_compress(exe_path: &PathBuf) -> Result<()> {
    print_banner("5/5  UPX Compression");
    log_color("Compressing executable with UPX...", ColorType::Blue);

    let upx_command = cfg_select! {
        windows => { &format!("{}\\builders\\upx.exe", std::env::current_dir().unwrap().as_path().to_str().unwrap()) },
        _ => { "upx" }
    };

    match Command::new(upx_command)
        .args(&["--best", exe_path.to_str().unwrap()])
        .status() {
            Ok(status) => {
                if status.success() {
                    log_color("UPX compression finished successfully.", ColorType::Green);
                } else {
                    log_color("UPX compression failed!", ColorType::Red);
                }
            },
            Err(e) => {
                log_color(&format!("UPX compression error: {}", e), ColorType::Red);
            }
        }

    Ok(())
}

fn compress_glb(glb: gltf::Document, buffers: Vec<gltf::buffer::Data>) -> Result<(Option<Vec<u8>>, Option<Vec<u8>>)> {
    let mut glb_vertices: Vec<Vertex> = vec![];
    let mut glb_indices: Vec<u32> = vec![];
    
    for mesh in glb.meshes() {
        for primitive in mesh.primitives() {
            // Get the index offset
            let index_offset = glb_vertices.len() as u32;

            // Get the vertices and indices from the primitive
            let (new_vertices, new_indices) = get_data_from_primitive(buffers.clone(), primitive)?;

            // Add the vertices and indices to the totals
            glb_vertices.extend(new_vertices);
            glb_indices.extend(new_indices.into_iter().map(|index| index + index_offset));
        }
    }

    // Remap vertex and index data
    let (vertex_count, remapped) = meshopt::generate_vertex_remap(glb_vertices.as_slice(), Some(glb_indices.as_slice()));
    glb_indices = meshopt::remap_index_buffer(Some(glb_indices.as_slice()), vertex_count, &remapped);
    glb_vertices = meshopt::remap_vertex_buffer(glb_vertices.as_slice(), vertex_count, &remapped);

    // Optimize vertex cache
    meshopt::optimize_vertex_cache(glb_indices.as_slice(), vertex_count);

    // Optimize vertex fetch
    glb_vertices = meshopt::optimize_vertex_fetch(&mut glb_indices, glb_vertices.as_slice());

    // Quantization
    let quantized_vertices = glb_vertices.iter().map(|v| {
        QuantizedVertex {
            position: [
                quantize_half(v.position[0]),
                quantize_half(v.position[1]),
                quantize_half(v.position[2]),
            ],
            color: [
                quantize_unorm(v.color[0], 8) as u8,
                quantize_unorm(v.color[1], 8) as u8,
                quantize_unorm(v.color[2], 8) as u8,
            ],
            normal: [
                quantize_snorm(v.normal[0], 8) as i8,
                quantize_snorm(v.normal[1], 8) as i8,
                quantize_snorm(v.normal[2], 8) as i8,
            ],
            uv: [
                quantize_unorm(v.uv[0], 16) as u16,
                quantize_unorm(v.uv[1], 16) as u16,
            ]
        }
    }).collect::<Vec<QuantizedVertex>>();

    // Compress vertices
    let vertex_bytes = match meshopt::encode_vertex_buffer(&quantized_vertices) {
        Ok(comp_verts) => comp_verts,
        Err(e) => {
            eprintln!("Failed to compress vertex data! {e}");
            return Ok((None, None))
        }
    };

    // Compress indices
    let index_bytes = match meshopt::encode_index_buffer(&glb_indices, vertex_count) {
        Ok(comp_indices) => comp_indices,
        Err(e) => {
            eprintln!("Failed to compress index data! {e}");
            return Ok((None, None))
        }
    };
    
    // Embed vertex count
    let mut final_vertex_bytes: Vec<u8> = vec![];
    final_vertex_bytes.extend((vertex_count as u32).to_be_bytes());
    final_vertex_bytes.extend(vertex_bytes);

    // Embed index count
    let mut final_index_bytes: Vec<u8> = vec![];
    final_index_bytes.extend((glb_indices.len() as u32).to_be_bytes());
    final_index_bytes.extend(index_bytes);

    Ok((Some(final_vertex_bytes), Some(final_index_bytes)))
}

fn write_mesh_to_file(source_root_dir: &String, dest_root_dir: &String, source_path_dir: &OsStr, source_path_file_name: &OsStr, vertices: Option<Vec<u8>>, indices: Option<Vec<u8>>) -> Result<()> {
    let vertices = vertices.unwrap();
    let indices = indices.unwrap();

    let dest_path_dir = {
        let source_path_string = String::from(source_path_dir.to_str().unwrap());
        source_path_string.replace(source_root_dir.as_str(), dest_root_dir.as_str())
    };

    // Create a new directory
    match fs::create_dir_all(&dest_path_dir) {
        Ok(_) => {},
        Err(e) => eprintln!("{e}"),
    };

    // Write vertex contents to file
    let file_name = format!("{}{}{}.vertbuff", dest_path_dir.as_str(), std::path::MAIN_SEPARATOR, source_path_file_name.to_str().unwrap());
    if let Err(e) = fs::write(file_name, vertices.as_slice()) {
        eprintln!("{e}");
    }

    // Write index contents to file
    let file_name = format!("{}{}{}.indbuff", dest_path_dir.as_str(), std::path::MAIN_SEPARATOR, source_path_file_name.to_str().unwrap());
    if let Err(e) = fs::write(file_name, indices.as_slice()) {
        eprintln!("{e}");
    }

    Ok(())
}

fn get_data_from_primitive(buffers: Vec<gltf::buffer::Data>, primitive: gltf::Primitive) -> Result<(Vec<Vertex>, Vec<u32>)> {
    
    // Read all the vertex attributes
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    let _tangents = reader.read_tangents();
    let mut positions = reader.read_positions().unwrap();
    let colors = reader.read_colors(0);
    let normals = reader.read_normals();
    let tex_coords = reader.read_tex_coords(0);
    
    // Get indices
    let indices = match reader.read_indices().unwrap() {
        gltf::mesh::util::ReadIndices::U8(iter) => iter.map(|i| i as u32).collect::<Vec<_>>(),
        gltf::mesh::util::ReadIndices::U16(iter) => iter.map(|i| i as u32).collect::<Vec<_>>(),
        gltf::mesh::util::ReadIndices::U32(iter) => iter.collect::<Vec<_>>(),
    };

    // Prepare space for incoming vertices
    let num_verts = &positions.len();
    let mut vertices: Vec<Vertex> = vec![];
    vertices.reserve(*num_verts);
    
    // Create vertices from the data
    for i in 0..*num_verts {
        
        // Position
        let pos: [f32; 3] = positions.next().unwrap_or_default().into();

        // Color
        let color: [f32; 3] = match colors.to_owned() {
            Some(gltf::mesh::util::ReadColors::RgbF32(mut rgb_iter)) => {
                rgb_iter.nth(i).unwrap_or([1.0; 3])
            },
            Some(gltf::mesh::util::ReadColors::RgbaF32(mut rgb_iter)) => {
                rgb_iter.nth(i).unwrap_or([1.0; 4])[0..3].try_into().unwrap()
            },
            _ => {
                [1.0; 3]
            },
        };

        // Normal
        let normal: [f32; 3] = match normals.to_owned() {
            Some(gltf::mesh::util::ReadNormals::Standard(mut normal_iter)) => {
                normal_iter.nth(i).unwrap_or([0.0, 0.0, 1.0])
            },
            _ => {
                [0.0, 0.0, 1.0]
            },
        };

        // UV
        let tex_coord: [f32; 2] = match tex_coords.to_owned() {
            Some(gltf::mesh::util::ReadTexCoords::F32(mut tex_coord_iter)) => {
                let mut coord:[f32; 2] = tex_coord_iter.nth(i).unwrap_or([0.0, 1.0]);
                coord[1] = 1.0 - coord[1];
                coord
            },
            _ => {
                [0.0, 1.0]
            },
        };

        let new_vertex = Vertex {
            position: pos,
            color: color,
            normal: normal,
            uv: tex_coord,
        };

        vertices.push(new_vertex);

    }

    Ok((vertices, indices))
}

fn traverse_directory(dir: &str, ext: Vec<&str>) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if entry.file_type()?.is_dir() {
            paths.extend(traverse_directory(path.to_str().unwrap(), ext.clone())?);
            continue;
        }

        if path.extension().is_some_and(|candidate_ext| ext.contains(&candidate_ext.to_ascii_lowercase().to_str().unwrap())) {
            paths.push(path);
        }
    }

    Ok(paths)
}