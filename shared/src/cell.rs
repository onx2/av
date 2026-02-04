//! Cell grid encoding/decoding helpers.
//!
//! This module centralizes the "cell id" scheme so it's easy to reason about world sizing
//! and to test correctness.
//!
//! # Model
//! - `CellId` is a compact `u16` identifying a cell in a square grid.
//! - The grid is `GRID_SIDE x GRID_SIDE` cells.
//! - World units are meters.
//! - World-to-cell mapping uses `WORLD_OFFSET` to shift negative coordinates into the
//!   `[0, GRID_SIDE)` range.
//!
//! # Encoding
//! We compute grid coords:
//! - `gx = floor((x + WORLD_OFFSET) / CELL_SIZE)`
//! - `gz = floor((z + WORLD_OFFSET) / CELL_SIZE)`
//! Then clamp each into `[0, GRID_SIDE-1]` and linearize in X-major order:
//! - `id = gx * GRID_SIDE + gz`
//!
//! # AOI
//! `get_aoi_block` returns a 3x3 block around a center cell, using wrapping arithmetic
//! to match the prior behavior (fast, branchless). If you need clamped-at-edges behavior,
//! add a separate helper.

use crate::{
    CellId,
    constants::{CELL_SIZE, GRID_SIDE, INV_CELL_SIZE, WORLD_OFFSET},
};

/// Returns the world span per axis in meters for the current grid configuration.
#[inline]
pub fn world_span_m() -> f32 {
    (GRID_SIDE as f32) * CELL_SIZE
}

/// Returns the maximum representable cell index (GRID_SIDE - 1).
#[inline]
pub fn max_cell_coord() -> u16 {
    GRID_SIDE - 1
}

/// Encodes world position (x, z) into a compact [`CellId`].
///
/// This is the canonical encoding used by server/client to compute AOI membership.
#[inline]
pub fn encode_cell_id(x: f32, z: f32) -> CellId {
    // Convert to cell coordinates in floating-point space (cell units).
    let gx_f = ((x + WORLD_OFFSET) * INV_CELL_SIZE).floor();
    let gz_f = ((z + WORLD_OFFSET) * INV_CELL_SIZE).floor();

    // Clamp to representable grid.
    let side_f = GRID_SIDE as f32;
    let gx = gx_f.clamp(0.0, side_f - 1.0) as u16;
    let gz = gz_f.clamp(0.0, side_f - 1.0) as u16;

    gx * GRID_SIDE + gz
}

/// Decodes a [`CellId`] into `(grid_x, grid_z)` coordinates in `[0, GRID_SIDE)`.
#[inline]
pub fn decode_cell_coords(id: CellId) -> (u16, u16) {
    let gx = id / GRID_SIDE;
    let gz = id % GRID_SIDE;
    (gx, gz)
}

/// Decodes a [`CellId`] into the world position `(x, z)` of the cell's minimum corner.
///
/// This is the inverse mapping of [`encode_cell_id`] up to floor/clamp behavior.
#[inline]
pub fn decode_cell_min_corner(id: CellId) -> (f32, f32) {
    let (gx, gz) = decode_cell_coords(id);
    (
        (gx as f32) * CELL_SIZE - WORLD_OFFSET,
        (gz as f32) * CELL_SIZE - WORLD_OFFSET,
    )
}

