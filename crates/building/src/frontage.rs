//! **A frontage**: a street line, and the plots that stand along it.
//!
//! # What this is for
//!
//! The building arcs stop at one building per scene. A street is not a building: its buildings share
//! their side walls, they face one line, and their ground steps with the street's slope. This module is
//! the smallest layer that answers those three questions and nothing else — **it derives plots, it does
//! not draw a town**. What stands on a plot is an author's decision, made with the constructs that
//! already exist: a `Mass`, an eave, a roof, a covering.
//!
//! # The one derivation
//!
//! A frontage is a polyline, a side, a depth, and the width of each plot. Walking the line by the plots'
//! widths (and the gap between them) gives, for each plot, the segment of the line it faces and the
//! segment behind it. **The plots tile the line exactly**: their widths and the gaps between them sum to
//! the length walked, so no plot overlaps its neighbour and none of the line is left over — which is what
//! makes a party wall a party wall rather than two walls with a street between them.
//!
//! # What is not here
//!
//! - **No bend mitering.** A plot's back is its own front segment offset by the depth, so at a bend the
//!   backs of two neighbouring plots leave a wedge behind them. A settlement that fills its blocks needs
//!   the block's own depth polygon, which is a second derivation and a different question.
//! - **No party wall as one wall.** Two plots that touch are two masses that touch, and the crate's
//!   answer to "one wall between two houses" is *two walls meeting*, which is what a real party wall is
//!   built as too — the drawing sees one thickness because the joint is closed.
//! - **No placement.** Which building goes on which plot, how tall, with what roof: the frontage says
//!   where the plots are and an author says what stands on them.

use crate::math::{Vec2, length, normalized};

/// A street's frontage: the line the buildings face, and the plots along it.
#[derive(Debug, Clone, PartialEq)]
pub struct Frontage {
    /// The line the plots face, walked in order. Two points are a straight street; more are a bent one.
    pub line: Vec<Vec2>,
    /// Which side of the walk the plots are on: `1` is the walker's left, `-1` the right.
    ///
    /// # Invariants
    ///
    /// It is **read and not guessed**: the walk's own direction decides which way the plots go, and a
    /// frontage whose side is zero is refused rather than defaulted, because "the plots are on the left"
    /// and "the plots are on the right" are two different streets.
    pub side: f32,
    /// How far back from the line each plot reaches, in metres.
    pub depth: f32,
    /// The frontage width of each plot, in the order they stand along the line.
    pub widths: Vec<f32>,
    /// The gap between two neighbouring plots, in metres. **Zero is a party wall.**
    pub gap: f32,
}

/// One plot: the segment of the frontage it faces, and the segment behind it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plot {
    /// Which plot along the frontage, counting from the line's own start.
    pub index: usize,
    /// The two ends of the plot's frontage, in the walk's order.
    pub front: [Vec2; 2],
    /// The two ends of the plot's back, the same order as [`Plot::front`].
    pub back: [Vec2; 2],
    /// The frontage width this plot was given.
    pub width: f32,
}

impl Plot {
    /// Returns the plot's four corners, positively wound in plan and stored open.
    ///
    /// # Invariants
    ///
    /// **The winding is normalised to the crate's own**, which is what a mass's outline has to be: the
    /// front is walked in the street's own direction and the back comes back the other way, and whether
    /// that comes out positive depends on which side of the line the plots are on. So the quad is built
    /// and then reversed when its own plan area is negative — one reading of one number, and a plot does
    /// not have a winding of its own to get wrong.
    pub fn corners(&self) -> Vec<Vec2> {
        let mut corners = vec![self.front[0], self.front[1], self.back[1], self.back[0]];
        if plan_area(&corners) < 0.0 {
            corners.reverse();
        }
        corners
    }

    /// Returns the plot's centre in plan.
    pub fn centre(&self) -> Vec2 {
        [
            (self.front[0][0] + self.front[1][0] + self.back[1][0] + self.back[0][0]) * 0.25,
            (self.front[0][1] + self.front[1][1] + self.back[1][1] + self.back[0][1]) * 0.25,
        ]
    }

