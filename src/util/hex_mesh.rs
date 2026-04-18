use std::collections::HashMap;
use bevy::{
	prelude::*,
	render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::{PrimitiveTopology, ShaderType}}
};
use hexx::{Hex, MeshInfo, HexLayout, ColumnMeshBuilder, HeightMapMeshBuilder};

use crate::util::hex::{axial_to_xz, HexCorner, HexEdge, corner_height};

/// Converts hexx MeshInfo into bevy Mesh.
/// From hexx docs example: https://docs.rs/hexx/latest/hexx/index.html#usage-in-bevy
fn hexagonal_mesh(mesh_info: MeshInfo, asset_usage: RenderAssetUsages) -> Mesh {
	Mesh::new(PrimitiveTopology::TriangleList, asset_usage)
	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, mesh_info.vertices)
	.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, mesh_info.normals)
	.with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, mesh_info.uvs)
	.with_inserted_indices(Indices::U16(mesh_info.indices))
}

#[allow(dead_code)] // TODO: Remove this function if still unused.
pub fn cell_column_mesh(height: f32, asset_usage: RenderAssetUsages) -> Mesh {
	hexagonal_mesh(ColumnMeshBuilder::new(
		&HexLayout::flat(),
		height,
	).build(), asset_usage)
}

#[allow(dead_code)] // TODO: Remove this function if still unused.
pub fn mountain_column_mesh(heights: &HashMap<Hex, u16>, asset_usage: RenderAssetUsages) -> Mesh {
	hexagonal_mesh(HeightMapMeshBuilder::new(
		&HexLayout::flat(),
		&heights.iter().map(|(pos, height)| (*pos, *height as f32)).collect::<HashMap<Hex, f32>>(),
	).build(), asset_usage)
}

fn cell_sharp(heights: &HashMap<Hex, u16>, pos: &Hex, world_transform: bool) -> Vec<Vec3> {
	let center_y = *heights.get(pos).unwrap() as f32;
	let [center_x, center_z] = if world_transform {axial_to_xz(pos)} else {[0., 0.]};
	let center_vertex = Vec3::new(center_x, center_y, center_z);
	let corner_vertices: Vec<Vec3> = HexCorner::get_array().iter().map(|corner| {
		let [x, z] = corner.to_xz();
		Vec3::new(center_x + x, corner_height(heights, pos, *corner), center_z + z)
	}).collect();
	let mut vertices: Vec<Vec3> = Vec::new();
	for i in 0..6 { // TODO: Make this more efficient (it runs a lot).
		vertices.push(center_vertex);
		vertices.push(corner_vertices[(i + 1) % 6]);
		vertices.push(corner_vertices[i]);
	}
	vertices
}

#[allow(dead_code)] // TODO: Remove this function if still unused.
pub fn cell_sharp_mesh(heights: &HashMap<Hex, u16>, pos: &Hex, asset_usage: RenderAssetUsages) -> Mesh {
	Mesh::new(PrimitiveTopology::TriangleList, asset_usage)
	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, cell_sharp(heights, pos, false))
	.with_inserted_indices(Indices::U16((0..18).collect()))
	.with_computed_smooth_normals()
}

#[allow(dead_code)] // TODO: Remove this function if still unused.
pub fn mountain_sharp_mesh(heights: &HashMap<Hex, u16>, asset_usage: RenderAssetUsages) -> Mesh {
	let mut vertices: Vec<Vec3> = Vec::new();
	for (pos, _cell) in heights.iter() {
		vertices.extend(cell_sharp(heights, pos, true));
	}

	Mesh::new(PrimitiveTopology::TriangleList, asset_usage)
	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
	.with_inserted_indices(Indices::U32((0..heights.keys().count() as u32 * 18).collect()))
	.with_computed_smooth_normals()
}

