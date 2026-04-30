//! Step-bounded forward search over (current node, trunk-bitmask, anims-left-bitmask).
//!
//! - At each step, pick a target = (a house whose pet is in trunk) OR (if trunk has < 4 pets, any not-yet-picked pet).
//! - Cost added = road distance from current to target (precomputed).
//! - Goal: trunk == 0 && anims_left == 0.
//! - Solution string in Lumosity convention: uppercase = pet pickup, lowercase = house drop, in visit order.
//!
//! State dedup uses a flat Vec keyed by a base-3 joint encoding of (trunk, anim_left)
//! plus the current node. The joint id is maintained incrementally between transitions
//! (pickup: +3^bit; drop: -2·3^bit), so dedup is a single array index.
//!
//! For 11-pet levels the dedup table is 32 MB which doesn't fit in L3, so the inner
//! loop software-prefetches the next iteration's slot to overlap DRAM latency with
//! compute on the current expansion.
//!
//! Buffers (table, records, state lists) live in thread-local storage so rayon
//! workers reuse allocations across levels.

use std::cell::RefCell;

use crate::level::Level;

const MAX_TRUNK: u32 = 4;
const MAX_PETS: usize = 12;

#[derive(Clone, Debug)]
pub struct SolveResult {
    pub fuel: u32,
    pub history: String,
    pub trunk_max: u8,
}

#[derive(Clone, Copy)]
struct Record {
    parent: u32,    // u32::MAX for the root
    event_char: u8, // 0 for the root; otherwise ASCII pickup ('A'+i) or drop ('a'+i)
}

#[derive(Clone, Copy)]
struct State {
    trunk: u32,
    anim_left: u32,
    joint_id: u32,
    current: u8,
}

#[derive(Clone, Copy)]
struct Slot {
    fuel: u32, // u32::MAX = unset
    rec_idx: u32,
}

const EMPTY_SLOT: Slot = Slot { fuel: u32::MAX, rec_idx: 0 };

struct Buf {
    table: Vec<Slot>, // index = joint_id * n_nodes + current
    records: Vec<Record>,
    cur_states: Vec<State>,
    next_states: Vec<State>,
    touched: Vec<u32>,
}

impl Buf {
    const fn new() -> Self {
        Self {
            table: Vec::new(),
            records: Vec::new(),
            cur_states: Vec::new(),
            next_states: Vec::new(),
            touched: Vec::new(),
        }
    }
}

thread_local! {
    static BUF: RefCell<Buf> = const { RefCell::new(Buf::new()) };
}

#[inline(always)]
fn prefetch(ptr: *const u8) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use core::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
        _mm_prefetch::<{ _MM_HINT_T0 }>(ptr as *const i8);
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = ptr;
    }
}

pub fn solve(level: &Level) -> Option<SolveResult> {
    solve_with_ub(level, u32::MAX)
}

