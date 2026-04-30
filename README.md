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

Every shipped level is solvable. **2,504** of Lumosity's supplied
solutions are already optimal; the remaining **54** are sub-optimal —
**32** waste exactly one fuel and **22** waste two.

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

```ansi
[1m[96m==>[39m[0m [1mPet Detective solver — 6 CPU threads[0m
[2m  levels:[0m ../web/public/data/levels.txt
[2m  sols:  [0m ../web/public/data/solutions.txt
[2m  json:  [0m ../web/public/data/precomputed.json

[1m[92mParsed:[39m[0m 2558 levels, 2558 supplied solutions

  [1mLumosity's supplied solutions:[0m [2m2558 levels[0m [2m· solved in 2.39 s (1071 levels/sec)[0m
  ─────────────────────────────
    [32moptimal    [39m  [32m████████████████████████████████████████[39m  [1m2504[0m   97.9%
    [33m+1 fuel    [39m  [33m█                                       [39m  [1m  32[0m    1.3%
    [33m+2 fuel    [39m  [33m█                                       [39m  [1m  22[0m    0.9%

  [1mSub-optimal levels by pet count[0m
  ───────────────────────────────
    2–7 pets    [2m·           [0m  [2m 0[0m / [2m1534[0m     0.0%
    8 pets      [33m█[39m             [1m 2[0m /  256     0.8%
    9 pets      [33m███[39m           [1m 5[0m /  256     2.0%
    10 pets     [33m██████████[39m    [1m21[0m /  256     8.2%
    11 pets     [33m████████████[39m  [1m26[0m /  256    10.2%

  [1mWorst gaps (showing 5 of 54)[0m
  ────────────────────────────
    level 1964   9 pets  given 28 → optimal 26  ([1m[33m+2[39m[0m)  optimal=`[32mCBcDEGdgIFbfAiHeha[39m`  given=`[33mDEBGgdIFbfAiHCceha[39m`
    level 2067  10 pets  given 32 → optimal 30  ([1m[33m+2[39m[0m)  optimal=`[32mDHBdhIFbGAgiEfeCaJcj[39m`  given=`[33mHDBdhIFbGAgiEfeCaJcj[39m`
    level 2073  10 pets  given 36 → optimal 34  ([1m[33m+2[39m[0m)  optimal=`[32mFGIgCcJEefiDAHjBdhab[39m`  given=`[33mAHDIhdBCcJaFbEefiGjg[39m`
    level 2146  10 pets  given 29 → optimal 27  ([1m[33m+2[39m[0m)  optimal=`[32mIDidCHEceJGAjFfBhabg[39m`  given=`[33mAIDidCHEceJGjFfBhabg[39m`
    level 2164  10 pets  given 31 → optimal 29  ([1m[33m+2[39m[0m)  optimal=`[32mAaIBDFfJbGiCdjEHcghe[39m`  given=`[33mIBDFfJdCbGiAajEHcghe[39m`
[1m[92mWrote:[39m[0m ../web/public/data/precomputed.json
```

## Acknowledgements

Pet Detective and all in-game artwork are © Lumos&nbsp;Labs,&nbsp;Inc. The
level data and sprites in this repository are derived from the
publicly-distributed Lumosity Android bundle and are reused here for
research and educational purposes only. This project is unaffiliated with
Lumosity. To play the actual game, head to
[lumosity.com](https://www.lumosity.com).