    /// Returns the direction the plot faces, from its back toward its front, normalised.
    pub fn facing(&self) -> Option<Vec2> {
        let back = midpoint(self.back);
        let front = midpoint(self.front);
        let out = [front[0] - back[0], front[1] - back[1]];
        if length(out) < 1.0e-6 {
            None
        } else {
            Some(normalized(out))
        }
    }
}

/// What is wrong with a [`Frontage`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrontageFault {
    /// A line of fewer than two points, so there is no street to walk.
    LineTooShort {
        /// How many points the line has.
        points: usize,
    },
    /// A side that is not `1` or `-1`, so nobody knows which way the plots go.
    SideNotASide {
        /// The side that was given.
        side: f32,
    },
    /// A depth that is not a positive length.
    DepthNotALength {
        /// The depth that was given.
        depth: f32,
    },
    /// A plot whose width is not a positive length.
    WidthNotALength {
        /// Which plot, counting from the start.
        plot: usize,
        /// The width that was given.
        width: f32,
    },
    /// A negative gap, which would make two neighbours overlap.
    GapIsNegative {
        /// The gap that was given.
        gap: f32,
    },
    /// No plots at all: a frontage with no building on it.
    NoPlots,
    /// The plots ask for more line than there is, so the last one would stand past the end of the street.
    LineTooShortForThePlots {
        /// How much line the plots ask for.
        wanted: f32,
        /// How much line there is.
        available: f32,
    },
}

impl Frontage {
    /// Builds a frontage.
    pub fn new(line: Vec<Vec2>, side: f32, depth: f32, widths: Vec<f32>, gap: f32) -> Self {
        Self {
            line,
            side,
            depth,
            widths,
            gap,
        }
    }

    /// Returns what is wrong with the frontage, or `None`.
    ///
    /// # Invariants
    ///
    /// **One reading, three callers**: the probe asks it, [`Frontage::plots`] refuses on it, and a scene
    /// that places buildings on the plots reads it before it does. Every arm names one number and one
    /// fix, which is the shape a fault has to have to be worth reporting.
    pub fn fault(&self) -> Option<FrontageFault> {
        if self.line.len() < 2 {
            return Some(FrontageFault::LineTooShort {
                points: self.line.len(),
            });
        }
        if !(self.side == 1.0 || self.side == -1.0) {
            return Some(FrontageFault::SideNotASide { side: self.side });
        }
        if !self.depth.is_finite() || self.depth <= 0.0 {
            return Some(FrontageFault::DepthNotALength { depth: self.depth });
        }
        if !self.gap.is_finite() || self.gap < 0.0 {
            return Some(FrontageFault::GapIsNegative { gap: self.gap });
        }
        if self.widths.is_empty() {
            return Some(FrontageFault::NoPlots);
        }
        for (plot, width) in self.widths.iter().enumerate() {
            if !width.is_finite() || *width <= 0.0 {
                return Some(FrontageFault::WidthNotALength {
                    plot,
                    width: *width,
                });
            }
        }
        let wanted = self.wanted_length();
        let available = self.line_length();
        if wanted > available + 1.0e-4 {
            return Some(FrontageFault::LineTooShortForThePlots { wanted, available });
        }
        None
    }

    /// Returns how much line the plots ask for: their widths and the gaps between them.
    pub fn wanted_length(&self) -> f32 {
        let widths: f32 = self.widths.iter().sum();
        widths + self.gap * (self.widths.len().saturating_sub(1)) as f32
    }

    /// Returns how long the line is, corner to corner.
    pub fn line_length(&self) -> f32 {
        let mut total = 0.0;
        for index in 0..self.line.len().saturating_sub(1) {
            total += length(subtract(self.line[index + 1], self.line[index]));
        }
        total
    }