/// Like `solve`, but prunes states whose fuel + an admissible remaining-cost
/// lower bound exceeds `ub`. A `Some(canned_fuel)` from `replay` is a valid
/// upper bound: pass it as `ub` to skip provably-non-optimal branches.
///
/// The admissible LB used is `remaining_events`: every event traverses ≥ 1
/// edge in the road graph, since each event lands at a unique pet/house cell
/// distinct from the cell of the prior event.
pub fn solve_with_ub(level: &Level, ub: u32) -> Option<SolveResult> {
    let n_pets = level.pet_count as u32;
    if n_pets == 0 {
        return Some(SolveResult { fuel: 0, history: String::new(), trunk_max: 0 });
    }
    let all_anims: u32 = (1u32 << n_pets) - 1;

    let (_important, dist) = level.all_pairs_distances();
    let n_nodes = (1 + 2 * n_pets) as usize;
    debug_assert!(_important.len() == n_nodes);

    let mut pow3 = [1u32; MAX_PETS + 1];
    for i in 1..=MAX_PETS {
        pow3[i] = pow3[i - 1] * 3;
    }
    let joint_size = pow3[n_pets as usize] as usize;
    let state_size = joint_size * n_nodes;

    BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        let buf = &mut *buf;

        for &id in &buf.touched {
            buf.table[id as usize] = EMPTY_SLOT;
        }
        buf.touched.clear();
        if buf.table.len() < state_size {
            buf.table.resize(state_size, EMPTY_SLOT);
        }

        buf.records.clear();
        buf.records.push(Record { parent: u32::MAX, event_char: 0 });

        let init_joint: u32 = (pow3[n_pets as usize] - 1) / 2;
        let init_state_id = init_joint as usize * n_nodes;
        buf.table[init_state_id] = Slot { fuel: 0, rec_idx: 0 };
        buf.touched.push(init_state_id as u32);

        buf.cur_states.clear();
        buf.cur_states.push(State {
            trunk: 0,
            anim_left: all_anims,
            joint_id: init_joint,
            current: 0,
        });

        let total_steps = 2 * n_pets as usize;
        for step in 0..total_steps {
            buf.next_states.clear();
            let n_cur = buf.cur_states.len();
            // After this step's expansion, new states are at step+1, with
            // (total_steps - step - 1) more events to do. Each event ≥ 1 fuel.
            let max_new_fuel = ub.saturating_sub((total_steps - step - 1) as u32);

            // Prefetch the first cur_state's slot to warm cache.
            if n_cur > 0 {
                let s0 = buf.cur_states[0];
                let id0 = (s0.joint_id as usize) * n_nodes + s0.current as usize;
                unsafe {
                    prefetch(buf.table.as_ptr().add(id0) as *const u8);
                }
            }

            for i in 0..n_cur {
                // Prefetch upcoming cur_state's slot (~4 ahead) to overlap latency.
                if i + 4 < n_cur {
                    let sn = buf.cur_states[i + 4];
                    let idn = (sn.joint_id as usize) * n_nodes + sn.current as usize;
                    unsafe {
                        prefetch(buf.table.as_ptr().add(idn) as *const u8);
                    }
                }

                let State { trunk, anim_left, joint_id, current } = buf.cur_states[i];
                let cur_state_id = (joint_id as usize) * n_nodes + current as usize;
                let cur_slot = buf.table[cur_state_id];
                let fuel = cur_slot.fuel;
                let rec_idx = cur_slot.rec_idx;
                let trunk_count = trunk.count_ones();
                let row = current as usize * n_nodes;

                // Drop-offs
                let mut t = trunk;
                while t != 0 {
                    let bit = t.trailing_zeros();
                    t &= t - 1;
                    let target = (1 + n_pets + bit) as u8;
                    let step = dist[row + target as usize];
                    if step == u16::MAX {
                        continue;
                    }
                    let new_trunk = trunk & !(1u32 << bit);
                    let new_joint = joint_id - 2 * pow3[bit as usize];
                    let new_state_id = (new_joint as usize) * n_nodes + target as usize;

                    // Prefetch the next iteration's slot if there is one.
                    if t != 0 {
                        let nbit = t.trailing_zeros();
                        let ntgt = (1 + n_pets + nbit) as u8;
                        let nj = joint_id - 2 * pow3[nbit as usize];
                        let nid = (nj as usize) * n_nodes + ntgt as usize;
                        unsafe {
                            prefetch(buf.table.as_ptr().add(nid) as *const u8);
                        }
                    }

                    let new_fuel = fuel + step as u32;
                    if new_fuel > max_new_fuel {
                        continue;
                    }
                    let prev = buf.table[new_state_id];
                    if new_fuel < prev.fuel {
                        let new_rec_idx = buf.records.len() as u32;
                        buf.records.push(Record {
                            parent: rec_idx,
                            event_char: b'a' + bit as u8,
                        });
                        if prev.fuel == u32::MAX {
                            buf.touched.push(new_state_id as u32);
                            buf.next_states.push(State {
                                trunk: new_trunk,
                                anim_left,
                                joint_id: new_joint,
                                current: target,
                            });
                        }
                        buf.table[new_state_id] = Slot { fuel: new_fuel, rec_idx: new_rec_idx };
                    }
                }

                // Pickups
                if trunk_count < MAX_TRUNK {
                    let mut a = anim_left;
                    while a != 0 {
                        let bit = a.trailing_zeros();
                        a &= a - 1;
                        let target = (1 + bit) as u8;
                        let step = dist[row + target as usize];
                        if step == u16::MAX {
                            continue;
                        }
                        let new_trunk = trunk | (1u32 << bit);
                        let new_anim_left = anim_left & !(1u32 << bit);
                        let new_joint = joint_id + pow3[bit as usize];
                        let new_state_id = (new_joint as usize) * n_nodes + target as usize;

                        if a != 0 {
                            let nbit = a.trailing_zeros();
                            let ntgt = (1 + nbit) as u8;
                            let nj = joint_id + pow3[nbit as usize];
                            let nid = (nj as usize) * n_nodes + ntgt as usize;
                            unsafe {
                                prefetch(buf.table.as_ptr().add(nid) as *const u8);
                            }
                        }

                        let new_fuel = fuel + step as u32;
                        if new_fuel > max_new_fuel {
                            continue;
                        }
                        let prev = buf.table[new_state_id];
                        if new_fuel < prev.fuel {
                            let new_rec_idx = buf.records.len() as u32;
                            buf.records.push(Record {
                                parent: rec_idx,
                                event_char: b'A' + bit as u8,
                            });
                            if prev.fuel == u32::MAX {
                                buf.touched.push(new_state_id as u32);
                                buf.next_states.push(State {
                                    trunk: new_trunk,
                                    anim_left: new_anim_left,
                                    joint_id: new_joint,
                                    current: target,
                                });
                            }
                            buf.table[new_state_id] = Slot { fuel: new_fuel, rec_idx: new_rec_idx };
                        }
                    }
                }
            }

            std::mem::swap(&mut buf.cur_states, &mut buf.next_states);
            if buf.cur_states.is_empty() {
                return None;
            }
        }

        // Goal: joint_id == 0 (all dropped). Find min fuel over `current`.
        let mut best: Option<(u32, u32)> = None;
        for current in 0..n_nodes {
            let id = current;
            let slot = buf.table[id];
            if slot.fuel != u32::MAX {
                match best {
                    None => best = Some((slot.fuel, slot.rec_idx)),
                    Some((bf, _)) if slot.fuel < bf => best = Some((slot.fuel, slot.rec_idx)),
                    _ => {}
                }
            }
        }

        let (fuel, mut rec_idx) = best?;

        let mut chars: Vec<u8> = Vec::new();
        while rec_idx != u32::MAX {
            let r = &buf.records[rec_idx as usize];
            if r.event_char != 0 {
                chars.push(r.event_char);
            }
            rec_idx = r.parent;
        }
        chars.reverse();

        let mut trunk_max: u8 = 0;
        let mut trunk_count: u8 = 0;
        for &c in &chars {
            if c.is_ascii_uppercase() {
                trunk_count += 1;
                if trunk_count > trunk_max {
                    trunk_max = trunk_count;
                }
            } else {
                trunk_count -= 1;
            }
        }
        let history = String::from_utf8(chars).expect("ASCII");

        Some(SolveResult { fuel, history, trunk_max })
    })
}