fn cell_triangle(heights: &HashMap<Hex, u16>, pos: &Hex, world_transform: bool) -> Vec<Vec3> {
	let center_y = *heights.get(pos).unwrap() as f32;
	let [center_x, center_z] = if world_transform {axial_to_xz(pos)} else {[0., 0.]};
	let center_vertex = Vec3::new(center_x, center_y, center_z);
	let corner_vertices: Vec<Vec3> = (0..12).filter_map(|i| {
		if i % 2 == 0 {
			let neighbor_y = match heights.get(&pos.all_neighbors()[i / 2]) {
				Some(neighbor_y) => *neighbor_y as f32,
				None => return None
			};
			let [x, z] = axial_to_xz(&Hex::ZERO.all_neighbors()[i / 2]);
			Some(Vec3::new(center_x + (x / 2.), (center_y + neighbor_y) / 2., center_z + (z / 2.)))
		} else {
			let mut neighbors_heights = vec![center_y];
			if let Some(height) = heights.get(&pos.all_neighbors()[((i / 2))]) {
				neighbors_heights.push(*height as f32);
			}
			if let Some(height) = heights.get(&pos.all_neighbors()[((i / 2) + 1) % 6]) {
				neighbors_heights.push(*height as f32);
			}
			if neighbors_heights.len() <= 1 {return None;}
			let point = Hex::ZERO.all_vertices()[((i / 2) + 1) % 6].direction.world_unit_vector(&HexLayout::flat());
			Some(Vec3::new(center_x + point.x, neighbors_heights.iter().sum::<f32>() / (neighbors_heights.len() as f32), center_z + point.y))
		}
	}).collect();
	let mut vertices: Vec<Vec3> = Vec::new();
	for i in 0..corner_vertices.len() { // TODO: Make this more efficient (it runs a lot).
		vertices.push(center_vertex);
		vertices.push(corner_vertices[(i + 1) % corner_vertices.len()]);
		vertices.push(corner_vertices[i]);
	}
	vertices
}

#[allow(dead_code)] // TODO: Remove this function if still unused.
pub fn cell_triangle_mesh(heights: &HashMap<Hex, u16>, pos: &Hex, asset_usage: RenderAssetUsages) -> Mesh {
	let vertices = cell_triangle(heights, pos, false);
	let vertices_len = vertices.len() as u32;
	Mesh::new(PrimitiveTopology::TriangleList, asset_usage)
	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
	.with_inserted_indices(Indices::U32((0..vertices_len).collect()))
	.with_computed_smooth_normals()
}

fn cell_triangle_smooth(heights: &HashMap<Hex, u16>, pos: &Hex, world_transform: bool) -> Vec<Vec3> {
	let center_y = *heights.get(pos).unwrap() as f32;
	let [center_x, center_z] = if world_transform {axial_to_xz(pos)} else {[0., 0.]};
	let center_vertex = Vec3::new(center_x, center_y, center_z);
	let corner_vertices: Vec<Vec3> = (0..12).map(|i| {
		if i % 2 == 0 {
			let neighbor_y = match heights.get(&pos.all_neighbors()[i / 2]) {
				Some(neighbor_y) => *neighbor_y as f32,
				None => center_y
			};
			let [x, z] = axial_to_xz(&Hex::ZERO.all_neighbors()[i / 2]);
			Vec3::new(center_x + (x / 2.), (center_y + neighbor_y) / 2., center_z + (z / 2.))
		} else {
			let mut neighbors_heights = vec![center_y];
			if let Some(height) = heights.get(&pos.all_neighbors()[((i / 2))]) {
				neighbors_heights.push(*height as f32);
			}
			if let Some(height) = heights.get(&pos.all_neighbors()[((i / 2) + 1) % 6]) {
				neighbors_heights.push(*height as f32);
			}
			let point = Hex::ZERO.all_vertices()[((i / 2) + 1) % 6].direction.world_unit_vector(&HexLayout::flat());
			Vec3::new(center_x + point.x, neighbors_heights.iter().sum::<f32>() / (neighbors_heights.len() as f32), center_z + point.y)
		}
	}).collect();
	let mut vertices: Vec<Vec3> = vec![center_vertex];
	for i in 0..corner_vertices.len() { // TODO: Make this more efficient (it runs a lot).
		vertices.push(corner_vertices[i]);
	}
	vertices
}

#[allow(dead_code)] // TODO: Remove this function if still unused.
pub fn cell_triangle_smooth_mesh(heights: &HashMap<Hex, u16>, pos: &Hex, asset_usage: RenderAssetUsages) -> Mesh {
	Mesh::new(PrimitiveTopology::TriangleList, asset_usage)
	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, cell_triangle_smooth(heights, pos, false))
	.with_inserted_indices(Indices::U32(vec![0, 2, 1, 0, 3, 2, 0, 4, 3, 0, 5, 4, 0, 6, 5, 0, 7, 6, 0, 8, 7, 0, 9, 8, 0, 10, 9, 0, 11, 10, 0, 12, 11, 0, 1, 12]))
	.with_computed_smooth_normals()
}

fn cell_fuzzy(heights: &HashMap<Hex, u16>, pos: &Hex, world_transform: bool) -> Vec<Vec3> {
	let [center_x, center_z] = if world_transform {axial_to_xz(pos)} else {[0., 0.]};
	let height = heights.get(pos).unwrap();
	vec![Vec3::new(center_x, *height as f32, center_z)]
	.into_iter().chain(HexCorner::get_array().iter().map(|corner| {
			let [corner_x, corner_z] = corner.to_xz();
			Vec3::new(center_x + corner_x, corner_height(heights, pos, *corner), center_z + corner_z)
		})
	).collect()
}