/// Returns the 9 cell IDs forming a 3x3 Area of Interest (AOI) block around `cell_id`.
///
/// Layout (top-down view, +Z = North):
///
/// [0] North-West | [1] North     | [2] North-East
/// ------------------------------------------------
/// [3] West       | [4] Center    | [5] East
/// ------------------------------------------------
/// [6] South-West | [7] South     | [8] South-East
///
/// This uses wrapping arithmetic to match the prior behavior.
#[inline]
pub fn get_aoi_block(cell_id: CellId) -> [CellId; 9] {
    let (x, z) = decode_cell_coords(cell_id);

    // Neighbor coordinates (wrap to valid range and avoid branching)
    let x_west = x.wrapping_sub(1);
    let x_east = x.wrapping_add(1);
    let z_north = z.wrapping_add(1);
    let z_south = z.wrapping_sub(1);

    let pack = |gx: u16, gz: u16| -> CellId { gx.wrapping_mul(GRID_SIDE).wrapping_add(gz) };

    [
        pack(x_west, z_north), // NW
        pack(x, z_north),      // N
        pack(x_east, z_north), // NE
        pack(x_west, z),       // W
        cell_id,               // Center
        pack(x_east, z),       // E
        pack(x_west, z_south), // SW
        pack(x, z_south),      // S
        pack(x_east, z_south), // SE
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_side_is_square_of_cell_id_space() {
        // For a u16 CellId and a square grid, GRID_SIDE must be 256 so GRID_SIDE^2 = 65536.
        assert_eq!(GRID_SIDE, 256);
        assert_eq!(
            (GRID_SIDE as u32) * (GRID_SIDE as u32),
            (u16::MAX as u32) + 1
        );
    }

    #[test]
    fn encode_decode_round_trip_for_cell_coords() {
        // For each cell coordinate pair, packing then unpacking returns the same coords.
        // This is the strongest invariant for the linearization scheme.
        let samples: &[(u16, u16)] = &[
            (0, 0),
            (0, max_cell_coord()),
            (max_cell_coord(), 0),
            (max_cell_coord(), max_cell_coord()),
            (1, 2),
            (42, 133),
            (128, 128),
            (255, 17),
        ];

        for &(gx, gz) in samples {
            let id = gx * GRID_SIDE + gz;
            let (dx, dz) = decode_cell_coords(id);
            assert_eq!((dx, dz), (gx, gz));
        }
    }

    #[test]
    fn decode_cell_min_corner_matches_cell_size_grid() {
        // The min corner of adjacent cells differs by exactly CELL_SIZE in the expected axis.
        let a = 10u16 * GRID_SIDE + 20u16;
        let b_east = 11u16 * GRID_SIDE + 20u16;
        let b_north = 10u16 * GRID_SIDE + 21u16;

        let (ax, az) = decode_cell_min_corner(a);
        let (bx, bz) = decode_cell_min_corner(b_east);
        let (nx, nz) = decode_cell_min_corner(b_north);

        assert!((bx - ax - CELL_SIZE).abs() < 1.0e-6);
        assert!((bz - az).abs() < 1.0e-6);

        assert!((nx - ax).abs() < 1.0e-6);
        assert!((nz - az - CELL_SIZE).abs() < 1.0e-6);
    }

    #[test]
    fn encode_clamps_to_grid_bounds() {
        // Extremely out-of-range positions should clamp to the grid edges.
        let huge = 1.0e12;

        let id_min = encode_cell_id(-huge, -huge);
        let (gx_min, gz_min) = decode_cell_coords(id_min);
        assert_eq!((gx_min, gz_min), (0, 0));

        let id_max = encode_cell_id(huge, huge);
        let (gx_max, gz_max) = decode_cell_coords(id_max);
        assert_eq!((gx_max, gz_max), (max_cell_coord(), max_cell_coord()));
    }

    #[test]
    fn aoi_block_center_is_input() {
        let center = 123u16 * GRID_SIDE + 45u16;
        let block = get_aoi_block(center);
        assert_eq!(block[4], center);
    }

    #[test]
    fn aoi_block_wraps_at_edges() {
        // At the (0,0) corner, west/south wrap.
        let center = 0u16 * GRID_SIDE + 0u16;
        let block = get_aoi_block(center);

        // West wraps to 255, South wraps to 255.
        let expected_w = 255u16 * GRID_SIDE + 0u16;
        let expected_s = 0u16 * GRID_SIDE + 255u16;
        let expected_sw = 255u16 * GRID_SIDE + 255u16;

        assert_eq!(block[3], expected_w); // W
        assert_eq!(block[7], expected_s); // S
        assert_eq!(block[6], expected_sw); // SW
    }

    #[test]
    fn world_span_and_offset_are_consistent() {
        // WORLD_OFFSET should be half the world span for centered mapping.
        let span = world_span_m();
        assert!((WORLD_OFFSET - (span * 0.5)).abs() < 1.0e-6);
    }
}
