use bevy::{
	prelude::*,
	render::{render_asset::RenderAssetUsages, primitives::Aabb, mesh::MeshAabb},
};

use crate::util::{
	hex::axial_to_xz,
	hex_mesh,
};
use crate::game::{
	placement::{
		cursor::{Cursor, Tool, HoverObjects},
		grid::{Grid, CellPos},
		gizmo_entity::{SetHoverGizmo, RemoveHoverGizmo},
	},
	object::{
		ObjectType,
		ObjectInstance,
		structure::{StructureInstance, SpawnStructure, DespawnStructure, UpdateStructureHeights},
		lift::{LiftInstance, SpawnLift}
	},
	material::Materials,
	surface::{Surface, cell_material},
	events::{UpdateHoverOutline, UpdateHoverGizmo},
};

pub struct GridEntityPlugin;
impl Plugin for GridEntityPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, setup);

		app.add_observer(update_meshes);
		app.add_observer(update_materials);
	}
}

#[derive(Component, Debug, Clone, Copy)]
/// Component for the visible cell of a mountain.
pub struct CellMesh;

pub fn setup(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	materials: Res<Materials>,
	grid: Res<Grid>,
) {
	for pos in grid.heights.keys() {
		let material = match grid.surfaces.get(pos) {
			Some(surface) => cell_material(&materials, &grid.heights, pos, *surface).clone(),
			None => {
				error!("Cell {:?} is missing a surface.", pos);
				materials.error.clone()
			}
		};
		let [x, z] = axial_to_xz(&pos);
		commands.spawn((
			CellMesh,
			CellPos(*pos),
			Mesh3d(meshes.add(hex_mesh::cell_triangle_mesh(&grid.heights, pos, RenderAssetUsages::all()))),
			MeshMaterial3d(material),
			Pickable::default(),
			Transform::from_xyz(x, 0., z),
		))
		.observe(handle_click)
		.observe(handle_hover_start)
		.observe(handle_hover_end);
	}
	for (instance_id, instance) in grid.objects.iter() {
		match instance {
			ObjectInstance::Structure(instance) => commands.trigger(SpawnStructure(*instance_id, *instance)),
			ObjectInstance::Lift(instance) => commands.trigger(SpawnLift(*instance_id, instance.clone())),
		}
	}
}

fn handle_hover_start(
	trigger: Trigger<Pointer<Over>>,
	mut commands: Commands,
	mut cursor: ResMut<Cursor>,
	cells: Query<&CellPos, With<CellMesh>>,
	grid: Res<Grid>,
) {
	let pos = match cells.get(trigger.target()) {Ok(pos) => pos.0, Err(e) => {error!("Mouse hovered over cell, but it's missing a CellPos position: {}", e); return}};
	cursor.hover_cell = Some(pos);
	commands.trigger(SetHoverGizmo(pos));
	/*if matches!(cursor.tool, Tool::None) || matches!(cursor.tool, Tool::Select(_)) {
		match grid.get_cell_objects(pos) {
			Some(objects) => {
				cursor.hover_objects = HoverObjects::Many(objects, 0);
			}
			None => (),
		}
		commands.trigger(UpdateHoverOutline);
	}*/
}

fn handle_hover_end(
	trigger: Trigger<Pointer<Out>>,
	mut commands: Commands,
	mut cursor: ResMut<Cursor>,
	cells: Query<&CellPos, With<CellMesh>>,
) {
	let cell_pos = match cells.get(trigger.target()) {Ok(pos) => pos.0, Err(e) => {error!("Mouse hovered over cell, but it's missing a CellPos position: {}", e); return}};
	let cursor_pos = match cursor.hover_cell {Some(pos) => pos, None => {return}};
	if cursor_pos == cell_pos {
		cursor.hover_cell = None;
		//cursor.hover_objects = HoverObjects::None;
		commands.trigger(UpdateHoverOutline);
	}
	commands.trigger(RemoveHoverGizmo(cell_pos));
}

