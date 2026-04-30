//! Parses the interleaved Lumosity Pet Detective level format.
//!
//! Each level text block is `(2H-1) × (2W-1)` characters:
//! - even text-rows = grid cells at even cols, h-roads at odd cols
//! - odd  text-rows = v-roads at even cols, filler at odd cols
//!
//! Cell glyphs:
//!   '.'   open road
//!   ' '   blocked
//!   'A'..'Z'  pet of index 0..25
//!   'a'..'z'  matching house
//!   '4'   car start

use crate::level::{CellKind, Level};

pub fn parse_levels(text: &str) -> Vec<(Level, u32, f64)> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out = Vec::with_capacity(2600);
    let mut buf: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if l.is_empty() {
            i += 1;
            continue;
        }
        // par_moves line: pure integer (sometimes trailing '.')
        let trimmed = l.trim_end_matches('.');
        if trimmed.bytes().all(|b| b.is_ascii_digit()) && !trimmed.is_empty() {
            let par_moves: u32 = trimmed.parse().expect("par_moves not integer");
            i += 1;
            let par_time: f64 = lines[i].parse().expect("par_time not float");
            i += 1;
            let level = parse_one(&buf);
            out.push((level, par_moves, par_time));
            buf.clear();
        } else {
            buf.push(l);
            i += 1;
        }
    }
    out
}

fn parse_one(grid_lines: &[&str]) -> Level {
    let text_h = grid_lines.len();
    let text_w = grid_lines.iter().map(|l| l.len()).max().unwrap_or(0);
    assert!(text_h % 2 == 1 && text_w % 2 == 1, "text dims must be odd");
    let h = (text_h + 1) / 2;
    let w = (text_w + 1) / 2;

    // Pad rows to text_w with spaces (handles trailing-space stripped lines)
    let row_chars: Vec<Vec<u8>> = grid_lines
        .iter()
        .map(|l| {
            let mut v: Vec<u8> = l.bytes().collect();
            while v.len() < text_w {
                v.push(b' ');
            }
            v
        })
        .collect();

    let mut cells = vec![CellKind::Blocked; w * h];
    let mut h_roads = vec![false; w * h]; // h_roads[y*w + x] = open road from (x,y) to (x+1,y); valid for x < w-1
    let mut v_roads = vec![false; w * h]; // v_roads[y*w + x] = open road from (x,y) to (x,y+1); valid for y < h-1
    let mut start: Option<(u8, u8)> = None;
    let mut pets: [Option<(u8, u8)>; 26] = [None; 26];
    let mut houses: [Option<(u8, u8)>; 26] = [None; 26];

    for grid_y in 0..h {
        let trow = &row_chars[2 * grid_y];
        for grid_x in 0..w {
            let c = trow[2 * grid_x];
            let kind = match c {
                b'.' => CellKind::Road,
                b' ' => CellKind::Blocked,
                b'4' => {
                    start = Some((grid_x as u8, grid_y as u8));
                    CellKind::Road
                }
                b'A'..=b'Z' => {
                    let idx = (c - b'A') as usize;
                    pets[idx] = Some((grid_x as u8, grid_y as u8));
                    CellKind::Pet(idx as u8)
                }
                b'a'..=b'z' => {
                    let idx = (c - b'a') as usize;
                    houses[idx] = Some((grid_x as u8, grid_y as u8));
                    CellKind::House(idx as u8)
                }
                _ => CellKind::Blocked,
            };
            cells[grid_y * w + grid_x] = kind;
            // h-road to the right of this cell, if not last column
            if grid_x + 1 < w {
                let h_indicator = trow[2 * grid_x + 1];
                let open = h_indicator == b'.' || h_indicator == b'-';
                h_roads[grid_y * w + grid_x] = open;
            }
        }
    }

    for vrow_idx in 0..h - 1 {
        let trow = &row_chars[2 * vrow_idx + 1];
        for grid_x in 0..w {
            let v_indicator = trow[2 * grid_x];
            let open = v_indicator == b'.' || v_indicator == b'|';
            v_roads[vrow_idx * w + grid_x] = open;
        }
    }

    // Count pets/houses
    let n_pets = pets.iter().filter(|p| p.is_some()).count();
    let n_houses = houses.iter().filter(|h| h.is_some()).count();
    assert_eq!(n_pets, n_houses, "pet/house count mismatch");
    let pet_count = n_pets as u8;

    Level {
        w: w as u8,
        h: h as u8,
        cells,
        h_roads,
        v_roads,
        start: start.expect("missing '4' start cell"),
        pets,
        houses,
        pet_count,
    }
}

pub fn parse_solutions(text: &str) -> Vec<String> {
    text.split('\n')
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect()
}