#[allow(dead_code)] // TODO: Remove this function if still unused.
pub fn cell_fuzzy_mesh(heights: &HashMap<Hex, u16>, pos: &Hex, asset_usage: RenderAssetUsages) -> Mesh {
	Mesh::new(PrimitiveTopology::TriangleList, asset_usage)
	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, cell_fuzzy(heights, pos, false))
	.with_inserted_indices(Indices::U16(vec![0, 2, 1, 0, 3, 2, 0, 4, 3, 0, 5, 4, 0, 6, 5, 0, 1, 6]))
	.with_computed_smooth_normals()
}

#[allow(dead_code)] // TODO: Remove this function if still unused.
/// Mesh consisting of soft hexagons.
pub fn mountain_fuzzy_mesh(heights: &HashMap<Hex, u16>, asset_usage: RenderAssetUsages) -> Mesh {
	let mut vertices: Vec<Vec3> = Vec::new();
	let mut vertices_count: u16 = 0;
	let mut triangles: Vec<u16> = Vec::new();
	for pos in heights.keys().into_iter() {
		let center_index = vertices_count;
		vertices.extend(cell_fuzzy(heights, pos, true));
		vertices_count += 7;
		for i in 0..6 {
			triangles.push(center_index);
			triangles.push(center_index + 1 + (1 + i) % 6);
			triangles.push(center_index + 1 + i);
		}
	}

	Mesh::new(PrimitiveTopology::TriangleList, asset_usage)
	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
	.with_inserted_indices(Indices::U16(triangles))
	.with_computed_smooth_normals()
}

#[allow(dead_code)] // TODO: Remove this function if still unused.
pub fn mountain_smooth_mesh(heights: &HashMap<Hex, u16>, asset_usage: RenderAssetUsages) -> Mesh { // BUG: Some vertices are displaced because later cells interpret the wrong vertices as existing.
	// Help function that finds the index of the corner vertex with a specific position, or creates one if it doesn't exist.
	let corner_vertex = |
		center_pos: Hex,
		center_cords: Vec3,
		corner: HexCorner,
		heights: &HashMap<Hex, u16>,
		vertices: &mut Vec<Vec3>,
		vertices_count: &mut u16,
		corner_vertex_indices: &mut [HashMap<Hex, u16>; 2]
	| -> u16 {
		let map_index = if corner.is_even() {0} else {1};
		let [edge_1, _edge_2] = corner.neighbor_edges();
		let corner_pos = center_pos + edge_1.direction();
		match corner_vertex_indices[map_index].get(&corner_pos) {
			Some(corner_index) => *corner_index, // This edge vertex already exists.
			None => { // This edge vertex doesn't exist yet, needs to be calculated.
				let [corner_x, corner_z] = corner.to_xz();
				let corner_cords = Vec3::new(center_cords.x + corner_x, corner_height(heights, &center_pos, corner), center_cords.z - corner_z);

				let corner_index = *vertices_count;
				vertices.push(corner_cords);
				*vertices_count += 1;
				corner_vertex_indices[map_index].insert(corner_pos, corner_index);
				corner_index
			}
		}
	};

	let mut vertices: Vec<Vec3> = Vec::new();
	let mut vertices_count: u16 = 0;
	// The edge vertices can be mapped on to two hexagonal grids, one for even directions and one for odd.
	let mut corner_vertex_indices: [HashMap<Hex, u16>; 2] = [HashMap::new(), HashMap::new()];
	let mut triangles: Vec<u16> = Vec::new();
	for (pos, height) in heights.iter() {
		let y = *height as f32;
		let [x, z] = axial_to_xz(&pos);
		let vertex_index = vertices_count;
		let cords = Vec3::new(x, y, z);
		vertices.push(cords);
		vertices_count += 1;
		for (corner_i, corner) in HexCorner::get_array().iter().enumerate() {
			let corner_1_index = corner_vertex(*pos, cords, *corner, heights, &mut vertices, &mut vertices_count, &mut corner_vertex_indices);
			let corner_2_index = corner_vertex(*pos, cords, HexCorner::get_array()[(corner_i + 1) % 6], heights, &mut vertices, &mut vertices_count, &mut corner_vertex_indices);
			triangles.push(vertex_index);
			triangles.push(corner_2_index);
			triangles.push(corner_1_index);
		}
	}

	Mesh::new(PrimitiveTopology::TriangleList, asset_usage)
	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
	.with_inserted_indices(Indices::U16(triangles))
	.with_computed_smooth_normals()
}
