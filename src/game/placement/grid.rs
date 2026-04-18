use std::collections::{HashMap, BTreeMap};
use bevy::prelude::*;
use hexx::Hex;
use noise::{Perlin, NoiseFn};
use rand::{prelude::*, random_bool};

use crate::util::hex::{axial_to_xz, offset_to_axial};
use crate::game::{
	surface::Surface,
	object::{ObjectInstance, structure::StructureInstance},
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct WorldGenSettings {
	/// 1-50
	pub peak_height: f64,
	/// 1-50
	pub peak_width: f64,
	/// 1-50
	pub slope_height: f64, // TODO: Add more parameters.
	/// Same scale as `peak_height`.
	/// 0-5
	pub bump_height: f64,
	/// Higher value mean narrower bumps.
	/// 1-20
	pub bump_width: f64,
}
impl Default for WorldGenSettings {
	fn default() -> Self {Self {
		peak_height: 10., peak_width: 30., slope_height: 40., bump_height: 1., bump_width: 10.
	}}
}

#[derive(Resource, Debug, Clone)]
pub struct Grid {
	pub heights: HashMap<Hex, u16>,
	pub surfaces: HashMap<Hex, Surface>,
	/// All placed objects indexed by their instance id.
	pub objects: BTreeMap<u32, ObjectInstance>,
	#[allow(unused_variables)] // TODO: Remove if still unused.
	pub width: u16,
	#[allow(unused_variables)] // TODO: Remove if still unused.
	pub length: u16,
	#[allow(unused_variables)] // TODO: Remove if still unused.
	pub settings: WorldGenSettings
}
impl Grid {
	pub const WATER_HEIGHT: f64 = -3.;

	/// Generates a new grid with set width and length.
	/// It is recommended to use an odd number for width to avoid sharp corners.
	pub fn new(width: u16, length: u16, settings: WorldGenSettings) -> Self {
		let mut grid = Grid {
			heights: HashMap::new(),
			surfaces: HashMap::new(),
			objects: BTreeMap::new(),
			settings: settings, length: length, width: width,
		};

		let mut rng = rand::rng();
		let perlin_main = Perlin::new(rng.random());
		let perlin_bump = Perlin::new(rng.random());
		let max_z = length as f64 * f64::sqrt(3.); // TODO: Use fancy new std::f32::consts::SQRT_3 when available. https://github.com/rust-lang/rust/issues/103883
		for col in 0..width as i32 {
			for row in 0..length as i32 + (col % 2) { // Adds one extra row every other column (avoids sharp corners.
				let pos_axial = offset_to_axial(col, row);

				let [x, z] = axial_to_xz(&pos_axial);
				let height = perlin_main.get([x as f64 / settings.peak_width, z as f64 / settings.peak_width]) * settings.peak_height + (z as f64 / max_z) * settings.slope_height
				+ (perlin_bump.get([settings.bump_width * x as f64 / settings.peak_width, settings.bump_width * z as f64 / settings.peak_width]) * settings.bump_height);
				grid.heights.insert(pos_axial, height as u16);

				// Add water if height is low enough.
				let surface = if height < Grid::WATER_HEIGHT {Surface::Water} else {Surface::Normal};
				grid.surfaces.insert(pos_axial, surface);

				// Add tree if height + randomness is high enough.
				if surface != Surface::Water
					&& random_bool((0.5 - height / (2. * (settings.peak_height + settings.slope_height))).clamp(0., 1.)
				) {
					grid.push_object(ObjectInstance::Structure(StructureInstance::new(1, pos_axial)));
				}
			}
		}
		grid
	}

	/// Adds the specified ObjectInstance to the objects map.
	/// Returns the resulting instance id in the map.
	pub fn push_object(&mut self, object: ObjectInstance) -> u32 {
		let instance_id = match self.objects.last_key_value() {Some((id, _object)) => *id + 1, None => 0};
		match self.objects.insert(instance_id, object) {
			Some(old_object) => error!("When pushing new object instance, the resulting instance id {} was already in use by {:?}, which was now replaced in the grid.", instance_id, old_object),
			None => (),
		}
		instance_id
	}

	/// Finds all object instances that overlap with the specified cell and returns their instance IDs.
	pub fn get_cell_objects(&self, pos: Hex) -> Option<Vec<u32>> {
		let mut results: Vec<u32> = Vec::new();
		for (instance_id, object) in self.objects.iter() {
			match object {
				ObjectInstance::Structure(structure) => {
					if structure.pos == pos {
						results.push(*instance_id);
					}
				},
				ObjectInstance::Lift(_lift) => (),
			}
		}
		if results.iter().count() <= 0 {
			return None
		}
		Some(results)
	}
}

#[derive(Component, Debug, PartialEq, Eq, Clone, Copy, Hash)]
/// Component for the axial position that corresponds to this mesh/structure.
pub struct CellPos(pub Hex);
