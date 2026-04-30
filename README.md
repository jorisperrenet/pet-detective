# Pet Detective Solver

Lumosity's [Pet Detective](https://www.lumosity.com/en/blog/pet-detective-behind-the-game)
is a routing puzzle: drive a four-seat car around a grid, pick up pets, drop
them at matching houses, do it in as few moves as possible. Lumosity's blog
muses that for big enough routing problems "computers couldn't even find a
solution in the lifetime of the universe." A parallel BFS in Rust disagrees
— it solves all 2,558 shipped levels in roughly two seconds.

Two pieces:

- **`rust_solver/`** — Parallel BFS solver (rayon + base-3 joint-state
  encoding + thread-local scratch buffers) that writes the optimal route for
  every level to `web/public/data/precomputed.json`.
- **`web/`** — Svelte 5 + Tailwind app with two modes:
  - **Find a level** — click pets onto an empty grid; the matcher narrows
    down which of the 2,558 shipped levels you're looking at.
  - **Solution library** — browse every level and watch its optimal route
    play out, animated step-by-step.

## Findings

Every shipped level is solvable. **2,504** of Lumosity's canned solutions
are already optimal; the remaining **54** are sub-optimal — **32** waste
exactly one fuel and **22** waste two.

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
runtime. Inputs (`levels.txt`, `solutions.txt`) live in `web/public/data/`,
so no extraction step is needed.

## Example solver output

```
==> Pet Detective solver — 6 CPU threads
  levels: ../web/public/data/levels.txt
  sols:   ../web/public/data/solutions.txt
  json:   ../web/public/data/precomputed.json

Parsed: 2558 levels, 2558 canned solutions

═══ Summary ════════════════════════════════════════
  Total: 2558 levels solved in 2.13s
         (1203.4 levels/sec wall-clock)

BFS optimum vs game's par_moves
  ✓  2503  optimum == par   (par is exactly optimal)
  ↓    55  optimum  < par   (par is loose; level beatable in fewer moves)
  !     0  optimum  > par   (shouldn't happen — model error or unsolvable level)

Canned Lumosity solution vs BFS optimum
  ✓  2504  canned == BFS    (the shipped solution is optimal)
  ↑    54  canned  > BFS    (shipped solution is sub-optimal)

Per-group breakdown
  pets   size     n  opt=par  opt<par  opt>par  can=opt  can>opt
     2    3x3   126     126        0        0      126        0
     2    5x3   128     128        0        0      128        0
     3    3x3   128     128        0        0      128        0
     3    6x4   128     128        0        0      128        0
     4    4x3   128     128        0        0      128        0
     4    6x4   128     128        0        0      128        0
     5    4x3   128     128        0        0      128        0
     5    6x4   128     128        0        0      128        0
     6    5x3   128     128        0        0      128        0
     6    6x4   128     128        0        0      128        0
     7    5x3   128     128        0        0      128        0
     7    6x4   128     128        0        0      128        0
     8    6x3   128     128        0        0      128        0
     8    6x4   128     126        2        0      126        2
     9    5x4   128     125        3        0      125        3
     9    6x4   128     126        2        0      126        2
    10    6x4   256     235       21        0      235       21
    11    6x4   256     229       27        0      230       26

Distribution of canned-vs-optimum fuel gap
  +1  fuel    32  ████████████████████████████████████████
  +2  fuel    22  ████████████████████████████

Sample: 54 canned solutions are sub-optimal — top 5 by gap:
  level 1964  pets=9   par=28   BFS=26   canned=28   gap=+2  optimal=`CBcDEGdgIFbfAiHeha`  canned=`DEBGgdIFbfAiHCceha`
  level 2067  pets=10  par=32   BFS=30   canned=32   gap=+2  optimal=`DHBdhIFbGAgiEfeCaJcj`  canned=`HDBdhIFbGAgiEfeCaJcj`
  level 2073  pets=10  par=36   BFS=34   canned=36   gap=+2  optimal=`FGIgCcJEefiDAHjBdhab`  canned=`AHDIhdBCcJaFbEefiGjg`
  level 2146  pets=10  par=29   BFS=27   canned=29   gap=+2  optimal=`IDidCHEceJGAjFfBhabg`  canned=`AIDidCHEceJGjFfBhabg`
  level 2164  pets=10  par=31   BFS=29   canned=31   gap=+2  optimal=`AaIBDFfJbGiCdjEHcghe`  canned=`IBDFfJdCbGiAajEHcghe`
Wrote: ../web/public/data/precomputed.json
```

## Acknowledgements

Pet Detective and all in-game artwork are © Lumos&nbsp;Labs,&nbsp;Inc. The
level data and sprites in this repository are derived from the
publicly-distributed Lumosity Android bundle and are reused here for
research and educational purposes only. This project is unaffiliated with
Lumosity. To play the actual game, head to
[lumosity.com](https://www.lumosity.com).