    /// Returns the point a distance `along` the line, and the line's direction there.
    ///
    /// # Invariants
    ///
    /// The walk is **one pass over the corners and it stops where the distance runs out**, so a distance
    /// past the end answers the last corner with the last segment's direction rather than a point that is
    /// not on the street. The direction is the segment's, normalised, which is what makes a bent street's
    /// plots square to *their own* stretch of it.
    pub fn point_at(&self, along: f32) -> Option<(Vec2, Vec2)> {
        if self.line.len() < 2 {
            return None;
        }
        let mut left = along.max(0.0);
        for index in 0..self.line.len() - 1 {
            let (from, to) = (self.line[index], self.line[index + 1]);
            let step = subtract(to, from);
            let span = length(step);
            if span < 1.0e-6 {
                continue;
            }
            let direction = normalized(step);
            if left <= span {
                return Some((
                    [from[0] + direction[0] * left, from[1] + direction[1] * left],
                    direction,
                ));
            }
            left -= span;
        }
        let last = self.line[self.line.len() - 1];
        let before = self.line[self.line.len() - 2];
        let step = subtract(last, before);
        if length(step) < 1.0e-6 {
            None
        } else {
            Some((last, normalized(step)))
        }
    }

    /// Returns the plots along the line, in order, or what is wrong with the frontage.
    ///
    /// # Invariants
    ///
    /// **The plots tile the line**: each one starts where the last one ended plus the gap, and the walk
    /// by widths is the same walk [`Frontage::point_at`] takes. The back of a plot is its own front
    /// segment pushed `depth` to the side the frontage names, and the *sign* of the side is read once and
    /// used for every plot, so a frontage cannot have its plots on two different sides.
    pub fn plots(&self) -> Result<Vec<Plot>, FrontageFault> {
        if let Some(fault) = self.fault() {
            return Err(fault);
        }
        let mut plots = Vec::with_capacity(self.widths.len());
        let mut walked = 0.0;
        for (index, width) in self.widths.iter().enumerate() {
            if index > 0 {
                walked += self.gap;
            }
            // **The direction is read at the plot's own middle**, because a plot that starts exactly on a
            // corner would otherwise be turned by the segment it left rather than the one it stands on.
            let (
                Some((start, start_direction)),
                Some((finish, finish_direction)),
                Some((_, middle)),
            ) = (
                self.point_at(walked),
                self.point_at(walked + width),
                self.point_at(walked + width * 0.5),
            )
            else {
                return Err(FrontageFault::LineTooShortForThePlots {
                    wanted: self.wanted_length(),
                    available: self.line_length(),
                });
            };
            let _ = (start_direction, finish_direction);
            // **Left is the walk's own left**, `[-d[1], d[0]]` in the crate's plan axes, turned by the
            // side. One turn for the whole frontage, so two plots cannot disagree about which way out is.
            let outward = [-middle[1] * self.side, middle[0] * self.side];
            let shift = |at: Vec2| {
                [
                    at[0] + outward[0] * self.depth,
                    at[1] + outward[1] * self.depth,
                ]
            };
            plots.push(Plot {
                index,
                front: [start, finish],
                back: [shift(start), shift(finish)],
                width: *width,
            });
            walked += width;
        }
        Ok(plots)
    }
}

/// Returns the signed plan area of a loop, positive when the crate reads it as a positive loop.
fn plan_area(loop_: &[Vec2]) -> f32 {
    let mut twice = 0.0_f32;
    for index in 0..loop_.len() {
        let from = loop_[index];
        let to = loop_[(index + 1) % loop_.len()];
        twice += from[0] * to[1] - to[0] * from[1];
    }
    twice * 0.5
}

/// Returns the point half way between two points in plan.
fn midpoint(ends: [Vec2; 2]) -> Vec2 {
    [
        (ends[0][0] + ends[1][0]) * 0.5,
        (ends[0][1] + ends[1][1]) * 0.5,
    ]
}

/// Returns `to - from` in plan.
fn subtract(to: Vec2, from: Vec2) -> Vec2 {
    [to[0] - from[0], to[1] - from[1]]
}
