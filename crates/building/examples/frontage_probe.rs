//! The frontage's acceptance: a street line turned into plots that tile it, measured both ways.
//!
//! The claim a frontage makes is one claim — **the plots tile the line** — and it has two readings: the
//! fronts' widths sum to the line, and each front starts where the last ended. Everything else here is
//! the sides, the depth, the bend and the refusals.
//!
//! Run with `cargo run -p whiting-building --example frontage_probe`.

use whiting_building::frontage::Frontage;

fn main() {
    println!("# five plots of 4 m on a straight 20 m street, 6 m deep, no gap");
    let straight = Frontage::new(vec![[0.0, 0.0], [20.0, 0.0]], 1.0, 6.0, vec![4.0; 5], 0.0);
    let plots = straight.plots().expect("a straight frontage derives");
    println!(
        "  {} plot(s), line {:.3} m, plots want {:.3} m",
        plots.len(),
        straight.line_length(),
        straight.wanted_length()
    );
    let mut walked = 0.0;
    let mut area = 0.0;
    for plot in &plots {
        let corners = plot.corners();
        area += plan_area(&corners).abs();
        assert!(
            (plot.front[0][0] - walked).abs() < 1.0e-5,
            "plot {} must start where the walk reached: {} against {walked}",
            plot.index,
            plot.front[0][0]
        );
        assert!((plot.front[1][0] - plot.front[0][0] - plot.width).abs() < 1.0e-5);
        walked += plot.width;
    }
    println!(
        "  every front starts where the last ended, and the last ends at {walked:.3} m; plots cover {area:.3} m²",
    );
    assert!(
        (walked - straight.line_length()).abs() < 1.0e-4,
        "the plots tile the line"
    );
    assert!((area - 20.0 * 6.0).abs() < 1.0e-3, "20 x 6 is 120 m²");

    // **A plot faces the street.** Its own facing is the back-to-front direction, and all five are the
    // same one on a straight street: `-z` for a line walked along `+x` with the plots on the left.
    println!("\n# every plot faces the same way on a straight street");
    for plot in &plots {
        let facing = plot.facing().expect("a plot has a facing");
        assert!(
            (facing[0]).abs() < 1.0e-6 && (facing[1] + 1.0).abs() < 1.0e-6,
            "facing {facing:?}"
        );
    }
    println!(
        "  all {} face {:.3},{:.3}",
        plots.len(),
        plots[0].facing().unwrap()[0],
        plots[0].facing().unwrap()[1]
    );

    println!("\n# the other side puts the same plots on the other side of the line");
    let mirrored = Frontage::new(vec![[0.0, 0.0], [20.0, 0.0]], -1.0, 6.0, vec![4.0; 5], 0.0);
    let across = mirrored.plots().expect("the other side derives");
    let lift = across[0].back[0][1] - plots[0].back[0][1];
    println!(
        "  the backs of the two sides are {lift:.3} m apart, and the fronts are the same line"
    );
    assert!(
        (lift.abs() - 12.0).abs() < 1.0e-4,
        "6 m each way is 12 m apart"
    );
    assert!((across[0].front[0][1] - plots[0].front[0][1]).abs() < 1.0e-6);

    println!("\n# a bent street bends the plots with it");
    // A line that turns 90 degrees at its middle: 10 m east, then 10 m north.
    let bent = Frontage::new(
        vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]],
        1.0,
        4.0,
        vec![5.0, 5.0, 5.0, 5.0],
        0.0,
    );
    let corners = bent.plots().expect("a bent frontage derives");
    println!(
        "  {} plot(s) over {:.3} m of bent line",
        corners.len(),
        bent.line_length()
    );
    for plot in &corners {
        println!(
            "    plot {}: front {:.2},{:.2} → {:.2},{:.2}; faces {:.2},{:.2}",
            plot.index,
            plot.front[0][0],
            plot.front[0][1],
            plot.front[1][0],
            plot.front[1][1],
            plot.facing().unwrap()[0],
            plot.facing().unwrap()[1]
        );
    }
    // **The bend turns the facing.** Plots 0 and 1 stand on the first segment and face `-z`; plots 2 and
    // 3 stand on the second and face `+x`, which is where the walk's own left is after the turn.
    for plot in &corners[..2] {
        let facing = plot.facing().expect("a plot has a facing");
        assert!(
            (facing[1] + 1.0).abs() < 1.0e-5,
            "plot {} turns early: {facing:?}",
            plot.index
        );
    }
    for plot in &corners[2..] {
        let facing = plot.facing().expect("a plot has a facing");
        assert!(
            (facing[0] - 1.0).abs() < 1.0e-5,
            "plot {} turns late: {facing:?}",
            plot.index
        );
    }
    println!("  the first two face -z and the last two face +x: the bend is in the plots");

    println!("\n# a gap is a gap, and a party wall is a gap of zero");
    // Five 4 m plots and four 0.5 m gaps want 22 m, so the street is 22 m long.
    let spaced = Frontage::new(vec![[0.0, 0.0], [22.0, 0.0]], 1.0, 6.0, vec![4.0; 5], 0.5);
    let apart = spaced.plots().expect("a frontage with gaps derives");
    let between = apart[1].front[0][0] - apart[0].front[1][0];
    println!(
        "  five 4 m plots with a 0.5 m gap want {:.2} m and the first two are {between:.2} m apart",
        spaced.wanted_length()
    );
    assert!((between - 0.5).abs() < 1.0e-5);

    println!("\n# a frontage that is not a frontage is refused by name");
    let cases: [(&str, Frontage, &str); 6] = [
        (
            "a line of one point",
            Frontage::new(vec![[0.0, 0.0]], 1.0, 6.0, vec![4.0], 0.0),
            "LineTooShort",
        ),
        (
            "a side of zero",
            Frontage::new(vec![[0.0, 0.0], [20.0, 0.0]], 0.0, 6.0, vec![4.0], 0.0),
            "SideNotASide",
        ),
        (
            "a depth of zero",
            Frontage::new(vec![[0.0, 0.0], [20.0, 0.0]], 1.0, 0.0, vec![4.0], 0.0),
            "DepthNotALength",
        ),
        (
            "a plot of no width",
            Frontage::new(vec![[0.0, 0.0], [20.0, 0.0]], 1.0, 6.0, vec![4.0, 0.0], 0.0),
            "WidthNotALength",
        ),
        (
            "a negative gap",
            Frontage::new(vec![[0.0, 0.0], [20.0, 0.0]], 1.0, 6.0, vec![4.0], -0.5),
            "GapIsNegative",
        ),
        (
            "more plots than line",
            Frontage::new(vec![[0.0, 0.0], [10.0, 0.0]], 1.0, 6.0, vec![4.0; 5], 0.0),
            "LineTooShortForThePlots",
        ),
    ];
    for (label, frontage, expect) in cases {
        // The fault's own name, without the numbers: `Some(WidthNotALength { .. })` is `WidthNotALength`.
        let fault = frontage.fault();
        let named = format!("{fault:?}");
        let named = named.trim_start_matches("Some(");
        let named = named
            .split(' ')
            .next()
            .unwrap_or("")
            .split('(')
            .next()
            .unwrap_or("")
            .to_owned();

        let _ = &named;
        println!("  {label}: {named} (expected {expect})");
        assert!(
            named == expect,
            "{label} must be {expect}, and the answer was {named}"
        );
    }
}

/// Returns the signed plan area of a loop.
fn plan_area(loop_: &[[f32; 2]]) -> f32 {
    let mut twice = 0.0_f32;
    for index in 0..loop_.len() {
        let from = loop_[index];
        let to = loop_[(index + 1) % loop_.len()];
        twice += from[0] * to[1] - to[0] * from[1];
    }
    twice * 0.5
}
