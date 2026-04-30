//! Solve every shipped Pet Detective level, print a summary, and write the
//! per-level optimum out to `web/public/data/precomputed.json` for the
//! browser app to consume.
//!
//! Output JSON: one entry per level, fields used by `web/src/lib/precomputed.ts`:
//!   { "i", "p", "w", "h", "par", "t", "fuel", "hist", "canned", "cf" }

use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use rayon::prelude::*;

use pet_detective_solver::parser;
use pet_detective_solver::solver;

#[derive(Clone, Debug)]
struct LevelOutcome {
    idx: usize,
    pet_count: u8,
    grid_w: u8,
    grid_h: u8,
    par_moves: u32,
    par_time: f64,
    bfs_fuel: Option<u32>,
    bfs_history: String,
    canned_solution: String,
    canned_fuel: Option<u32>,
}

fn main() {
    let levels_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../web/public/data/levels.txt".to_string());
    let solutions_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "../web/public/data/solutions.txt".to_string());
    let json_out: PathBuf = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "../web/public/data/precomputed.json".to_string())
        .into();

    println!(
        "{} {}",
        "==>".bright_cyan().bold(),
        format!("Pet Detective solver — {} CPU threads", rayon::current_num_threads()).bold()
    );
    println!("{} {}", "  levels:".dimmed(), levels_path);
    println!("{} {}", "  sols:  ".dimmed(), solutions_path);
    println!("{} {}", "  json:  ".dimmed(), json_out.display());
    println!();

    let levels_text = std::fs::read_to_string(&levels_path).expect("read levels file");
    let solutions_text = std::fs::read_to_string(&solutions_path).expect("read solutions file");

    let parsed = parser::parse_levels(&levels_text);
    let solutions = parser::parse_solutions(&solutions_text);
    println!(
        "{} {} levels, {} canned solutions",
        "Parsed:".bright_green().bold(),
        parsed.len(),
        solutions.len()
    );
    if parsed.len() != solutions.len() {
        eprintln!(
            "{}: level count {} != non-blank solution count {} — alignment will be by index, mismatched ones skipped",
            "warning".yellow().bold(),
            parsed.len(),
            solutions.len()
        );
    }

    let progress = ProgressBar::new(parsed.len() as u64);
    progress.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>5}/{len:5} ({eta}) {msg}",
        )
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏ "),
    );

    let started = Instant::now();
    let outcomes: Vec<LevelOutcome> = parsed
        .par_iter()
        .progress_with(progress.clone())
        .enumerate()
        .map(|(idx, (level, par_moves, par_time))| {
            let canned = solutions.get(idx).cloned().unwrap_or_default();
            let canned_fuel = if canned.is_empty() {
                None
            } else {
                solver::replay(level, &canned)
            };
            // Use canned solution's fuel (if valid) as a branch-and-bound upper
            // bound. Falls back to no-pruning when no valid canned exists.
            let ub = canned_fuel.unwrap_or(u32::MAX);
            let res = solver::solve_with_ub(level, ub);
            LevelOutcome {
                idx,
                pet_count: level.pet_count,
                grid_w: level.w,
                grid_h: level.h,
                par_moves: *par_moves,
                par_time: *par_time,
                bfs_fuel: res.as_ref().map(|r| r.fuel),
                bfs_history: res.as_ref().map(|r| r.history.clone()).unwrap_or_default(),
                canned_solution: canned,
                canned_fuel,
            }
        })
        .collect();
    progress.finish_and_clear();
    let elapsed = started.elapsed();

    print_summary(&outcomes, elapsed);
    write_json(&outcomes, &json_out);
}

/// Serialise outcomes to the schema `web/src/lib/precomputed.ts` expects.
fn write_json(outcomes: &[LevelOutcome], out_path: &std::path::Path) {
    let mut s = String::with_capacity(outcomes.len() * 160);
    s.push('[');
    s.push('\n');
    for (i, o) in outcomes.iter().enumerate() {
        if i > 0 {
            s.push(',');
            s.push('\n');
        }
        write!(
            s,
            "{{\"i\":{},\"p\":{},\"w\":{},\"h\":{},\"par\":{},\"t\":{},",
            o.idx, o.pet_count, o.grid_w, o.grid_h, o.par_moves, o.par_time
        )
        .unwrap();
        match o.bfs_fuel {
            Some(f) => write!(s, "\"fuel\":{},\"hist\":{:?},", f, o.bfs_history).unwrap(),
            None => s.push_str("\"fuel\":null,\"hist\":null,"),
        }
        write!(s, "\"canned\":{:?},", o.canned_solution).unwrap();
        match o.canned_fuel {
            Some(f) => write!(s, "\"cf\":{}", f).unwrap(),
            None => s.push_str("\"cf\":null"),
        }
        s.push('}');
    }
    s.push(']');
    s.push('\n');

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).expect("create json output dir");
    }
    std::fs::write(out_path, s).expect("write precomputed.json");
    println!(
        "{} {}",
        "Wrote:".bright_green().bold(),
        out_path.display()
    );
}

