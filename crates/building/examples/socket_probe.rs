//! The socket slice's acceptance: a part hung on a feature, and whether it follows that feature when
//! the document moves underneath it.
//!
//! A pin says "0.11 m along, 2.47 m up" and knows nothing about the door it hangs over. A joint says
//! "my back socket sits on the doorway's head", so moving the doorway's head must move the canopy by
//! exactly as much — that is the whole difference between placing a part and connecting it, and it is
//! the number this probe exists to print.
//!
//! Run with `cargo run -p whiting-building --example socket_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::openings::{Inset, Opening};
use whiting_building::ops;
use whiting_building::parts::{Anchor, Joint, Pin, Placement, Socket, Template};
use whiting_building::{Building, Mass, MassId, PlanNode};

/// Returns a canopy `width` across, `thickness` thick, reaching `reach` out of the wall, with a
/// `"back"` socket at the middle of its top-back edge — the line it hangs from.
///
/// The body keeps its floor strictly between its soffit and its top, because a `Mass` is a room and a
/// plate with a coincident floor is a degenerate one.
fn canopy(width: f32, thickness: f32, reach: f32) -> Template {
    let slab = thickness * 0.5;
    let floor = slab;
    let outline = [[0.0, -reach], [width, -reach], [width, 0.0], [0.0, 0.0]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, floor))
        .collect();
    Template::new(
        "canopy",
        vec![
            Mass::new(outline, thickness)
                .with_wall(0.02)
                .with_slab(slab),
        ],
        Vec::new(),
    )
    .with_socket(Socket::new("back", (width * 0.5, thickness)))
}

/// Builds the shed document with a doorway cut to `head` inset, and a canopy hung on its head.
fn shed_with(head: f32, hang_on_head: bool) -> Building {
    let plan = Starter {
        width: 5.0,
        depth: 8.0,
        height: 2.8,
        roof: RoofStyle::Gable,
        ..Starter::default()
    };
    let mut skeleton = starter(&plan).skeleton.clone();
    let mass = MassId(0);
    let front = FaceRef::new(mass, 2);
    for face in skeleton.faces_of(mass) {
        ops::split_face(
            &mut skeleton,
            face,
            &Rhythm::Even { count: 3 },
            &Rhythm::Even { count: 1 },
        );
    }
    let doorway = ops::add_opening(
        &mut skeleton,
        Opening::new(front, 0, 1, Inset::new(0.25, 0.25, 0.0, head)),
    )
    .expect("the middle bay is in the grid");
    let shelter = ops::add_template(&mut skeleton, canopy(1.45, 0.10, 0.70));
    let placement = if hang_on_head {
        Placement::new(shelter, front, 0, 1, Pin::new(0.0, 0.0))
            .with_joint(Joint::new("back", Anchor::OpeningHead { opening: doorway }))
    } else {
        // The free placement the shed scene used before sockets existed: the same canopy, pinned at
        // numbers that were worked out by hand.
        Placement::new(shelter, front, 0, 1, Pin::new(0.11, 2.47))
    };
    ops::place_part(&mut skeleton, placement).expect("the doorway is in a bay of the grid");
    Building::new(skeleton)
}

fn main() {
    println!("# where a canopy lands, hung on the doorway's head");
    for head in [0.45_f32, 0.25] {
        let building = shed_with(head, true);
        let built = building.build().expect("the shed expands");
        let (low, high) = built.bounds_of(whiting_building::SolidId::Part(0));
        let hole = building
            .skeleton
            .aperture(0)
            .expect("the doorway was cut")
            .1;
        println!(
            "  doorway head at {:.4}: canopy y [{:.4}, {:.4}], hole head {:.4}, its top is {:.4} \
             above the hole's head",
            hole.high,
            low[1],
            high[1],
            hole.high,
            high[1] - hole.high
        );
    }

    println!("\n# the same canopy pinned by numbers, moved the same way");
    for head in [0.45_f32, 0.25] {
        let building = shed_with(head, false);
        let built = building.build().expect("the shed expands");
        let (low, high) = built.bounds_of(whiting_building::SolidId::Part(0));
        let hole = building
            .skeleton
            .aperture(0)
            .expect("the doorway was cut")
            .1;
        println!(
            "  doorway head at {:.4}: canopy y [{:.4}, {:.4}], its top is {:.4} above the hole's head",
            hole.high,
            low[1],
            high[1],
            high[1] - hole.high
        );
    }

    println!("\n# a part hung on a bay's own head, measured against that bay");
    let mut skeleton = shed_with(0.45, true).skeleton;
    let front = FaceRef::new(MassId(0), 2);
    let shelter = skeleton.parts.template_count() - 1;
    let bay = skeleton.bay(front, 0, 1).expect("the grid is there");
    ops::place_part(
        &mut skeleton,
        Placement::new(shelter, front, 0, 2, Pin::new(0.0, 0.0))
            .with_joint(Joint::new("back", Anchor::BayHead)),
    )
    .expect("the third bay is in the grid");
    let placed = skeleton.placed(1).expect("just placed");
    println!(
        "  bay [{:.4}, {:.4}] x [{:.4}, {:.4}]: its head anchor is at along {:.4} up {:.4}, and the \
         canopy's socket puts its origin at along {:.4} up {:.4}",
        bay.from,
        bay.to,
        bay.low,
        bay.high,
        bay.width() * 0.5,
        bay.height(),
        placed.at.along,
        placed.at.up
    );

    println!("\n# the two faults a joint can have, refused by the verb and named by the validator");
    let mut document = shed_with(0.45, true).skeleton;
    let template = document.parts.template(0).cloned().expect("the canopy");
    let bad_socket = Placement::new(0, front, 0, 1, Pin::new(0.0, 0.0))
        .with_joint(Joint::new("sill", Anchor::OpeningHead { opening: 0 }));
    println!(
        "  a socket the template does not name: the verb answers {:?}",
        ops::place_part(&mut document, bad_socket.clone())
    );
    let mut hand_written = document.clone();
    let mut parts = whiting_building::parts::Parts::new();
    parts.add_template(template.clone());
    parts.place(bad_socket);
    hand_written.parts = parts;
    let named: Vec<String> = Building::new(hand_written)
        .validate()
        .iter()
        .map(|violation| format!("{violation:?}"))
        .filter(|text| text.contains("PlacementSocketMissing"))
        .collect();
    println!("  the same document handed to the validator: {named:?}");

    let wrong_wall = Placement::new(0, FaceRef::new(MassId(0), 1), 0, 0, Pin::new(0.0, 0.0))
        .with_joint(Joint::new("back", Anchor::OpeningHead { opening: 0 }));
    println!(
        "  an anchor naming an opening in another wall: the verb answers {:?}",
        ops::place_part(&mut document, wrong_wall.clone())
    );
    let mut hand_written = document.clone();
    let mut parts = whiting_building::parts::Parts::new();
    parts.add_template(template);
    parts.place(wrong_wall);
    hand_written.parts = parts;
    let named: Vec<String> = Building::new(hand_written)
        .validate()
        .iter()
        .map(|violation| format!("{violation:?}"))
        .filter(|text| text.contains("PlacementAnchorMissing"))
        .collect();
    println!("  the same document handed to the validator: {named:?}");
}
