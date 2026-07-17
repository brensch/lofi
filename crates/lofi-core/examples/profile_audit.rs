use lofi_core::music::{Arrangement, Role};

fn main() {
    let roster = [1, 2, 3];
    for (seed, phrase) in [(0, 4), (1, 3), (2, 5), (6, 13)] {
        print!("probe seed {seed} phrase {phrase}:");
        for ids in [&[1][..], &[1, 2][..], &[1, 2, 3][..]] {
            let arrangement = Arrangement::at(seed, ids, phrase);
            print!(
                " {:?}/{:?}",
                arrangement.spotlight, arrangement.params.bass_walk
            );
        }
        println!();
    }
    print!("seed 6 spotlight sequence:");
    for phrase in 8..17 {
        let value = Arrangement::at(6, &roster, phrase);
        print!(" {phrase}:{:?}", value.spotlight);
    }
    println!();
    for source_slot in 0..3_u64 {
        println!("source {source_slot}");
        find(source_slot, source_slot, &roster, "half", |arrangement| {
            arrangement.spotlight == Role::Pulse && arrangement.params.half_time
        });
        find(
            source_slot,
            source_slot + 3,
            &roster,
            "double",
            |arrangement| {
                arrangement.spotlight == Role::Pocket && arrangement.params.hat_density == 2
            },
        );
        find(
            source_slot,
            source_slot + 6,
            &roster,
            "walk",
            |arrangement| arrangement.spotlight == Role::Low && arrangement.params.bass_walk,
        );
        find(
            source_slot,
            source_slot + 9,
            &roster,
            "sparse",
            |arrangement| {
                arrangement.spotlight == Role::Pocket && arrangement.params.hat_density == 0
            },
        );
    }
}

fn find(
    source_slot: u64,
    first_seed: u64,
    roster: &[u64],
    label: &str,
    matches: impl Fn(Arrangement) -> bool,
) {
    for seed in (first_seed..100_000).step_by(3) {
        for phrase in 1..64 {
            let arrangement = Arrangement::at(seed, roster, phrase);
            if matches(arrangement) {
                println!("  {label}: seed {seed}, phrase {phrase}");
                return;
            }
        }
    }
    panic!("no {label} profile for source {source_slot}");
}