fn print_summary(outcomes: &[LevelOutcome], elapsed: std::time::Duration) {
    let total = outcomes.len();
    let mut bfs_eq_par = 0;
    let mut bfs_lt_par = 0;
    let mut bfs_gt_par = 0;
    let mut bfs_failed = 0;
    let mut canned_eq_bfs = 0;
    let mut canned_lt_bfs = 0; // shouldn't happen if BFS is optimal
    let mut canned_gt_bfs = 0;
    let mut canned_invalid = 0;
    let mut canned_missing = 0;

    for o in outcomes {
        match o.bfs_fuel {
            None => bfs_failed += 1,
            Some(f) if f == o.par_moves => bfs_eq_par += 1,
            Some(f) if f < o.par_moves => bfs_lt_par += 1,
            Some(_) => bfs_gt_par += 1,
        }
        match (o.bfs_fuel, o.canned_fuel, o.canned_solution.is_empty()) {
            (_, _, true) => canned_missing += 1,
            (_, None, false) => canned_invalid += 1,
            (Some(b), Some(c), false) if b == c => canned_eq_bfs += 1,
            (Some(b), Some(c), false) if c < b => canned_lt_bfs += 1,
            (Some(_), Some(_), false) => canned_gt_bfs += 1,
            _ => {}
        }
    }

    println!();
    println!("{}", "═══ Summary ════════════════════════════════════════".bright_cyan().bold());
    println!(
        "  {} {} levels solved in {:.2}s",
        "Total:".dimmed(),
        total.bold(),
        elapsed.as_secs_f64()
    );
    println!(
        "         ({:.1} levels/sec wall-clock)",
        total as f64 / elapsed.as_secs_f64()
    );
    println!();

    println!("{}", "BFS optimum vs game's par_moves".bold().underline());
    println!(
        "  {} {:>5}  optimum == par   ({})",
        "✓".green().bold(),
        bfs_eq_par.bright_green().bold(),
        "par is exactly optimal"
    );
    println!(
        "  {} {:>5}  optimum  < par   ({})",
        "↓".yellow().bold(),
        bfs_lt_par.yellow().bold(),
        "par is loose; level beatable in fewer moves"
    );
    println!(
        "  {} {:>5}  optimum  > par   ({})",
        "!".red().bold(),
        bfs_gt_par.red().bold(),
        "shouldn't happen — model error or unsolvable level"
    );
    if bfs_failed > 0 {
        println!(
            "  {} {:>5}  BFS failed       (no solution found at all)",
            "✗".red().bold(),
            bfs_failed.red().bold()
        );
    }

    println!();
    println!("{}", "Canned Lumosity solution vs BFS optimum".bold().underline());
    println!(
        "  {} {:>5}  canned == BFS    ({})",
        "✓".green().bold(),
        canned_eq_bfs.bright_green().bold(),
        "the shipped solution is optimal"
    );
    println!(
        "  {} {:>5}  canned  > BFS    ({})",
        "↑".yellow().bold(),
        canned_gt_bfs.yellow().bold(),
        "shipped solution is sub-optimal"
    );
    if canned_lt_bfs > 0 {
        println!(
            "  {} {:>5}  canned  < BFS    ({})",
            "?".magenta().bold(),
            canned_lt_bfs.magenta().bold(),
            "BFS not finding optimum — bug"
        );
    }
    if canned_invalid > 0 {
        println!(
            "  {} {:>5}  canned invalid   ({})",
            "✗".red().bold(),
            canned_invalid.red().bold(),
            "solution string can't be replayed"
        );
    }
    if canned_missing > 0 {
        println!(
            "  {} {:>5}  canned missing   ({})",
            "·".dimmed(),
            canned_missing.dimmed(),
            "no solution string for this level"
        );
    }

    // Per-group breakdown — group by (pet_count, grid_size)
    println!();
    println!("{}", "Per-group breakdown".bold().underline());
    println!(
        "  {:>4}  {:>5}  {:>4}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}",
        "pets".dimmed(),
        "size".dimmed(),
        "n".dimmed(),
        "opt=par".dimmed(),
        "opt<par".dimmed(),
        "opt>par".dimmed(),
        "can=opt".dimmed(),
        "can>opt".dimmed()
    );
    let mut groups: Vec<(u8, u8, u8, Vec<&LevelOutcome>)> = Vec::new();
    for o in outcomes {
        let key = (o.pet_count, o.grid_w, o.grid_h);
        if let Some(g) = groups.iter_mut().find(|g| (g.0, g.1, g.2) == key) {
            g.3.push(o);
        } else {
            groups.push((key.0, key.1, key.2, vec![o]));
        }
    }
    groups.sort_by_key(|g| (g.0, g.1, g.2));
    for (pc, w, h, outs) in &groups {
        let mut eq = 0;
        let mut lt = 0;
        let mut gt = 0;
        let mut ce = 0;
        let mut cg = 0;
        for o in outs {
            match o.bfs_fuel {
                Some(f) if f == o.par_moves => eq += 1,
                Some(f) if f < o.par_moves => lt += 1,
                Some(_) => gt += 1,
                None => {}
            }
            if let (Some(b), Some(c)) = (o.bfs_fuel, o.canned_fuel) {
                if c == b {
                    ce += 1;
                } else if c > b {
                    cg += 1;
                }
            }
        }
        let size_str = format!("{}x{}", w, h);
        println!(
            "  {:>4}  {:>5}  {:>4}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}",
            pc,
            size_str,
            outs.len(),
            if eq > 0 { eq.green().to_string() } else { eq.dimmed().to_string() },
            if lt > 0 { lt.yellow().to_string() } else { lt.dimmed().to_string() },
            if gt > 0 { gt.red().to_string() } else { gt.dimmed().to_string() },
            if ce > 0 { ce.green().to_string() } else { ce.dimmed().to_string() },
            if cg > 0 { cg.yellow().to_string() } else { cg.dimmed().to_string() },
        );
    }

    // Show sample sub-optimal canned solutions, with a histogram of gaps.
    let mut suboptimal: Vec<(&LevelOutcome, u32)> = outcomes
        .iter()
        .filter_map(|o| match (o.bfs_fuel, o.canned_fuel) {
            (Some(b), Some(c)) if c > b => Some((o, c - b)),
            _ => None,
        })
        .collect();
    if !suboptimal.is_empty() {
        // Largest gap first.
        suboptimal.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.idx.cmp(&b.0.idx)));

        println!();
        println!("{}", "Distribution of canned-vs-optimum fuel gap".bold().underline());
        let max_gap = suboptimal.iter().map(|(_, g)| *g).max().unwrap_or(0);
        let mut counts: Vec<usize> = vec![0; (max_gap as usize) + 1];
        for (_, g) in &suboptimal {
            counts[*g as usize] += 1;
        }
        let max_count = *counts.iter().max().unwrap_or(&0);
        let bar_width = 40usize;
        for (gap, &count) in counts.iter().enumerate() {
            if gap == 0 {
                continue;
            }
            let bar_len = if max_count == 0 {
                0
            } else {
                (count * bar_width + max_count - 1) / max_count
            };
            let bar: String = "█".repeat(bar_len);
            println!(
                "  +{:<2} fuel  {:>4}  {}",
                gap.to_string().yellow(),
                count.bold(),
                bar.yellow()
            );
        }

        println!();
        println!(
            "{} {} canned solutions are sub-optimal — top 5 by gap:",
            "Sample:".bold(),
            suboptimal.len().yellow().bold()
        );
        for (o, gap) in suboptimal.iter().take(5) {
            println!(
                "  level {:>4}  pets={:<2}  par={:<3}  BFS={:<3}  canned={:<3}  gap=+{}  optimal=`{}`  canned=`{}`",
                o.idx,
                o.pet_count,
                o.par_moves,
                o.bfs_fuel.unwrap(),
                o.canned_fuel.unwrap(),
                gap.to_string().yellow().bold(),
                o.bfs_history.green(),
                o.canned_solution.yellow(),
            );
        }
    }

}
