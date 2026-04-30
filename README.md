# Pet Detective Solver

An optimal-route solver and visualiser for Lumosity's
[Pet Detective](https://www.lumosity.com/en/blog/pet-detective-behind-the-game) —
the routing puzzle where you drive a four-seat car around a grid, picking
up animals and dropping them off at matching houses with as little fuel as
possible.

Two pieces:

- **`rust_solver/`** — Parallel BFS solver (rayon + base-3 joint-state encoding
  + thread-local scratch buffers). Solves all 2,558 shipped levels in roughly
  2.3 s on six cores and writes the optimal routes to
  `web/public/data/precomputed.json`.
- **`web/`** — Svelte 5 + Tailwind app with two modes:
  - **Find a level** — click pets onto an empty grid; the matcher narrows
    down which of the 2,558 shipped levels you're looking at.
  - **Solution library** — browse every level and watch its optimal route
    play out, animated step-by-step.

## Findings

The shipped levels all have solutions. **2,504** of Lumosity's canned
solutions are already optimal; the remaining **54** are sub-optimal — **32**
waste exactly one fuel and **22** waste two.

## Run locally

### Web app
```sh
cd web
npm install
npm run dev
```
Open <http://localhost:5173>.

### Rust solver
```sh
cd rust_solver
cargo run --release
```
Solves every level, prints a coloured summary, and rewrites
`../web/public/data/precomputed.json` — the file the web app fetches at
runtime. Inputs (`levels.txt`, `solutions.txt`) are read from
`web/public/data/`, so no extraction step is needed.

## Deploy

`.github/workflows/pages.yml` builds the web app and publishes to GitHub
Pages on every push to `main`. After the first push, switch
**Settings → Pages → Source** to **GitHub Actions** in the repo.

## How it works

- **Solver.** Forward search over `(current node, trunk-bitmask, animals-left-bitmask)`.
  At each step the car either drops off an animal already in its trunk or, if
  the trunk has fewer than four pets, picks up an unvisited animal. The
  joint state is encoded in base-3 so every `(trunk, animals_left)` pair
  maps to a single integer; the dedup table is then a flat `Vec` indexed by
  `joint_id * n_nodes + current_node`. For 11-pet levels that table is 32 MB
  (doesn't fit in L3), so the inner loop software-prefetches the next
  iteration's slot to overlap DRAM latency. The canned Lumosity solution
  cost is used as a branch-and-bound upper bound to prune provably
  non-optimal expansions.
- **Matcher.** Lumosity randomises species per level at runtime, so the
  matcher only compares unordered (pet-cell, house-cell) pairs across every
  reflection of the grid (identity, H-flip, V-flip, both). Once a unique
  level + variant + letter assignment is found, the editor locks onto it,
  auto-fills implied houses, auto-places the final pet, and redirects to the
  solution player with the right orientation and sprite map.
- **Sprites.** Most assets are vector meshes (Unity's `vector_graphics`
  package); a converter walks the triangulated mesh and emits one path per
  same-coloured run. A few species (hedgehog, turtle, cockatiel, ferret) lay
  out the mesh as alternating "AA-edge" and "body" runs whose averaged
  colour was producing muddy mid-tones — for those, consecutive runs are
  paired and painted with the body colour.

## Acknowledgements

Pet Detective and all in-game artwork are © Lumos&nbsp;Labs,&nbsp;Inc.
The level data and sprites in this repository are derived from the
publicly-distributed Lumosity Android bundle and are reused here for
research and educational purposes only. This project is unaffiliated with
Lumosity. To play the actual game, head to
[lumosity.com](https://www.lumosity.com).