fn handle_click(
	trigger: Trigger<Pointer<Pressed>>,
	mut commands: Commands,
	mut cursor: ResMut<Cursor>,
	mut grid: ResMut<Grid>,
	cells: Query<&CellPos, With<CellMesh>>,
) {
	let pos = match cells.get(trigger.target()) {Ok(pos) => pos.0, Err(e) => {error!("Mouse clicked unknown cell: {}", e); return}};

	/*if matches!(cursor.tool, Tool::None) || matches!(cursor.tool, Tool::Select(_)) {
		if matches!(trigger.button, PointerButton::Primary) {
			match cursor.hover_object() {
				Some(instance_id) => cursor.tool = Tool::Select(instance_id),
				None => cursor.tool = Tool::None,
			};
			commands.trigger(UpdateHoverOutline);
			commands.trigger(UpdateHoverGizmo);
		}
	} else */if matches!(cursor.tool, Tool::Terrain) {
		let height = match grid.heights.get_mut(&pos) {Some(cell) => cell, None => {error!("Attempted to change height, but grid contains no cell height for pos {:?}.", pos); return}};
		if matches!(trigger.button, PointerButton::Primary) {
			*height += 1;
			commands.trigger(UpdateMeshes);
			commands.trigger(UpdateMaterials);
			commands.trigger(UpdateStructureHeights);
			commands.trigger(UpdateHoverGizmo);

		} else if matches!(trigger.button, PointerButton::Secondary) {
			if *height <= 0 {
				warn!("Can't lower cell {:?} because it's already at height {}.", pos, height);
			} else {
				*height -= 1;
				commands.trigger(UpdateMeshes);
				commands.trigger(UpdateMaterials);
				commands.trigger(UpdateStructureHeights);
				commands.trigger(UpdateHoverGizmo);
			}
		}
	} else if matches!(cursor.tool, Tool::Surface) {
		let surface = match grid.surfaces.get_mut(&pos) {Some(cell) => cell, None => {error!("Attempted to change surface, but grid contains no cell surface for pos {:?}.", pos); return}};
		if matches!(trigger.button, PointerButton::Primary) {
			if matches!(surface, Surface::Normal) {
				*surface = Surface::Piste;
				commands.trigger(UpdateMaterials);
			} else {
				warn!("Can't add piste because the surface is not normal.");
			}
		} else if matches!(trigger.button, PointerButton::Secondary) {
			if matches!(surface, Surface::Piste) {
				*surface = Surface::Normal;
				commands.trigger(UpdateMaterials);
			} else {
				warn!("Can't remove piste because the surface is already not piste.");
			}
		}
		commands.trigger(UpdateMaterials);
	} else if matches!(cursor.tool, Tool::Place) {
		if matches!(trigger.button, PointerButton::Primary) {
			match cursor.selected_object_type {
				Some(object_type) => {
					let object_instance = match object_type {
						ObjectType::Structure(structure_id) => ObjectInstance::Structure(StructureInstance::new(structure_id, pos)),
						ObjectType::Lift => ObjectInstance::Lift(LiftInstance {nodes: Vec::new()}),
					};
					let instance_id = grid.push_object(object_instance.clone());
					match object_instance {
						ObjectInstance::Structure(instance) => commands.trigger(SpawnStructure(instance_id, instance)),
						ObjectInstance::Lift(instance) => commands.trigger(SpawnLift(instance_id, instance)),
					}
				},
				None => warn!("Can't place object at cell {:?} because no object is currently selected.", pos),
			}
		}
	}
}

#[derive(Event, Debug, Clone, Copy)]
pub struct UpdateMeshes;
fn update_meshes(
	_trigger: Trigger<UpdateMeshes>,
	mut meshes: ResMut<Assets<Mesh>>,
	grid: Res<Grid>,
	mut query: Query<(&CellPos, &mut Mesh3d, &mut Aabb), With<CellMesh>>,
) {
	for (cell_pos, mut mesh, mut aabb) in query.iter_mut() {
		let pos = cell_pos.0;
		let new_mesh = hex_mesh::cell_triangle_mesh(&grid.heights, &pos, RenderAssetUsages::all());
		// TODO: Remove this if mesh picking bug is fixed.
		// Currently, the Axis-Aligned Bounding Box is
		// not updated automatically when the mesh changes.
		// https://github.com/bevyengine/bevy/issues/18221#issuecomment-2746183172
		*aabb = new_mesh.compute_aabb().unwrap();
		*mesh = Mesh3d(meshes.add(new_mesh));
	}
}

#[derive(Event, Debug, Clone, Copy)]
pub struct UpdateMaterials;
fn update_materials(
	_trigger: Trigger<UpdateMaterials>,
	materials: Res<Materials>,
	grid: Res<Grid>,
	mut query: Query<(&CellPos, &mut MeshMaterial3d<StandardMaterial>), With<CellMesh>>,
) {
	for (cell_pos, mut material) in query.iter_mut() {
		let pos = cell_pos.0;
		let new_material = match grid.surfaces.get(&pos) {
			Some(surface) => cell_material(&materials, &grid.heights, &pos, *surface).clone(),
			None => {
				error!("Cell {:?} is missing a surface.", pos);
				materials.error.clone()
			}
		};
		if material.0.id() != new_material.id() {
			*material = MeshMaterial3d(new_material.clone());
		}
	}
}

