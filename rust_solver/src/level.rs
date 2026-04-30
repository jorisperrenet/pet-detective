//! Level data and road-graph helpers.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CellKind {
    Blocked,
    Road,
    Pet(u8),   // pet index 0..25
    House(u8), // house index 0..25
}

#[derive(Clone, Debug)]
pub struct Level {
    pub w: u8,
    pub h: u8,
    pub cells: Vec<CellKind>,        // len = w*h, row-major (y*w+x)
    pub h_roads: Vec<bool>,          // open between (x,y) and (x+1,y); valid for x<w-1
    pub v_roads: Vec<bool>,          // open between (x,y) and (x,y+1); valid for y<h-1
    pub start: (u8, u8),
    pub pets: [Option<(u8, u8)>; 26],
    pub houses: [Option<(u8, u8)>; 26],
    pub pet_count: u8,
}

impl Level {
    #[inline]
    pub fn idx(&self, x: u8, y: u8) -> usize {
        y as usize * self.w as usize + x as usize
    }

    pub fn is_passable(&self, x: u8, y: u8) -> bool {
        !matches!(self.cells[self.idx(x, y)], CellKind::Blocked)
    }

    pub fn neighbors(&self, x: u8, y: u8) -> [(Option<(u8, u8)>, bool); 4] {
        let i = self.idx(x, y);
        let w = self.w;
        let h = self.h;
        let up = if y > 0 {
            (Some((x, y - 1)), self.v_roads[(y as usize - 1) * w as usize + x as usize])
        } else {
            (None, false)
        };
        let down = if y + 1 < h {
            (Some((x, y + 1)), self.v_roads[i])
        } else {
            (None, false)
        };
        let left = if x > 0 {
            (Some((x - 1, y)), self.h_roads[i - 1])
        } else {
            (None, false)
        };
        let right = if x + 1 < w {
            (Some((x + 1, y)), self.h_roads[i])
        } else {
            (None, false)
        };
        [up, down, left, right]
    }

    /// All-pairs shortest path between every "important" cell.
    /// Returns `(important_cells, dist_matrix)` where dist[i*N + j] is the road distance from
    /// important_cells[i] to important_cells[j], or u16::MAX if unreachable.
    pub fn all_pairs_distances(&self) -> (Vec<(u8, u8)>, Vec<u16>) {
        let mut important: Vec<(u8, u8)> = Vec::with_capacity(1 + 2 * self.pet_count as usize);
        important.push(self.start);
        for p in self.pets.iter().flatten() {
            important.push(*p);
        }
        for h in self.houses.iter().flatten() {
            important.push(*h);
        }

        let n = important.len();
        let mut dist = vec![u16::MAX; n * n];
        for (i, &src) in important.iter().enumerate() {
            self.bfs_from(src, &important, &mut dist[i * n..(i + 1) * n]);
        }
        (important, dist)
    }

    fn bfs_from(&self, src: (u8, u8), targets: &[(u8, u8)], out: &mut [u16]) {
        let w = self.w as usize;
        let h = self.h as usize;
        let mut visited = vec![false; w * h];
        let mut queue: std::collections::VecDeque<((u8, u8), u16)> =
            std::collections::VecDeque::with_capacity(w * h);
        queue.push_back((src, 0));
        visited[src.1 as usize * w + src.0 as usize] = true;

        // Build a target index for quick lookup
        let mut target_of = vec![u8::MAX; w * h];
        for (i, &t) in targets.iter().enumerate() {
            target_of[t.1 as usize * w + t.0 as usize] = i as u8;
        }

        while let Some(((x, y), d)) = queue.pop_front() {
            let ti = target_of[y as usize * w + x as usize];
            if ti != u8::MAX {
                out[ti as usize] = d;
            }
            for (nb, open) in self.neighbors(x, y) {
                if !open {
                    continue;
                }
                if let Some((nx, ny)) = nb {
                    let ni = ny as usize * w + nx as usize;
                    if !visited[ni] && self.is_passable(nx, ny) {
                        visited[ni] = true;
                        queue.push_back(((nx, ny), d + 1));
                    }
                }
            }
        }
    }
}
