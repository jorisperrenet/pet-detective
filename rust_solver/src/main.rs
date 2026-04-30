//! Solve every shipped Pet Detective level, print a summary, and write the
//! per-level optimum out to `web/public/data/precomputed.json` for the
//! browser app to consume.
//!
//! Output JSON: one entry per level, fields used by `web/src/lib/precomputed.ts`:
//!   { "i", "p", "w", "h", "par", "t", "fuel", "hist", "canned", "cf" }

use std::collections::BTreeMap;
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
        "{} {} levels, {} supplied solutions",
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

    // Per-pet-count totals and per-pet-count imperfect counts.
    let mut total_by_pet: BTreeMap<u8, usize> = BTreeMap::new();
    let mut imperfect_by_pet: BTreeMap<u8, usize> = BTreeMap::new();
    // Counts per fuel-gap (gap = supplied_fuel - bfs_fuel; only positive gaps).
    let mut count_by_gap: BTreeMap<u32, usize> = BTreeMap::new();
    let mut optimal_count: usize = 0;
    let mut sample: Vec<(&LevelOutcome, u32)> = Vec::new();

    for o in outcomes {
        *total_by_pet.entry(o.pet_count).or_default() += 1;
        match (o.bfs_fuel, o.canned_fuel) {
            (Some(b), Some(c)) if c > b => {
                let gap = c - b;
                *count_by_gap.entry(gap).or_default() += 1;
                *imperfect_by_pet.entry(o.pet_count).or_default() += 1;
                sample.push((o, gap));
            }
            _ => optimal_count += 1,
        }
    }
    sample.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.idx.cmp(&b.0.idx)));

    // Top section: how many supplied solutions are optimal vs off-by-N fuel.
    println!();
    println!(
        "  {} {} {}",
        "Lumosity's supplied solutions:".bold(),
        format!("{} levels", total).dimmed(),
        format!("· solved in {:.2} s ({:.0} levels/sec)", elapsed.as_secs_f64(), total as f64 / elapsed.as_secs_f64()).dimmed()
    );
    println!("  ─────────────────────────────");
    let bar_w_top = 40usize;
    let max_top = optimal_count
        .max(*count_by_gap.values().max().unwrap_or(&0));
    let scale_bar = |n: usize, width: usize, max: usize| -> String {
        if max == 0 || n == 0 {
            return String::new();
        }
        let len = ((n * width + max - 1) / max).max(1).min(width);
        "█".repeat(len)
    };
    println!(
        "    {:<11}  {:<width$}  {:>4}   {:>4.1}%",
        "optimal".green(),
        scale_bar(optimal_count, bar_w_top, max_top).green(),
        optimal_count.bold(),
        100.0 * optimal_count as f64 / total as f64,
        width = bar_w_top
    );
    for (&gap, &count) in &count_by_gap {
        let label = format!("+{} fuel", gap);
        println!(
            "    {:<11}  {:<width$}  {:>4}   {:>4.1}%",
            label.yellow(),
            scale_bar(count, bar_w_top, max_top).yellow(),
            count.bold(),
            100.0 * count as f64 / total as f64,
            width = bar_w_top
        );
    }

    // Where the imperfections live (by pet count). Skip if everything's perfect.
    if !imperfect_by_pet.is_empty() {
        let max_imperfect = *imperfect_by_pet.values().max().unwrap();
        let bar_w_pet = 12usize;

        // Aggregate the all-perfect low pet-counts into one "X-Y pets" row.
        let pets: Vec<u8> = total_by_pet.keys().copied().collect();
        let lowest_imperfect = *imperfect_by_pet.keys().next().unwrap();
        let mut perfect_total = 0usize;
        let mut perfect_low: Option<u8> = None;
        let mut perfect_high: Option<u8> = None;
        for &pc in &pets {
            if pc < lowest_imperfect {
                perfect_total += total_by_pet[&pc];
                perfect_low.get_or_insert(pc);
                perfect_high = Some(pc);
            }
        }

        println!();
        println!("  {}", "Sub-optimal levels by pet count".bold());
        println!("  ───────────────────────────────");
        if let (Some(lo), Some(hi)) = (perfect_low, perfect_high) {
            let label = if lo == hi {
                format!("{} pets", lo)
            } else {
                format!("{}–{} pets", lo, hi)
            };
            println!(
                "    {:<10}  {:<width$}  {:>2} / {:>4}    {:>4.1}%",
                label,
                "·".dimmed(),
                0.dimmed(),
                perfect_total.dimmed(),
                0.0,
                width = bar_w_pet
            );
        }
        for (&pc, &imp) in &imperfect_by_pet {
            let total_pc = total_by_pet[&pc];
            let pct = 100.0 * imp as f64 / total_pc as f64;
            let label = format!("{} pets", pc);
            let bar = scale_bar(imp, bar_w_pet, max_imperfect);
            let pad: String = " ".repeat(bar_w_pet.saturating_sub(bar.chars().count()));
            println!(
                "    {:<10}  {}{}  {:>2} / {:>4}    {:>4.1}%",
                label,
                bar.yellow(),
                pad,
                imp.bold(),
                total_pc,
                pct
            );
        }
    }

    // Largest fuel gaps — concrete examples.
    if !sample.is_empty() {
        let shown = 5.min(sample.len());
        let header = if sample.len() > shown {
            format!("Worst gaps (showing {} of {})", shown, sample.len())
        } else {
            "Worst gaps".to_string()
        };
        let underline: String = "─".repeat(header.chars().count());
        println!();
        println!("  {}", header.bold());
        println!("  {}", underline);
        for (o, gap) in sample.iter().take(shown) {
            println!(
                "    level {:>4}  {:>2} pets  given {} → optimal {}  ({})  optimal=`{}`  given=`{}`",
                o.idx,
                o.pet_count,
                o.par_moves,
                o.bfs_fuel.unwrap(),
                format!("+{}", gap).yellow().bold(),
                o.bfs_history.green(),
                o.canned_solution.yellow(),
            );
        }
    }
}
