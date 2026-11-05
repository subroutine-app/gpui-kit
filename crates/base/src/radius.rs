use gpui::{Pixels, px};

/// The corner radius that keeps a shape concentric inside one of radius
/// `outer`, when its edge is `inset` from the outer shape's edge.
///
/// `inset` is the padding plus any border drawn inside the outer box. A ring
/// drawn outside the box, such as a popover's edge, does not count. A shape
/// deeper than the corner arc reads fine with any radius, and the result is
/// zero once `inset` reaches `outer`.
pub fn concentric_radius(outer: Pixels, inset: Pixels) -> Pixels {
    (outer - inset).max(Pixels::ZERO)
}

/// The smallest corner radius a nested row or tile gets, capped at half its
/// frame's radius.
const NESTED_RADIUS_FLOOR: Pixels = px(3.);

/// The corner radius for a row or tile nested `inset` inside a frame of radius
/// `outer`, which never reads as square.
///
/// This is [`concentric_radius`] with a minimum. Where the inset is large next
/// to the frame's radius, the exact rule leaves a row with a 1 or 2 px corner
/// that looks sharp, so the result is at least 3 px, capped at half of `outer`:
///
/// ```text
/// max(outer - inset, min(3px, outer / 2))
/// ```
///
/// A 6 or 8 px frame gives its rows at least 3 px, a 4 px frame gives them
/// 2 px, and a frame with no radius stays square. Use it for rows, tiles and
/// buttons inside a frame. Use [`concentric_radius`] where the shape must meet
/// its frame exactly, such as a table header fill, a focus line, a segmented
/// tab or a switch thumb.
pub fn nested_radius(outer: Pixels, inset: Pixels) -> Pixels {
    concentric_radius(outer, inset).max(NESTED_RADIUS_FLOOR.min(outer / 2.))
}

/// The radius of a ring or frame drawn `inset` outside a shape of radius
/// `inner`. The inverse of [`concentric_radius`].
pub fn enclosing_radius(inner: Pixels, inset: Pixels) -> Pixels {
    inner + inset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concentric_radius_follows_the_inset_and_stops_at_zero() {
        assert_eq!(concentric_radius(px(6.), px(4.)), px(2.));
        assert_eq!(concentric_radius(px(8.), px(5.)), px(3.));
        assert_eq!(concentric_radius(px(6.), px(0.)), px(6.));
        assert_eq!(concentric_radius(px(6.), px(6.)), px(0.));
        assert_eq!(concentric_radius(px(6.), px(9.)), px(0.));
        // A frame drawn outside grows the radius instead.
        assert_eq!(concentric_radius(px(6.), px(-3.)), px(9.));
    }

    #[test]
    fn nested_radius_never_drops_below_the_minimum() {
        assert_eq!(nested_radius(px(6.), px(4.)), px(3.));
        assert_eq!(nested_radius(px(8.), px(7.)), px(3.));
        assert_eq!(nested_radius(px(8.), px(5.)), px(3.));
        assert_eq!(nested_radius(px(8.), px(8.)), px(3.));
        assert_eq!(nested_radius(px(8.), px(12.)), px(3.));
    }

    #[test]
    fn nested_radius_keeps_the_exact_radius_above_the_minimum() {
        assert_eq!(nested_radius(px(8.), px(4.)), px(4.));
        assert_eq!(nested_radius(px(8.), px(0.)), px(8.));
        assert_eq!(nested_radius(px(12.), px(4.)), px(8.));
        assert_eq!(nested_radius(px(4.), px(1.)), px(3.));
        assert_eq!(nested_radius(px(6.), px(-3.)), px(9.));
    }

    #[test]
    fn nested_radius_caps_the_minimum_at_half_the_frame() {
        assert_eq!(nested_radius(px(4.), px(4.)), px(2.));
        assert_eq!(nested_radius(px(2.), px(2.)), px(1.));
        assert_eq!(nested_radius(px(0.), px(0.)), px(0.));
        assert_eq!(nested_radius(px(0.), px(4.)), px(0.));
    }

    #[test]
    fn enclosing_radius_undoes_concentric_radius() {
        assert_eq!(enclosing_radius(px(6.), px(3.)), px(9.));
        assert_eq!(
            concentric_radius(enclosing_radius(px(6.), px(3.)), px(3.)),
            px(6.)
        );
    }
}
