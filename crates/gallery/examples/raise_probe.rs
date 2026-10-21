//! Measures `ops::raise` on one of the gallery's own houses: the document must still expand clean raised by each terrace rise, and the
//! openings must stay inside their walls (round 169's two failures were `OpeningOutsideWall { past }` equal to the raise itself).

use building::ops;

fn main() {
    for delta in [0.0_f32, 0.07, 1.65, 3.30] {
        let mut building = whiting_gallery::earthen::document();
        ops::raise(&mut building.skeleton, delta);
        match building.validate() {
            problems if problems.is_empty() => println!("raise {delta:+.2}: clean"),
            problems => {
                for problem in problems {
                    println!("raise {delta:+.2}: {problem:?}");
                }
            }
        }
    }
}