/// Replay a Lumosity solution string and compute its fuel cost.
pub fn replay(level: &Level, sol: &str) -> Option<u32> {
    if sol.is_empty() {
        return None;
    }
    let n_pets = level.pet_count as u32;
    let (_, dist) = level.all_pairs_distances();
    let n_nodes = (1 + 2 * n_pets) as usize;
    let pet_node = |i: u32| 1 + i as usize;
    let house_node = |i: u32| 1 + n_pets as usize + i as usize;

    let mut current: usize = 0;
    let mut trunk: u32 = 0;
    let mut anims_left: u32 = (1u32 << n_pets) - 1;
    let mut fuel: u32 = 0;

    for ch in sol.chars() {
        let (target, new_trunk, new_anims) = if ch.is_ascii_uppercase() {
            let idx = ch as u32 - 'A' as u32;
            if anims_left & (1u32 << idx) == 0 || trunk.count_ones() >= MAX_TRUNK {
                return None;
            }
            (pet_node(idx), trunk | (1u32 << idx), anims_left & !(1u32 << idx))
        } else if ch.is_ascii_lowercase() {
            let idx = ch as u32 - 'a' as u32;
            if trunk & (1u32 << idx) == 0 {
                return None;
            }
            (house_node(idx), trunk & !(1u32 << idx), anims_left)
        } else {
            return None;
        };
        let step = dist[current * n_nodes + target];
        if step == u16::MAX {
            return None;
        }
        fuel += step as u32;
        current = target;
        trunk = new_trunk;
        anims_left = new_anims;
    }

    if trunk == 0 && anims_left == 0 {
        Some(fuel)
    } else {
        None
    }
}
