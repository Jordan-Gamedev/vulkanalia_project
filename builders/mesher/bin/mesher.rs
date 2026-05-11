use meshopt::{quantize_half, quantize_snorm, quantize_unorm};
use std::ffi::OsStr;
use std::fs;
use std::io::Result;
use std::path::PathBuf;
use std::str::FromStr;

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

fn main() -> Result<()> {

    let mut cl_args = std::env::args();

    if cl_args.by_ref().len() != 3 {
        panic!("Mesher Usage: .\\mesher <in_directory> <out_directory>");
    }

    let source_root_dir = cl_args.nth(1).unwrap();
    let dest_root_dir = cl_args.next().unwrap();

    // Get all 
    let paths = traverse_directory(&source_root_dir.as_str(), "glb")?;

    // Remove previous directory
    match fs::remove_dir_all(&dest_root_dir) {
        Ok(_) => {},
        Err(e) => eprintln!("{e}"),
    };

    // Go over every glb that was gathered
    for path in paths {
        println!("Found model: {}", path.to_str().unwrap());

        // Import glb
        match gltf::import(&path) {
            // Compress valid glb
            Ok((glb, buffers, _)) => {
                let (verts, inds) = compress_glb(glb, buffers)?;
                write_mesh_to_file(&source_root_dir, &dest_root_dir, path.parent().unwrap().as_os_str(), &path.file_stem().unwrap(), verts, inds)?;
            }
            // Invalid glb
            Err(e) => {
                eprintln!("Failed to load the model! {e}");
                continue;
            },
        };
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

    // Compress glb
    println!("Compressing file. . .");

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
    println!("Exporting file. . .");

    let vertices = vertices.unwrap();
    let indices = indices.unwrap();

    let dest_path_dir = {
        let source_path_string = String::from_str(source_path_dir.to_str().unwrap()).unwrap();
        source_path_string.replace(source_root_dir.as_str(), dest_root_dir.as_str())
    };

    // Create a new directory
    match fs::create_dir_all(&dest_path_dir) {
        Ok(_) => {},
        Err(e) => eprintln!("{e}"),
    };

    // Write vertex contents to file
    let file_name = format!("{}{}{}.vertbuff", dest_path_dir.as_str(), std::path::MAIN_SEPARATOR, source_path_file_name.to_str().unwrap());
    match fs::write(file_name, vertices.as_slice()) {
        Ok(_) => println!("Vertex Export Successful!"),
        Err(e) => eprintln!("{e}"),
    }

    // Write index contents to file
    let file_name = format!("{}{}{}.indbuff", dest_path_dir.as_str(), std::path::MAIN_SEPARATOR, source_path_file_name.to_str().unwrap());
    match fs::write(file_name, indices.as_slice()) {
        Ok(_) => println!("Index Export Successful!"),
        Err(e) => eprintln!("{e}"),
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

fn traverse_directory(dir: &str, ext: &str) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = vec![];

    // Get all files in current directory
    paths.extend(fs::read_dir(dir.to_string()).expect(format!("Failed to open directory: {dir}").as_str())
        .into_iter()
        .filter(|f| {
            
            // Only get files that end with glb
            let file = f.as_ref().unwrap();
            file.file_type().unwrap().is_file() && file.path().extension().is_some_and(|ext| ext.eq_ignore_ascii_case(ext))

        })
        .map(|f| f.unwrap().path())
        .collect::<Vec<PathBuf>>()
    );

    // Afterwards go into all the subdirectories in current directory
    paths.extend(fs::read_dir(dir.to_string()).expect(format!("Failed to open directory: {dir}").as_str())
        .into_iter()
        .filter(|f| f.as_ref().unwrap().file_type().unwrap().is_dir())
        .map(|f| traverse_directory(f.unwrap().path().to_str().unwrap(), ext).unwrap())
        .collect::<Vec<Vec<PathBuf>>>()
        .into_iter()
        .flatten()
        .collect::<Vec<PathBuf>>()
    );

    Ok(paths)
}